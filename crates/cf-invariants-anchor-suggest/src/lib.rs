// cf-invariants-anchor-suggest — ranked invariant suggestion.
//
// The `InvariantClass` trait + `ClassRegistry` are extensible: each
// class is one impl behind the trait, and `ClassRegistry::default()`
// composes the current shipping set. The suggester is pure heuristic
// (no AI call) on this path; the AI-suggested variant lives in
// `cf-invariants-anchor-ai`, which emits the same `InvariantCandidate`
// shape with `InvariantSource::AiSuggested {...}` attached.
//
// Currently registered (post Phase-1-AI build):
//   - `balance_conservation`  — original Phase 0 class.
//   - `monotonic_accounting`  — non-decreasing lifetime/cumulative fields.
//   - `access_control`        — signer-gated mutations of sensitive state.
//
// Adding a class is one new struct + `InvariantClass` impl plus a
// registration line in `default()`. The emit and renderer crates
// dispatch on `InvariantCandidate.class` (a string), so adding here
// without teaching `cf-invariants-anchor-emit` would produce
// candidates the renderer rejects — keep them in lockstep.

use cf_invariants_anchor_core::{
    ContractSurface, EmitHints, IdlType, Instruction, InvariantCandidate, InvariantSource,
    LedgerMove, RelationSpec, TypeDef, TypeDefBody,
};

pub const SUGGESTER_VERSION: &str = "0.3.0";

// Class identifiers, hoisted into constants so emit + the AI prompt +
// the renderer all key off the same string and a typo can't drift.
pub const CLASS_BALANCE_CONSERVATION: &str = "balance_conservation";
pub const CLASS_MONOTONIC_ACCOUNTING: &str = "monotonic_accounting";
pub const CLASS_ACCESS_CONTROL: &str = "access_control";
pub const CLASS_RELATION_INVARIANTS: &str = "relation_invariants";

/// One pluggable invariant class.
pub trait InvariantClass {
    /// Stable identifier — what `InvariantCandidate.class` carries.
    fn class_id(&self) -> &'static str;

    /// Propose ranked candidates for the given surface.
    fn propose(&self, surface: &ContractSurface) -> Vec<InvariantCandidate>;
}

/// Composable list of registered classes.
pub struct ClassRegistry {
    classes: Vec<Box<dyn InvariantClass>>,
}

impl ClassRegistry {
    pub fn empty() -> Self {
        Self { classes: vec![] }
    }

    /// Phase-0 single-class registry (kept for back-compat tests).
    pub fn phase0() -> Self {
        let mut r = Self::empty();
        r.register(Box::new(BalanceConservation));
        r
    }

    /// Current default: all four shipping classes.
    pub fn default() -> Self {
        let mut r = Self::empty();
        r.register(Box::new(BalanceConservation));
        r.register(Box::new(MonotonicAccounting));
        r.register(Box::new(AccessControl));
        r.register(Box::new(RelationInvariants));
        r
    }

    pub fn register(&mut self, class: Box<dyn InvariantClass>) {
        self.classes.push(class);
    }

