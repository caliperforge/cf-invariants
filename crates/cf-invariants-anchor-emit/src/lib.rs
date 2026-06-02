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

use cf_invariants_anchor_core::{ContractSurface, InvariantCandidate, InvariantSource};

/// Which downstream harness we render for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Asymmetric Research's Crucible (LibAFL + LiteSVM). Phase 0 default.
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

    let mut s = String::new();
    s.push_str(&format!(
"// {invariant_fn}
//
// Emitted by cf-invariants-anchor v0.1.0 for the {class} class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// {disclosure}
//
// {summary}
//
// Fixture-side bookkeeping field: `{expected_field}: u128` — walked
// through every action and asserted against `{account_ty}.{field}`
// after each step.

#![allow(unused_imports)]

use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
// `::` prefix disambiguates the program crate from a `vault_ref`
// module re-exported via `crucible_fuzzer::*` (rustc E0659 otherwise).
use ::{program_name}::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

const INITIAL_BALANCE: u64 = 10_000_000_000;

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
"));
    s
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

#[cfg(test)]
mod tests {
    use super::*;
    use cf_invariants_anchor_core::{
        BalanceField, ContractSurface, EmitHints, InvariantCandidate, InvariantSource, Instruction,
    };

    fn fix() -> (ContractSurface, InvariantCandidate) {
        let surface = ContractSurface {
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
        };
        let candidate = InvariantCandidate {
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
                suggester_version: "0.1.0".into(),
            },
        };
        (surface, candidate)
    }

    #[test]
    fn crucible_emit_uses_real_v0_2_0_api() {
        let (surface, candidate) = fix();
        let out = render(&surface, &candidate, Target::Crucible);
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
        let (surface, candidate) = fix();
        let out = render(&surface, &candidate, Target::Trident);
        assert!(out.contains("Trident emit target is staged for Phase 1"));
    }

    #[test]
    fn ai_source_renders_ai_disclosure_banner() {
        let (surface, mut candidate) = fix();
        candidate.source = InvariantSource::AiSuggested {
            model: "claude-sonnet-4-6".into(),
            prompt_version: "invariant_suggestion_v1".into(),
            timestamp_utc: "2026-06-01T18:00:00Z".into(),
        };
        let out = render(&surface, &candidate, Target::Crucible);
        assert!(out.contains("Source: AI-SUGGESTED, UNVERIFIED"));
        assert!(out.contains("claude-sonnet-4-6"));
    }
}
