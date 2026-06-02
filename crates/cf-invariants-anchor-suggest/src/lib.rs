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
    ContractSurface, EmitHints, Instruction, InvariantCandidate, InvariantSource,
};

pub const SUGGESTER_VERSION: &str = "0.2.0";

// Class identifiers, hoisted into constants so emit + the AI prompt +
// the renderer all key off the same string and a typo can't drift.
pub const CLASS_BALANCE_CONSERVATION: &str = "balance_conservation";
pub const CLASS_MONOTONIC_ACCOUNTING: &str = "monotonic_accounting";
pub const CLASS_ACCESS_CONTROL: &str = "access_control";

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

    /// Current default: all three shipping classes.
    pub fn default() -> Self {
        let mut r = Self::empty();
        r.register(Box::new(BalanceConservation));
        r.register(Box::new(MonotonicAccounting));
        r.register(Box::new(AccessControl));
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
            // Pick the most plausible authority field name from the
            // surface. Phase-1 heuristic: prefer `depositor`, then
            // `authority`, then `owner`, then `admin`.
            let authority_field = pick_authority_field(surface);
            let emit_hints = EmitHints {
                // Account_type is picked by emit using its own
                // PDA-discovery heuristic; we name the convention
                // field here as a hint.
                account_type: "Vault".into(),
                field: authority_field.clone(),
                expected_expression: format!("fixture.vault.{}", authority_field),
                action_names: vec![ix.name.clone()],
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

#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::{BalanceField, ContractSurface, Instruction};

    fn vault_surface() -> ContractSurface {
        ContractSurface {
            program_id: "Va111tRef1111111111111111111111111111111111".into(),
            program_name: "vault_ref".into(),
            instructions: vec![
                Instruction {
                    name: "initialize".into(),
                    args: vec![],
                    accounts: vec!["vault".into(), "depositor".into(), "system_program".into()],
                },
                Instruction {
                    name: "deposit".into(),
                    args: vec!["amount".into()],
                    accounts: vec!["vault".into(), "depositor".into(), "system_program".into()],
                },
                Instruction {
                    name: "withdraw".into(),
                    args: vec!["amount".into()],
                    accounts: vec!["vault".into(), "depositor".into()],
                },
            ],
            balance_fields: vec![BalanceField {
                account: "Vault".into(),
                field: "amount".into(),
                ty: "u64".into(),
            }],
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