    /// Walk every registered class and concatenate proposals,
    /// re-sorted by `rank` descending.
    pub fn propose_all(&self, surface: &ContractSurface) -> Vec<InvariantCandidate> {
        let mut out = Vec::new();
        for c in &self.classes {
            out.extend(c.propose(surface));
        }
        out.sort_by(|a, b| {
            b.rank
                .partial_cmp(&a.rank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }
}

// ---------------------------------------------------------------------------
// Class 1: balance_conservation (Phase 0).
// ---------------------------------------------------------------------------

/// For each scalar uint field on an account referenced by ≥1 movement
/// instruction (deposit/withdraw/mint/burn/transfer/claim/redeem/...),
/// emit a candidate that asserts the on-chain value equals a fixture-
/// side ledger walked through the same actions.
pub struct BalanceConservation;

impl InvariantClass for BalanceConservation {
    fn class_id(&self) -> &'static str {
        CLASS_BALANCE_CONSERVATION
    }

    fn propose(&self, surface: &ContractSurface) -> Vec<InvariantCandidate> {
        let movement: Vec<&str> = surface
            .movement_instructions()
            .into_iter()
            .map(|i| i.name.as_str())
            .collect();
        if movement.is_empty() {
            return vec![];
        }
        let mut candidates = Vec::new();
        for f in &surface.balance_fields {
            let strong = matches!(
                f.field.as_str(),
                "amount" | "balance" | "total_amount" | "total_assets" | "supply"
            );
            // Skip monotonic-flavored fields — those are the other
            // class's job; emitting balance_conservation on them
            // would create a false positive on legitimate ratchet-up.
            if looks_monotonic(&f.field) {
                continue;
            }
            let rank = if strong { 0.92 } else { 0.55 };
            let invariant_name = format!(
                "invariant_{}_conservation",
                f.field.to_ascii_lowercase()
            );
            let summary = format!(
                "{}.{} == fixture-tracked sum of deposits − sum of withdrawals",
                f.account, f.field
            );
            let rationale = format!(
                "Detected balance-bearing field `{}.{}: {}` on an account \
                 mutated by movement-class instructions ({}). A correct \
                 implementation keeps this field in lock-step with the \
                 net amount transferred in via these instructions; any \
                 drift is a balance-conservation violation. The fixture \
                 walks `expected_{}` through `action_deposit`/`action_withdraw` \
                 and asserts equality after every action.",
                f.account,
                f.field,
                f.ty,
                movement.join(", "),
                f.field
            );
            let emit_hints = EmitHints {
                account_type: f.account.clone(),
                field: f.field.clone(),
                expected_expression: format!("fixture.expected_{}", f.field),
                action_names: movement.iter().map(|s| s.to_string()).collect(),
                ledger_moves: ledger_moves(surface),
                relation_specs: vec![],
                suppressed_specs: vec![],
            };
            candidates.push(InvariantCandidate {
                name: invariant_name,
                summary,
                class: self.class_id().to_string(),
                rank,
                rationale,
                emit_hints,
                source: InvariantSource::Heuristic {
                    suggester_version: SUGGESTER_VERSION.to_string(),
                },
            });
        }
        candidates
    }
}

// ---------------------------------------------------------------------------
// Class 2: monotonic_accounting.
// ---------------------------------------------------------------------------

/// For each scalar uint field whose NAME signals lifetime / cumulative
/// / counter / sequence semantics, emit a candidate asserting the
/// on-chain value never decreases between successive observations.
///
/// Captures the "no value created from nothing" class of bugs: a
/// withdraw path that accidentally decrements a "lifetime_deposited"
/// counter, an admin action that resets a sequence number, an upgrade
/// that lowers a version field. These often pass conservation checks
/// (because the live balance moves correctly) but break audit
/// downstream.
pub struct MonotonicAccounting;

impl InvariantClass for MonotonicAccounting {
    fn class_id(&self) -> &'static str {
        CLASS_MONOTONIC_ACCOUNTING
    }

    fn propose(&self, surface: &ContractSurface) -> Vec<InvariantCandidate> {
        let movement: Vec<&str> = surface
            .movement_instructions()
            .into_iter()
            .map(|i| i.name.as_str())
            .collect();
        // Without any movement instructions, monotonicity has nothing
        // to drive — every value is trivially monotone over a zero-
        // action trace. Skip rather than emit something that always
        // passes.
        if movement.is_empty() {
            return vec![];
        }
        let mut candidates = Vec::new();
        for f in &surface.balance_fields {
            if !looks_monotonic(&f.field) {
                continue;
            }
            let invariant_name = format!(
                "invariant_{}_monotonic",
                f.field.to_ascii_lowercase()
            );
            let summary = format!(
                "{}.{} never decreases across successive observations",
                f.account, f.field
            );
            let rationale = format!(
                "Detected lifetime/cumulative-flavored field `{}.{}: {}` \
                 on an account mutated by program instructions ({}). \
                 Fields with this naming convention are ratchet-only — \
                 a correct implementation never decreases them. The \
                 fixture snapshots `last_seen_{}` after `setup` and \
                 each action; any subsequent observation that violates \
                 monotonic ≥ is the bug.",
                f.account,
                f.field,
                f.ty,
                movement.join(", "),
                f.field
            );
            let emit_hints = EmitHints {
                account_type: f.account.clone(),
                field: f.field.clone(),
                // Sentinel: emit pulls `last_seen_<field>` directly.
                expected_expression: format!("fixture.last_seen_{}", f.field),
                action_names: movement.iter().map(|s| s.to_string()).collect(),
                ledger_moves: vec![],
                relation_specs: vec![],
                suppressed_specs: vec![],
            };
            candidates.push(InvariantCandidate {
                name: invariant_name,
                summary,
                class: self.class_id().to_string(),
                rank: 0.83,
                rationale,
                emit_hints,
                source: InvariantSource::Heuristic {
                    suggester_version: SUGGESTER_VERSION.to_string(),
                },
            });
        }
        candidates
    }
}

/// Fixture-side ledger hypothesis: which instruction argument moves
/// the tracked field, and in which direction. Direction comes from the
/// movement-marker name (deliberately Phase-0 lexical, same status as
/// `movement_instructions()`); the argument is the first scalar-uint
/// arg the IDL declares on that instruction. No IDL args → no move
/// (the arm still fires, the ledger just does not walk).
fn ledger_moves(surface: &ContractSurface) -> Vec<LedgerMove> {
    let mut moves = Vec::new();
    for ix in surface.movement_instructions() {
        let Some(add) = movement_direction(&ix.name) else {
            continue;
        };
        let Some(arg) = first_scalar_uint_arg(ix) else {
            continue;
        };
        moves.push(LedgerMove {
            action: ix.name.clone(),
            arg,
            add,
        });
    }
    moves
}

/// `Some(true)` = inflow, `Some(false)` = outflow, `None` = ambiguous
/// (e.g. `transfer`) — no ledger move. Outflow markers are checked
/// first so `unstake` does not match `stake`.
fn movement_direction(name: &str) -> Option<bool> {
    let n = name.to_ascii_lowercase();
    const OUT: &[&str] = &["withdraw", "burn", "unstake", "redeem", "claim"];
    const IN: &[&str] = &["deposit", "mint", "stake"];
    if OUT.iter().any(|m| n.contains(m)) {
        return Some(false);
    }
    if IN.iter().any(|m| n.contains(m)) {
        return Some(true);
    }
    None
}

fn first_scalar_uint_arg(ix: &Instruction) -> Option<String> {
    ix.arg_defs
        .iter()
        .find(|a| {
            matches!(
                a.ty,
                IdlType::U8 | IdlType::U16 | IdlType::U32 | IdlType::U64 | IdlType::U128
            )
        })
        .map(|a| a.name.clone())
}

/// A field name that signals ratchet-only / cumulative semantics.
fn looks_monotonic(field: &str) -> bool {
    let n = field.to_ascii_lowercase();
    const PREFIXES: &[&str] = &[
        "lifetime_",
        "cumulative_",
        "total_lifetime_",
        "ever_",
    ];
    const SUFFIXES: &[&str] = &[
        "_counter",
        "_count",
        "_seq",
        "_sequence",
        "_version",
        "_nonce",
    ];
    PREFIXES.iter().any(|p| n.starts_with(p))
        || SUFFIXES.iter().any(|s| n.ends_with(s))
        || n == "sequence_number"
        || n == "version"
}

// ---------------------------------------------------------------------------
// Class 3: access_control.
// ---------------------------------------------------------------------------

/// For each instruction whose NAME signals authority-gated mutation
/// (withdraw / admin_* / set_* / transfer_authority / mint / burn),
/// emit a candidate asserting the program rejects the call when invoked
/// by an unauthorized signer.
///
/// Captures the "no unauthorized side-effect" class of bugs: a
/// withdraw path missing `has_one`, a `set_authority` that forgot to
/// require the current authority's signature, a mint instruction with
/// a Signer<'_> account but no PDA-owner check. The emitted fuzz
/// fixture probes with a randomly-generated `attacker` Keypair and
/// fails if the call SUCCEEDS — the inverse of a positive test.
pub struct AccessControl;

impl InvariantClass for AccessControl {
    fn class_id(&self) -> &'static str {
        CLASS_ACCESS_CONTROL
    }

