// cf-invariants-anchor-emit — render Crucible-compatible fixtures.
//
// IMPORTANT — Crucible API drift note:
//   The original ticket (T-cf-invariants-anchor-phase0-2026-06-01)
//   referenced `crucible::invariants!{…}` as the emit target. That is
//   not the v0.2.0 surface. The real Crucible v0.2.0 API (verified
//   against asymmetric-research/crucible@main, examples/escrow) is
//   the attribute-macro pair `#[fuzz_fixture]` + `#[invariant_test]`
//   from the `crucible_fuzzer` crate, paired with assertion macros
//   `fuzz_assert_eq!`, `fuzz_assert_lt!`, `fuzz_assert_le!`.
//
// This crate emits the real shape. Trident is a noted secondary emit
// target — the renderer is class-driven so adding a `TridentTarget`
// in Phase 1 is a one-file change.
//
// Class dispatch:
//   - balance_conservation  -> render_crucible_balance
//   - monotonic_accounting  -> render_crucible_monotonic
//   - access_control        -> render_crucible_access_control
// A class string the renderer doesn't recognize falls through to a
// placeholder so a typo in the suggester is loud, not silent.
//
// Shared scaffolding (the header comment, the use-block + INITIAL_BALANCE
// constant, and the setup-body intro) lives in the small `render_*` helpers
// below. The 3 class-specific renderers compose those helpers and inline
// only the bits that genuinely differ (fixture struct fields, action arms,
// and the #[invariant_test] body). Output is byte-identical to the
// pre-2026-06-04 emitter — the kamino-class-lending examples regenerate
// unchanged.

use cf_invariants_anchor_core::{ContractSurface, InvariantCandidate, InvariantSource};

/// Which downstream harness we render for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Asymmetric Research's Crucible (LibAFL + LiteSVM). Default.
    Crucible,
    /// Ackee's Trident (TridentSVM, manual invariants). Phase 1.
    /// Stubbed — emit returns an explanatory placeholder so the
    /// `cli emit --target trident …` path is reachable.
    Trident,
}

/// Render the candidate as a Crucible fuzz-test source string.
pub fn render(
    surface: &ContractSurface,
    candidate: &InvariantCandidate,
    target: Target,
) -> String {
    match target {
        Target::Crucible => render_crucible(surface, candidate),
        Target::Trident => render_trident_stub(surface, candidate),
    }
}

fn render_crucible(surface: &ContractSurface, candidate: &InvariantCandidate) -> String {
    match candidate.class.as_str() {
        "balance_conservation" => render_crucible_balance(surface, candidate),
        "monotonic_accounting" => render_crucible_monotonic(surface, candidate),
        "access_control" => render_crucible_access_control(surface, candidate),
        other => render_unknown_class(other, candidate),
    }
}

// ---------------------------------------------------------------------------
// Shared scaffolding helpers — used by all 3 class renderers.
// ---------------------------------------------------------------------------

/// Leading `// …` comment block shared by every emitted fixture. Ends at
/// the last `\n` of `fixture_doc`; caller adds the blank-line separator.
fn render_header_comment_block(
    invariant_fn: &str,
    class: &str,
    disclosure: &str,
    summary: &str,
    fixture_doc: &str,
) -> String {
    format!(
"// {invariant_fn}
//
// Emitted by cf-invariants-anchor v0.2.0 for the {class} class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// {disclosure}
//
// {summary}
//
{fixture_doc}"
    )
}

/// `#![allow(…)]` + `use …` + `INITIAL_BALANCE` block shared by every
/// emitted fixture. `disambig_comment` (if non-empty, must end in `\n`)
/// is injected between `use crucible_fuzzer::*;` and the program-crate
/// `use ::…::*;` — only balance currently passes one, preserving the
/// pre-2026-06-04 byte-identical output for the existing examples.
fn render_imports_and_const(program_name: &str, disambig_comment: &str) -> String {
    format!(
"#![allow(unused_imports)]

use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
{disambig_comment}use ::{program_name}::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

const INITIAL_BALANCE: u64 = 10_000_000_000;"
    )
}