    fn propose(&self, surface: &ContractSurface) -> Vec<InvariantCandidate> {
        let mut candidates = Vec::new();
        for ix in &surface.instructions {
            if !looks_authority_gated(ix) {
                continue;
            }
            let invariant_name = format!(
                "invariant_{}_rejects_unauthorized",
                ix.name.to_ascii_lowercase()
            );
            let summary = format!(
                "{} rejects when invoked by anyone other than the authorized signer",
                ix.name
            );
            let rationale = format!(
                "Detected authority-gated instruction `{}` (name signals \
                 a privileged mutation). The emitted fuzz fixture probes \
                 with a freshly-generated attacker `Keypair` (never the \
                 vault depositor / authority) and asserts the call \
                 returns an error. Any success is an access-control \
                 violation — the program failed to verify the signer.",
                ix.name
            );
            // Pick the most plausible authority account name from the
            // surface. Phase-1 heuristic: prefer `depositor`, then
            // `authority`, then `owner`, then `admin`.
            let authority_field = pick_authority_field(surface);
            let emit_hints = EmitHints {
                // The sticky-flag fixture reads no state account, so
                // account_type is informational: the first tracked
                // account type from THIS program's IDL (never a name
                // copied from a reference example).
                account_type: surface
                    .balance_fields
                    .first()
                    .map(|f| f.account.clone())
                    .unwrap_or_default(),
                field: authority_field.clone(),
                expected_expression: "fixture.unauthorized_success_observed".into(),
                action_names: vec![ix.name.clone()],
                // Positive-direction moves seed observable state before
                // the attack probes.
                ledger_moves: ledger_moves(surface),
                relation_specs: vec![],
                suppressed_specs: vec![],
            };
            candidates.push(InvariantCandidate {
                name: invariant_name,
                summary,
                class: self.class_id().to_string(),
                rank: 0.78,
                rationale,
                emit_hints,
                source: InvariantSource::Heuristic {
                    suggester_version: SUGGESTER_VERSION.to_string(),
                },
            });
        }
        candidates
    }
}

fn looks_authority_gated(ix: &Instruction) -> bool {
    let n = ix.name.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "withdraw",
        "admin",
        "set_",
        "transfer_authority",
        "mint",
        "burn",
        "close",
        "freeze",
        "thaw",
        "upgrade",
    ];
    MARKERS.iter().any(|m| n.contains(m))
}

fn pick_authority_field(surface: &ContractSurface) -> String {
    // We look at instruction account-name lists for a likely
    // authority. Anchor IDL conventions tend to name these
    // `depositor`, `authority`, `owner`, or `admin`.
    const PREFERENCE: &[&str] = &["depositor", "authority", "owner", "admin"];
    for ix in &surface.instructions {
        for acct in &ix.accounts {
            let a = acct.to_ascii_lowercase();
            if PREFERENCE.iter().any(|p| a == *p) {
                return acct.clone();
            }
        }
    }
    // Default: the convention vault_ref uses.
    "depositor".to_string()
}

// ---------------------------------------------------------------------------
// Class 4: relation_invariants (R2 — the multi-field / token-account lift).
// ---------------------------------------------------------------------------

/// Emits one bundle candidate per movement / mutation instruction that
/// surfaces at least one derivable relation. Each spec inside the bundle
/// is one non-trivial invariant the emitter turns into a distinct
/// `#[invariant_test]` fn. The classes here are exactly the R2a spec's
/// derivable set (RC-A, RC-D, RC-E, RC-G, RC-I), keyed off IDL structure:
///
///   - **RC-A (Binding)** — for each program-owned state struct S read in
///     an instruction I, and for each pubkey field on S whose *name*
///     matches an instruction-account name in I, emit
///     `S.<field> == <account>.key`.
///   - **RC-D (SupplyMint)** — for each `*supply` u64 field on S whose
///     naming pairs with a `*_mint` / `share_mint` / `lp_mint`
///     instruction account, emit
///     `S.<supply_field> == Mint::unpack(<mint>).supply`.
///   - **RC-E (FeeMonotone)** — for each `*fee*`, `*accumulator`,
///     `cumulative_*`, `total_*_collected` u64 on S, emit
///     `prev(<field>) <= current(<field>)`.
///   - **RC-G (VaultBinding)** — for each `*vault*` instruction account
///     paired with a matching `*mint` and an `authority` in the same
///     instruction, emit `TokenAccount(<vault>).mint == <mint>` and
///     `TokenAccount(<vault>).owner == <authority>`.
///   - **RC-I (MintAuthority)** — for each `*mint*` instruction account
///     paired with an `authority`, emit
///     `Mint::unpack(<mint>).mint_authority == Some(<authority>)`.
///
/// De-duplication: relation specs are keyed by their tuple contents;
/// each unique tuple appears at most once in the emitted bundle.
pub struct RelationInvariants;