/// Shared `setup()` body: ctx construction, depositor `Keypair`, PDA
/// derivation, `Initialize` call. Used by balance + monotonic; access
/// control inlines its own to keep the seed-deposit comment anchored
/// before `Initialize`. 8-space indented, ends at `.unwrap();`.
fn render_setup_init_body(program_name: &str) -> String {
    format!(
"        let mut ctx = TestContext::new();
        let program_id = Pubkey::new_from_array(ID.to_bytes());
        ctx.add_program(&program_id, \"../../target/deploy/{program_name}.so\")
            .unwrap();

        let depositor = Rc::new(Keypair::new());
        ctx.create_account()
            .pubkey(depositor.pubkey())
            .lamports(INITIAL_BALANCE)
            .owner(system_program::ID)
            .create()
            .unwrap();

        let (vault_pda, _) = Pubkey::find_program_address(
            &[b\"vault\", depositor.pubkey().as_ref()],
            &program_id,
        );

        ctx.program(program_id)
            .call(instruction::Initialize {{}})
            .accounts(accounts::Initialize {{
                vault: vault_pda,
                depositor: depositor.pubkey(),
                system_program: system_program::ID,
            }})
            .signers(&[&*depositor])
            .send()
            .unwrap();"
    )
}

// ---------------------------------------------------------------------------
// balance_conservation.
// ---------------------------------------------------------------------------

fn render_crucible_balance(surface: &ContractSurface, candidate: &InvariantCandidate) -> String {
    let h = &candidate.emit_hints;
    let program_name = &surface.program_name;
    let fixture_name = format!(
        "{}Fixture",
        capitalize(&surface.program_name.replace(['_', '-'], ""))
    );
    let invariant_fn = &candidate.name;
    let account_ty = &h.account_type;
    let field = &h.field;
    let expected_field = format!("expected_{}", field);
    let disclosure = disclosure_header(&candidate.source);
    let class = &candidate.class;
    let summary = &candidate.summary;

    let header = render_header_comment_block(
        invariant_fn,
        class,
        &disclosure,
        summary,
        &format!(
"// Fixture-side bookkeeping field: `{expected_field}: u128` — walked
// through every action and asserted against `{account_ty}.{field}`
// after each step."
        ),
    );
    // Balance keeps the disambiguation comment that was historically only
    // present in this renderer — preserves byte-identical output for the
    // pre-2026-06-04 emitted examples.
    let imports = render_imports_and_const(
        program_name,
        "// `::` prefix disambiguates the program crate from a `vault_ref`
// module re-exported via `crucible_fuzzer::*` (rustc E0659 otherwise).
",
    );
    let init_body = render_setup_init_body(program_name);

    format!(
"{header}

{imports}

#[derive(Clone)]
struct {fixture_name} {{
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Fixture-side ledger. Walked through every action; asserted
    /// against on-chain `{account_ty}.{field}` after each step.
    {expected_field}: u128,
}}

#[fuzz_fixture]
impl {fixture_name} {{
    pub fn setup() -> Self {{
{init_body}

        Self {{
            ctx,
            program_id,
            depositor,
            vault_pda,
            {expected_field}: 0,
        }}
    }}

    pub fn action_deposit(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {{
        let ok = self.ctx
            .program(self.program_id)
            .call(instruction::Deposit {{ amount }})
            .accounts(accounts::Deposit {{
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
                system_program: system_program::ID,
            }})
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {{
            // Mirror the on-chain bookkeeping move.
            self.{expected_field} = self.{expected_field}.saturating_add(amount as u128);
        }}
        ok
    }}

    pub fn action_withdraw(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {{
        let ok = self.ctx
            .program(self.program_id)
            .call(instruction::Withdraw {{ amount }})
            .accounts(accounts::Withdraw {{
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
            }})
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {{
            self.{expected_field} = self.{expected_field}.saturating_sub(amount as u128);
        }}
        ok
    }}
}}

// Balance-conservation invariant.
//
// After every action, the on-chain `{account_ty}.{field}` must equal
// the fixture-side ledger (`{expected_field}`). Any drift indicates
// the program's bookkeeping has decoupled from the lamports it
// actually moved — the classic conservation violation.
#[invariant_test]
fn {invariant_fn}(fixture: &mut {fixture_name}) {{
    let vault: {account_ty} = fixture
        .ctx
        .read_anchor_account::<{account_ty}>(&fixture.vault_pda)
        .expect(\"vault PDA initialized in setup\");
    fuzz_assert_eq!(
        vault.{field} as u128,
        fixture.{expected_field},
        \"{account_ty}.{field} drift: on-chain={{}} expected={{}}\",
        vault.{field},
        fixture.{expected_field}
    );
}}
"
    )
}

// ---------------------------------------------------------------------------
// monotonic_accounting.
// ---------------------------------------------------------------------------

fn render_crucible_monotonic(surface: &ContractSurface, candidate: &InvariantCandidate) -> String {
    let h = &candidate.emit_hints;
    let program_name = &surface.program_name;
    let fixture_name = format!(
        "{}MonotonicFixture",
        capitalize(&surface.program_name.replace(['_', '-'], ""))
    );
    let invariant_fn = &candidate.name;
    let account_ty = &h.account_type;
    let field = &h.field;
    let snap_field = format!("last_seen_{}", field);
    let disclosure = disclosure_header(&candidate.source);
    let class = &candidate.class;
    let summary = &candidate.summary;

    let header = render_header_comment_block(
        invariant_fn,
        class,
        &disclosure,
        summary,
        &format!(
"// Fixture-side snapshot field: `{snap_field}: u128` — refreshed on
// every action AFTER the invariant check; the invariant asserts the
// current on-chain value is >= the previously-snapshotted value."
        ),
    );
    let imports = render_imports_and_const(program_name, "");
    let init_body = render_setup_init_body(program_name);

    format!(
"{header}

{imports}

#[derive(Clone)]
struct {fixture_name} {{
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Last-observed value of `{account_ty}.{field}`. The invariant
    /// asserts the current observation is >= this; it is then
    /// refreshed to the current observation.
    {snap_field}: u128,
}}

#[fuzz_fixture]
impl {fixture_name} {{
    pub fn setup() -> Self {{
{init_body}

        Self {{
            ctx,
            program_id,
            depositor,
            vault_pda,
            {snap_field}: 0,
        }}
    }}

    pub fn action_deposit(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {{
        self.ctx
            .program(self.program_id)
            .call(instruction::Deposit {{ amount }})
            .accounts(accounts::Deposit {{
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
                system_program: system_program::ID,
            }})
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }}

    pub fn action_withdraw(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {{
        self.ctx
            .program(self.program_id)
            .call(instruction::Withdraw {{ amount }})
            .accounts(accounts::Withdraw {{
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
            }})
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }}
}}

// Monotonic-accounting invariant.
//
// `{account_ty}.{field}` is a lifetime/cumulative counter — a correct
// implementation never decreases it. The invariant snapshots the
// last observed value in `fixture.{snap_field}`, asserts the current
// value is >= the snapshot, then refreshes the snapshot.
#[invariant_test]
fn {invariant_fn}(fixture: &mut {fixture_name}) {{
    let vault: {account_ty} = fixture
        .ctx
        .read_anchor_account::<{account_ty}>(&fixture.vault_pda)
        .expect(\"vault PDA initialized in setup\");
    let current = vault.{field} as u128;
    fuzz_assert_le!(
        fixture.{snap_field},
        current,
        \"{account_ty}.{field} regressed: snapshot={{}} current={{}}\",
        fixture.{snap_field},
        current
    );
    // Ratchet the snapshot forward.
    fixture.{snap_field} = current;
}}
"
    )
}

// ---------------------------------------------------------------------------
// access_control.
// ---------------------------------------------------------------------------

fn render_crucible_access_control(
    surface: &ContractSurface,
    candidate: &InvariantCandidate,
) -> String {
    let program_name = &surface.program_name;
    let fixture_name = format!(
        "{}AccessFixture",
        capitalize(&surface.program_name.replace(['_', '-'], ""))
    );
    let invariant_fn = &candidate.name;
    // We pull the privileged instruction name from action_names[0]
    // — the suggester always populates it that way.
    let ix_name = candidate
        .emit_hints
        .action_names
        .first()
        .cloned()
        .unwrap_or_else(|| "withdraw".to_string());
    let ix_struct = capitalize_snake(&ix_name);
    let disclosure = disclosure_header(&candidate.source);
    let class = &candidate.class;
    let summary = &candidate.summary;

    let header = render_header_comment_block(
        invariant_fn,
        class,
        &disclosure,
        summary,
"// The fixture pre-deposits a small amount on behalf of the real
// `depositor`, then in every `action_attack_*` arm probes the
// privileged instruction with a freshly-minted attacker `Keypair`
// signing instead of the depositor. The invariant fails iff the
// program returned success on any attacker call.",
    );
    let imports = render_imports_and_const(program_name, "");
    // NB: access_control's setup body is inlined (not reusing
    // render_setup_init_body) because the "Initialize + small deposit"
    // explanatory comment sits between the PDA derivation and the
    // Initialize call — the helper places Initialize last, so reusing
    // it would shift the comment off its anchor by one block. Keep the
    // 25-line duplication here; preserving byte-identical emitted output
    // is worth more than the cut.

    format!(
"{header}

{imports}

#[derive(Clone)]
struct {fixture_name} {{
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Set to `true` on any successful attacker call. The invariant
    /// asserts this stays `false` for the lifetime of the run.
    unauthorized_success_observed: bool,
}}

#[fuzz_fixture]
impl {fixture_name} {{
    pub fn setup() -> Self {{
        let mut ctx = TestContext::new();
        let program_id = Pubkey::new_from_array(ID.to_bytes());
        ctx.add_program(&program_id, \"../../target/deploy/{program_name}.so\")
            .unwrap();

        let depositor = Rc::new(Keypair::new());
        ctx.create_account()
            .pubkey(depositor.pubkey())
            .lamports(INITIAL_BALANCE)
            .owner(system_program::ID)
            .create()
            .unwrap();

        let (vault_pda, _) = Pubkey::find_program_address(
            &[b\"vault\", depositor.pubkey().as_ref()],
            &program_id,
        );

        // Initialize + small deposit so a successful unauthorized
        // {ix_name} is observable as a state change (not just a no-op).
        ctx.program(program_id)
            .call(instruction::Initialize {{}})
            .accounts(accounts::Initialize {{
                vault: vault_pda,
                depositor: depositor.pubkey(),
                system_program: system_program::ID,
            }})
            .signers(&[&*depositor])
            .send()
            .unwrap();

        let _ = ctx
            .program(program_id)
            .call(instruction::Deposit {{ amount: 1_000_000 }})
            .accounts(accounts::Deposit {{
                vault: vault_pda,
                depositor: depositor.pubkey(),
                system_program: system_program::ID,
            }})
            .signers(&[&*depositor])
            .send();

        Self {{
            ctx,
            program_id,
            depositor,
            vault_pda,
            unauthorized_success_observed: false,
        }}
    }}

    /// Attacker arm — probes `{ix_name}` with a freshly-minted
    /// `Keypair` that has never deposited to this vault. A correct
    /// program rejects the call; if the call succeeds, the fixture
    /// trips its sticky flag and the invariant fails.
    pub fn action_attack_{ix_name}(
        &mut self,
        #[range(1..1_000_000)] amount: u64,
    ) -> bool {{
        let attacker = Keypair::new();
        // Fund attacker so a missing signer-check is the only way the
        // call can succeed (otherwise it would fail on rent / fee
        // reasons unrelated to access control).
        let _ = self.ctx
            .create_account()
            .pubkey(attacker.pubkey())
            .lamports(INITIAL_BALANCE)
            .owner(system_program::ID)
            .create();
        let attempted = self.ctx
            .program(self.program_id)
            .call(instruction::{ix_struct} {{ amount }})
            .accounts(accounts::{ix_struct} {{
                vault: self.vault_pda,
                depositor: attacker.pubkey(),
            }})
            .signers(&[&attacker])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if attempted {{
            self.unauthorized_success_observed = true;
        }}
        // Always return true so the fuzzer keeps generating actions.
        true
    }}
}}

// Access-control invariant.
//
// If the program ever accepted a `{ix_name}` call signed by anyone
// other than the recorded depositor, the sticky flag is `true` and
// this assertion fails.
#[invariant_test]
fn {invariant_fn}(fixture: &mut {fixture_name}) {{
    fuzz_assert_eq!(
        fixture.unauthorized_success_observed, false,
        \"unauthorized {ix_name} succeeded on vault {{}}\",
        fixture.vault_pda
    );
}}
"
    )
}

// ---------------------------------------------------------------------------
// Shared helpers.
// ---------------------------------------------------------------------------

fn render_unknown_class(class: &str, candidate: &InvariantCandidate) -> String {
    format!(
        "// emit error: class `{class}` not recognized by cf-invariants-anchor-emit.
// Candidate `{}` ({}) was passed through, but no Crucible rendering
// is registered for it. Either teach `cf-invariants-anchor-emit` how
// to render this class, or remove the candidate from the suggester
// path. See docs/architecture.md §emit-classes.
",
        candidate.name, candidate.class
    )
}

fn render_trident_stub(_surface: &ContractSurface, candidate: &InvariantCandidate) -> String {
    format!(
        "// Trident emit target is staged for Phase 1.
//
// Phase-0 ranking + candidate generation already runs end-to-end for
// `{}` (class = {}), but the Trident `#[init]` / `FuzzData` rendering
// shape is not yet wired. See docs/architecture.md §emit-targets.
",
        candidate.name, candidate.class,
    )
}

fn disclosure_header(source: &InvariantSource) -> String {
    match source {
        InvariantSource::Manual => "Source: Manual.".into(),
        InvariantSource::Heuristic { suggester_version } => format!(
            "Source: Heuristic (suggester v{suggester_version}). \
             No AI suggestion in this candidate."
        ),
        InvariantSource::AiSuggested {
            model,
            prompt_version,
            timestamp_utc,
        } => format!(
            "Source: AI-SUGGESTED, UNVERIFIED until reviewed by author. \
             model={model} prompt_version={prompt_version} timestamp_utc={timestamp_utc}."
        ),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Convert snake_case → CamelCase for the Anchor `instruction::Foo` /
/// `accounts::Foo` shape. `withdraw_admin` → `WithdrawAdmin`.
fn capitalize_snake(s: &str) -> String {
    s.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::{
        BalanceField, ContractSurface, EmitHints, InvariantCandidate, InvariantSource, Instruction,
    };

    fn surface() -> ContractSurface {
        ContractSurface {
            program_id: "Va111tRef1111111111111111111111111111111111".into(),
            program_name: "vault_ref".into(),
            instructions: vec![
                Instruction { name: "deposit".into(),  args: vec!["amount".into()], accounts: vec![] },
                Instruction { name: "withdraw".into(), args: vec!["amount".into()], accounts: vec![] },
            ],
            balance_fields: vec![BalanceField {
                account: "Vault".into(),
                field: "amount".into(),
                ty: "u64".into(),
            }],
        }
    }

    fn balance_candidate() -> InvariantCandidate {
        InvariantCandidate {
            name: "invariant_amount_conservation".into(),
            summary: "Vault.amount == sum(deposits) - sum(withdrawals)".into(),
            class: "balance_conservation".into(),
            rank: 0.92,
            rationale: "rationale".into(),
            emit_hints: EmitHints {
                account_type: "Vault".into(),
                field: "amount".into(),
                expected_expression: "fixture.expected_amount".into(),
                action_names: vec!["deposit".into(), "withdraw".into()],
            },
            source: InvariantSource::Heuristic {
                suggester_version: "0.2.0".into(),
            },
        }
    }

    fn monotonic_candidate() -> InvariantCandidate {
        InvariantCandidate {
            name: "invariant_lifetime_deposited_monotonic".into(),
            summary: "Vault.lifetime_deposited never decreases".into(),
            class: "monotonic_accounting".into(),
            rank: 0.83,
            rationale: "rationale".into(),
            emit_hints: EmitHints {
                account_type: "Vault".into(),
                field: "lifetime_deposited".into(),
                expected_expression: "fixture.last_seen_lifetime_deposited".into(),
                action_names: vec!["deposit".into(), "withdraw".into()],
            },
            source: InvariantSource::Heuristic {
                suggester_version: "0.2.0".into(),
            },
        }
    }

    fn access_control_candidate() -> InvariantCandidate {
        InvariantCandidate {
            name: "invariant_withdraw_rejects_unauthorized".into(),
            summary: "withdraw rejects when invoked by anyone other than depositor".into(),
            class: "access_control".into(),
            rank: 0.78,
            rationale: "rationale".into(),
            emit_hints: EmitHints {
                account_type: "Vault".into(),
                field: "depositor".into(),
                expected_expression: "fixture.vault.depositor".into(),
                action_names: vec!["withdraw".into()],
            },
            source: InvariantSource::Heuristic {
                suggester_version: "0.2.0".into(),
            },
        }
    }

    // -- balance_conservation (Phase 0 contract — preserved verbatim) ----

    #[test]
    fn crucible_emit_uses_real_v0_2_0_api() {
        let out = render(&surface(), &balance_candidate(), Target::Crucible);
        // Real v0.2.0 macros — NOT `crucible::invariants!`.
        assert!(out.contains("#[fuzz_fixture]"));
        assert!(out.contains("#[invariant_test]"));
        assert!(out.contains("fuzz_assert_eq!"));
        assert!(out.contains("use crucible_fuzzer::"));
        // Action arms wired up.
        assert!(out.contains("pub fn action_deposit"));
        assert!(out.contains("pub fn action_withdraw"));
        // Fixture-side ledger named correctly.
        assert!(out.contains("expected_amount"));
        // Invariant function name passes through.
        assert!(out.contains("fn invariant_amount_conservation"));
        // Disclosure header present for heuristic source.
        assert!(out.contains("Source: Heuristic"));
        // NOT the wrong macro from the stale ticket spec.
        assert!(!out.contains("crucible::invariants!"));
    }

    #[test]
    fn trident_emit_is_phase1_stub() {
        let out = render(&surface(), &balance_candidate(), Target::Trident);
        assert!(out.contains("Trident emit target is staged for Phase 1"));
    }

    #[test]
    fn ai_source_renders_ai_disclosure_banner() {
        let mut c = balance_candidate();
        c.source = InvariantSource::AiSuggested {
            model: "claude-sonnet-4-6".into(),
            prompt_version: "invariant_suggestion_v1".into(),
            timestamp_utc: "2026-06-01T18:00:00Z".into(),
        };
        let out = render(&surface(), &c, Target::Crucible);
        assert!(out.contains("Source: AI-SUGGESTED, UNVERIFIED"));
        assert!(out.contains("claude-sonnet-4-6"));
    }

    // -- monotonic_accounting --------------------------------------------

    #[test]
    fn monotonic_emit_uses_le_assertion_and_snapshot() {
        let out = render(&surface(), &monotonic_candidate(), Target::Crucible);
        assert!(out.contains("#[invariant_test]"));
        // Monotonic uses `fuzz_assert_le!(snapshot, current)`.
        assert!(out.contains("fuzz_assert_le!"));
        // Snapshot field named per emit-hint convention.
        assert!(out.contains("last_seen_lifetime_deposited"));
        // Disclosure intact.
        assert!(out.contains("Source: Heuristic"));
        // No conservation-style ledger.
        assert!(!out.contains("expected_amount"));
    }

    #[test]
    fn monotonic_emit_handles_ai_source() {
        let mut c = monotonic_candidate();
        c.source = InvariantSource::AiSuggested {
            model: "claude-sonnet-4-6".into(),
            prompt_version: "invariant_suggestion_v1".into(),
            timestamp_utc: "2026-06-02T12:00:00Z".into(),
        };
        let out = render(&surface(), &c, Target::Crucible);
        assert!(out.contains("Source: AI-SUGGESTED, UNVERIFIED"));
    }

    // -- access_control --------------------------------------------------

    #[test]
    fn access_control_emit_uses_attacker_keypair_and_sticky_flag() {
        let out = render(&surface(), &access_control_candidate(), Target::Crucible);
        assert!(out.contains("#[invariant_test]"));
        // Attacker probe arm uses a freshly-minted Keypair.
        assert!(out.contains("let attacker = Keypair::new();"));
        // Sticky-flag pattern: success on an unauthorized call sets a bool.
        assert!(out.contains("unauthorized_success_observed"));
        // Invariant function name passes through.
        assert!(out.contains("fn invariant_withdraw_rejects_unauthorized"));
        // The attacker arm probes the privileged instruction.
        assert!(out.contains("action_attack_withdraw"));
        // CamelCase mapping snake_case → struct name.
        assert!(out.contains("instruction::Withdraw"));
    }

    #[test]
    fn access_control_emit_picks_up_admin_instruction() {
        let mut c = access_control_candidate();
        c.name = "invariant_admin_close_rejects_unauthorized".into();
        c.emit_hints.action_names = vec!["admin_close".into()];
        let out = render(&surface(), &c, Target::Crucible);
        assert!(out.contains("instruction::AdminClose"));
        assert!(out.contains("action_attack_admin_close"));
    }

    // -- unknown class fallthrough ---------------------------------------

    #[test]
    fn unknown_class_yields_loud_placeholder_not_silent_empty() {
        let mut c = balance_candidate();
        c.class = "totally_made_up".into();
        let out = render(&surface(), &c, Target::Crucible);
        assert!(out.contains("emit error: class `totally_made_up`"));
    }
}