impl InvariantClass for RelationInvariants {
    fn class_id(&self) -> &'static str {
        CLASS_RELATION_INVARIANTS
    }

    fn propose(&self, surface: &ContractSurface) -> Vec<InvariantCandidate> {
        let mut specs: Vec<RelationSpec> = Vec::new();

        for ix in &surface.instructions {
            // Skip loud-exclusion instructions the emitter won't render
            // anyway (admin-pinned signers, unrenderable args). We can't
            // easily replicate `instruction_supported` here without
            // pulling emit as a dep; the emitter filters again downstream
            // so any speck we emit for an excluded ix is a no-op.
            let acct_names: Vec<&str> =
                ix.account_defs.iter().map(|a| a.name.as_str()).collect();
            if acct_names.is_empty() {
                continue;
            }
            for state_acct in &ix.account_defs {
                let state_ty = match match_state_type(surface, &state_acct.name) {
                    Some(t) => t,
                    None => continue,
                };
                let tdef = match struct_type_def(surface, &state_ty) {
                    Some(t) => t,
                    None => continue,
                };
                let fields = match &tdef.body {
                    TypeDefBody::Struct { fields } => fields,
                    _ => continue,
                };

                // RC-A: pubkey-field <-> sibling account-name binding.
                for f in fields {
                    if !matches!(f.ty, IdlType::Pubkey) {
                        continue;
                    }
                    if acct_names.contains(&f.name.as_str())
                        && f.name != state_acct.name
                    {
                        let spec = RelationSpec::Binding {
                            state_account_type: state_ty.clone(),
                            state_account_name: state_acct.name.clone(),
                            pubkey_field: f.name.clone(),
                            bound_account_name: f.name.clone(),
                        };
                        push_unique(&mut specs, spec);
                    }
                }

                // RC-D: supply-vs-mint (paired with a `*mint*` sibling).
                for f in fields {
                    if !is_u64(&f.ty) {
                        continue;
                    }
                    if !looks_supply(&f.name) {
                        continue;
                    }
                    if let Some(mint) = pick_mint_sibling(&acct_names, &f.name) {
                        let spec = RelationSpec::SupplyMint {
                            state_account_type: state_ty.clone(),
                            state_account_name: state_acct.name.clone(),
                            supply_field: f.name.clone(),
                            mint_account_name: mint.to_string(),
                        };
                        push_unique(&mut specs, spec);
                    }
                }

                // RC-E: fee-accumulator monotonicity. Excluded from
                // instructions that legitimately DECREASE the field
                // (collect_*, withdraw_fee_*, etc).
                if legitimately_resets_fees(&ix.name) {
                    // Do not emit a FeeMonotone spec from an instruction
                    // whose whole purpose is to decrement — but do emit
                    // from other instructions with the same state read.
                    continue;
                }
                for f in fields {
                    if !is_u64(&f.ty) {
                        continue;
                    }
                    if !looks_fee_accumulator(&f.name) {
                        continue;
                    }
                    let spec = RelationSpec::FeeMonotone {
                        state_account_type: state_ty.clone(),
                        state_account_name: state_acct.name.clone(),
                        field: f.name.clone(),
                    };
                    push_unique(&mut specs, spec);
                }
            }

            // RC-G: vault <-> mint + owner binding. Requires an authority
            // sibling in the same instruction (the pool authority PDA).
            let auth_acct = acct_names
                .iter()
                .find(|a| **a == "authority" || a.ends_with("_authority"))
                .copied();
            if let Some(authority) = auth_acct {
                for vault_name in acct_names.iter().filter(|a| looks_vault(a)) {
                    // Vault-binding: require an EXACT prefix-matched
                    // mint. The fallback (lp_mint) would be wrong: a
                    // vault holds one of the paired tokens, not the LP.
                    if let Some(mint) = pick_prefix_mint_sibling(&acct_names, vault_name) {
                        let spec = RelationSpec::VaultBinding {
                            vault_account_name: vault_name.to_string(),
                            expected_mint_name: mint.to_string(),
                            expected_owner_name: authority.to_string(),
                        };
                        push_unique(&mut specs, spec);
                    }
                }

                // RC-I: mint-authority binding. Restricted to plausibly
                // program-owned mints (LP / share / pool). Firing on
                // user-token mints would false-fail because those carry
                // unrelated authorities.
                for mint_name in acct_names
                    .iter()
                    .filter(|a| looks_program_owned_mint(a) && !looks_vault(a))
                {
                    let spec = RelationSpec::MintAuthority {
                        mint_account_name: mint_name.to_string(),
                        expected_authority_name: authority.to_string(),
                    };
                    push_unique(&mut specs, spec);
                }
            }
        }

        if specs.is_empty() {
            return vec![];
        }

        // Rank: relation bundles score above single-scalar conservation
        // because their assertions bind ≥2 fields or a token account.
        let summary = format!(
            "{} relation invariants (RC-A/D/E/G/I) across {} shape(s)",
            specs.len(),
            distinct_kinds(&specs)
        );
        let rationale = format!(
            "Detected {} derivable relations from the IDL + account layout: \
             cross-account pubkey bindings (RC-A), cross-program supply-vs-mint \
             conservation (RC-D), fee-accumulator monotonicity (RC-E), token \
             account ownership/mint binding (RC-G), and SPL mint-authority \
             binding (RC-I). Each spec turns into an independent \
             `#[invariant_test]` fn in the emitted fixture; the assertions \
             read live SPL state (Mint::unpack / Account::unpack) so a bug \
             that decouples the program's ledger from the actual SPL state \
             trips the invariant.",
            specs.len(),
        );
        let hints = EmitHints {
            account_type: String::new(),
            field: String::new(),
            expected_expression: String::new(),
            action_names: surface
                .instructions
                .iter()
                .map(|i| i.name.clone())
                .collect(),
            ledger_moves: vec![],
            relation_specs: specs,
            suppressed_specs: vec![],
        };
        vec![InvariantCandidate {
            name: "invariant_relation_bundle".to_string(),
            summary,
            class: self.class_id().to_string(),
            rank: 0.95,
            rationale,
            emit_hints: hints,
            source: InvariantSource::Heuristic {
                suggester_version: SUGGESTER_VERSION.to_string(),
            },
        }]
    }
}

fn push_unique(v: &mut Vec<RelationSpec>, spec: RelationSpec) {
    if !v.contains(&spec) {
        v.push(spec);
    }
}

fn distinct_kinds(specs: &[RelationSpec]) -> usize {
    let mut s = std::collections::BTreeSet::new();
    for x in specs {
        s.insert(match x {
            RelationSpec::Binding { .. } => "binding",
            RelationSpec::SupplyMint { .. } => "supply_mint",
            RelationSpec::FeeMonotone { .. } => "fee_monotone",
            RelationSpec::VaultBinding { .. } => "vault_binding",
            RelationSpec::MintAuthority { .. } => "mint_authority",
        });
    }
    s.len()
}

/// The IDL does not link instruction accounts to state-account types.
/// Bind by snake_case name equality — the same rule the emitter's
/// `tracked_account_binding` uses. Returns `Some(type_name)` iff the
/// name matches an entry in `surface.account_types`.
fn match_state_type(surface: &ContractSurface, ix_acct_name: &str) -> Option<String> {
    let want = ix_acct_name.to_ascii_lowercase();
    for at in &surface.account_types {
        if to_snake(&at.name) == want {
            return Some(at.name.clone());
        }
    }
    None
}

fn struct_type_def<'a>(surface: &'a ContractSurface, name: &str) -> Option<&'a TypeDef> {
    surface.type_defs.iter().find(|t| t.name == name)
}

fn is_u64(ty: &IdlType) -> bool {
    matches!(ty, IdlType::U64)
}

fn looks_supply(n: &str) -> bool {
    let l = n.to_ascii_lowercase();
    l == "lp_supply"
        || l == "total_supply"
        || l == "total_shares"
        || l == "supply"
        || l.ends_with("_supply")
        || l == "total_liquidity"
}

fn looks_fee_accumulator(n: &str) -> bool {
    let l = n.to_ascii_lowercase();
    // Do not match `*_fee_rate` — those are configs, not accumulators.
    if l.ends_with("_rate") || l.ends_with("_bps") || l.ends_with("_fee_rate") {
        return false;
    }
    l.contains("_fee")
        || l.ends_with("_accumulator")
        || l.starts_with("cumulative_")
        || l.contains("_collected")
        || l.contains("_paid")
}

fn legitimately_resets_fees(ix_name: &str) -> bool {
    let l = ix_name.to_ascii_lowercase();
    l.starts_with("collect_")
        || l.starts_with("withdraw_fee")
        || l.starts_with("reset_")
        || l.starts_with("skim_")
        || l.contains("_collect_fee")
}

fn looks_vault(name: &str) -> bool {
    // Strict: end-with markers only, so `vault_0_mint` is NOT classified
    // as a vault (it is a mint). `*_vault`, `input_vault`, `output_vault`,
    // `*_reserve` all qualify; anything else does not.
    let l = name.to_ascii_lowercase();
    l.ends_with("_vault") || l == "vault" || l.ends_with("_reserve")
}

/// True for names that are plausibly a program-owned SPL Mint (LP,
/// share, pool) — the ONLY mints whose `mint_authority` is expected to
/// bind to a program authority PDA. User-facing mints (USDC, etc.)
/// carry unrelated authorities and would false-fail if RC-I fired.
fn looks_program_owned_mint(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l == "lp_mint"
        || l == "share_mint"
        || l == "pool_mint"
        || l.starts_with("lp_")
        || l.ends_with("_lp_mint")
        || l.ends_with("_share_mint")
        || l.ends_with("_pool_mint")
}

/// For a supply/vault field named e.g. `lp_supply` or `token_0_vault`,
/// pick the sibling instruction account that plausibly is its Mint.
///
/// Preference order — most specific first:
///   1. Exact prefix rewrite: `lp_supply` -> `lp_mint`; `token_0_vault`
///      -> `token_0_mint`. Handles the flagship CP-Swap shape.
///   2. Fallback: a program-owned mint sibling (LP / share / pool).
fn pick_mint_sibling<'a>(acct_names: &[&'a str], target_field: &str) -> Option<&'a str> {
    if let Some(a) = pick_prefix_mint_sibling(acct_names, target_field) {
        return Some(a);
    }
    // Weaker fallback: a sibling name-ending in `_mint` (not user-tokens
    // — those are `input_token_mint`, `output_token_mint`, which pair
    // supply with the wrong side of the swap). Restricted to program-
    // owned mint names so RC-D does not false-fire.
    acct_names
        .iter()
        .find(|a| looks_program_owned_mint(a))
        .copied()
}

/// Prefix-only pick: `lp_supply` -> `lp_mint`; `token_0_vault` ->
/// `token_0_mint`. No fallback — returns None if no prefix match.
fn pick_prefix_mint_sibling<'a>(acct_names: &[&'a str], target_field: &str) -> Option<&'a str> {
    let low = target_field.to_ascii_lowercase();
    for (suffix, mint) in [
        ("_supply", "_mint"),
        ("_vault", "_mint"),
        ("_reserve", "_mint"),
    ] {
        if low.ends_with(suffix) {
            let base = &low[..low.len() - suffix.len()];
            let want = format!("{base}{mint}");
            if let Some(a) = acct_names.iter().find(|a| a.to_ascii_lowercase() == want) {
                return Some(a);
            }
        }
    }
    None
}

fn to_snake(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::{BalanceField, ContractSurface, Instruction, IxArgDef};

    fn vault_surface() -> ContractSurface {
        ContractSurface {
            program_id: "Va111tRef1111111111111111111111111111111111".into(),
            program_name: "vault_ref".into(),
            instructions: vec![
                Instruction {
                    name: "initialize".into(),
                    args: vec![],
                    accounts: vec!["vault".into(), "depositor".into(), "system_program".into()],
                    ..Default::default()
                },
                Instruction {
                    name: "deposit".into(),
                    args: vec!["amount".into()],
                    accounts: vec!["vault".into(), "depositor".into(), "system_program".into()],
                    arg_defs: vec![IxArgDef {
                        name: "amount".into(),
                        ty: IdlType::U64,
                    }],
                    ..Default::default()
                },
                Instruction {
                    name: "withdraw".into(),
                    args: vec!["amount".into()],
                    accounts: vec!["vault".into(), "depositor".into()],
                    arg_defs: vec![IxArgDef {
                        name: "amount".into(),
                        ty: IdlType::U64,
                    }],
                    ..Default::default()
                },
            ],
            balance_fields: vec![BalanceField {
                account: "Vault".into(),
                field: "amount".into(),
                ty: "u64".into(),
            }],
            ..Default::default()
        }
    }

    fn vault_with_lifetime_counter() -> ContractSurface {
        let mut s = vault_surface();
        s.balance_fields.push(BalanceField {
            account: "Vault".into(),
            field: "lifetime_deposited".into(),
            ty: "u64".into(),
        });
        s
    }

    // -- balance_conservation --------------------------------------------

    #[test]
    fn balance_conservation_phase0_compat() {
        let s = vault_surface();
        let r = ClassRegistry::phase0();
        let cs = r.propose_all(&s);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].class, CLASS_BALANCE_CONSERVATION);
        assert_eq!(cs[0].name, "invariant_amount_conservation");
    }

    #[test]
    fn balance_conservation_skips_monotonic_flavored_fields() {
        // `lifetime_deposited` should NOT be picked up by the
        // conservation class — the monotonic class owns it.
        let s = vault_with_lifetime_counter();
        let bc = BalanceConservation;
        let cs = bc.propose(&s);
        assert!(cs.iter().all(|c| c.name != "invariant_lifetime_deposited_conservation"));
    }

    #[test]
    fn balance_conservation_carries_idl_derived_ledger_moves() {
        let s = vault_surface();
        let cs = BalanceConservation.propose(&s);
        assert!(!cs.is_empty());
        let moves = &cs[0].emit_hints.ledger_moves;
        assert!(moves
            .iter()
            .any(|m| m.action == "deposit" && m.arg == "amount" && m.add));
        assert!(moves
            .iter()
            .any(|m| m.action == "withdraw" && m.arg == "amount" && !m.add));
    }

    #[test]
    fn access_control_hints_never_carry_reference_example_names() {
        // C1 friction F9: access candidates carried `account_type:
        // "Vault"` hardcoded even on programs with no such account.
        let mut s = vault_surface();
        s.balance_fields = vec![BalanceField {
            account: "PoolState".into(),
            field: "lp_supply".into(),
            ty: "u64".into(),
        }];
        let cs = AccessControl.propose(&s);
        assert!(!cs.is_empty());
        for c in &cs {
            assert_ne!(c.emit_hints.account_type, "Vault");
            assert!(!c.emit_hints.expected_expression.contains("fixture.vault"));
        }
    }

    // -- monotonic_accounting --------------------------------------------

    #[test]
    fn monotonic_picks_lifetime_field() {
        let s = vault_with_lifetime_counter();
        let m = MonotonicAccounting;
        let cs = m.propose(&s);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].class, CLASS_MONOTONIC_ACCOUNTING);
        assert_eq!(cs[0].name, "invariant_lifetime_deposited_monotonic");
        assert!(cs[0].emit_hints.expected_expression.contains("last_seen_"));
    }

    #[test]
    fn monotonic_ignores_plain_amount() {
        let s = vault_surface();
        let m = MonotonicAccounting;
        assert!(m.propose(&s).is_empty());
    }

    #[test]
    fn monotonic_handles_suffix_patterns() {
        let mut s = vault_surface();
        s.balance_fields = vec![BalanceField {
            account: "Counter".into(),
            field: "tx_counter".into(),
            ty: "u64".into(),
        }];
        let m = MonotonicAccounting;
        let cs = m.propose(&s);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].name, "invariant_tx_counter_monotonic");
    }

    #[test]
    fn monotonic_skips_when_no_movement_instructions() {
        let mut s = vault_with_lifetime_counter();
        s.instructions = vec![Instruction {
            name: "initialize".into(),
            args: vec![],
            accounts: vec![],
            ..Default::default()
        }];
        let m = MonotonicAccounting;
        assert!(m.propose(&s).is_empty());
    }

    // -- access_control --------------------------------------------------

    #[test]
    fn access_control_picks_withdraw() {
        let s = vault_surface();
        let a = AccessControl;
        let cs = a.propose(&s);
        assert!(
            cs.iter().any(|c| c.name == "invariant_withdraw_rejects_unauthorized"),
            "candidates: {:?}",
            cs.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        let w = cs
            .iter()
            .find(|c| c.name == "invariant_withdraw_rejects_unauthorized")
            .unwrap();
        assert_eq!(w.class, CLASS_ACCESS_CONTROL);
        assert_eq!(w.emit_hints.field, "depositor");
    }

    #[test]
    fn access_control_picks_admin_instructions() {
        let mut s = vault_surface();
        s.instructions.push(Instruction {
            name: "admin_close".into(),
            args: vec![],
            accounts: vec!["vault".into(), "authority".into()],
            ..Default::default()
        });
        let a = AccessControl;
        let cs = a.propose(&s);
        assert!(cs.iter().any(|c| c.name == "invariant_admin_close_rejects_unauthorized"));
    }

    #[test]
    fn access_control_skips_non_privileged_instructions() {
        let mut s = vault_surface();
        // Strip privileged instructions: only `initialize` left
        // (matches no marker).
        s.instructions.retain(|i| i.name == "initialize");
        let a = AccessControl;
        assert!(a.propose(&s).is_empty());
    }

    // -- registry composition --------------------------------------------

    #[test]
    fn default_registry_proposes_all_three_classes_on_full_surface() {
        let s = vault_with_lifetime_counter();
        let r = ClassRegistry::default();
        let cs = r.propose_all(&s);
        let classes: std::collections::BTreeSet<_> =
            cs.iter().map(|c| c.class.as_str()).collect();
        assert!(classes.contains(CLASS_BALANCE_CONSERVATION));
        assert!(classes.contains(CLASS_MONOTONIC_ACCOUNTING));
        assert!(classes.contains(CLASS_ACCESS_CONTROL));
    }

    #[test]
    fn default_registry_returns_sorted_by_rank() {
        let s = vault_with_lifetime_counter();
        let cs = ClassRegistry::default().propose_all(&s);
        for w in cs.windows(2) {
            assert!(w[0].rank >= w[1].rank, "{:?}", cs);
        }
    }

    #[test]
    fn default_registry_marks_heuristic_source() {
        let s = vault_with_lifetime_counter();
        let cs = ClassRegistry::default().propose_all(&s);
        for c in &cs {
            match &c.source {
                InvariantSource::Heuristic { suggester_version } => {
                    assert_eq!(suggester_version, SUGGESTER_VERSION);
                }
                _ => panic!("expected Heuristic source, got {:?}", c.source),
            }
        }
    }
}
