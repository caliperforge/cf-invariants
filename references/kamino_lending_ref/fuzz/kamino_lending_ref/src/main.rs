// invariant_total_assets_conservation
//
// Emitted by cf-invariants-anchor v0.2.0 for the balance_conservation class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// Source: Heuristic (suggester v0.2.0). No AI suggestion in this candidate.
//
// LendingVault.total_assets == fixture-tracked sum of deposits − sum of withdrawals
//
// Fixture-side bookkeeping field: `expected_total_assets: u128` — walked
// through every action and asserted against `LendingVault.total_assets`
// after each step.

#![allow(unused_imports)]

use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
// `::` prefix disambiguates the program crate from a `vault_ref`
// module re-exported via `crucible_fuzzer::*` (rustc E0659 otherwise).
use ::kamino_lending_ref::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

const INITIAL_BALANCE: u64 = 10_000_000_000;

#[derive(Clone)]
struct KaminolendingrefFixture {
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Fixture-side ledger. Walked through every action; asserted
    /// against on-chain `LendingVault.total_assets` after each step.
    expected_total_assets: u128,
}

#[fuzz_fixture]
impl KaminolendingrefFixture {
    pub fn setup() -> Self {
        let mut ctx = TestContext::new();
        let program_id = Pubkey::new_from_array(ID.to_bytes());
        ctx.add_program(&program_id, "../../target/deploy/kamino_lending_ref.so")
            .unwrap();

        let depositor = Rc::new(Keypair::new());
        ctx.create_account()
            .pubkey(depositor.pubkey())
            .lamports(INITIAL_BALANCE)
            .owner(system_program::ID)
            .create()
            .unwrap();

        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", depositor.pubkey().as_ref()],
            &program_id,
        );

        ctx.program(program_id)
            .call(instruction::Initialize {})
            .accounts(accounts::Initialize {
                vault: vault_pda,
                depositor: depositor.pubkey(),
                system_program: system_program::ID,
            })
            .signers(&[&*depositor])
            .send()
            .unwrap();

        Self {
            ctx,
            program_id,
            depositor,
            vault_pda,
            expected_total_assets: 0,
        }
    }

    pub fn action_deposit(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(instruction::Deposit { amount })
            .accounts(accounts::Deposit {
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
                system_program: system_program::ID,
            })
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {
            // Mirror the on-chain bookkeeping move.
            self.expected_total_assets = self.expected_total_assets.saturating_add(amount as u128);
        }
        ok
    }

    pub fn action_withdraw(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(instruction::Withdraw { amount })
            .accounts(accounts::Withdraw {
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
            })
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {
            self.expected_total_assets = self.expected_total_assets.saturating_sub(amount as u128);
        }
        ok
    }
}

// Balance-conservation invariant.
//
// After every action, the on-chain `LendingVault.total_assets` must equal
// the fixture-side ledger (`expected_total_assets`). Any drift indicates
// the program's bookkeeping has decoupled from the lamports it
// actually moved — the classic conservation violation.
#[invariant_test]
fn invariant_total_assets_conservation(fixture: &mut KaminolendingrefFixture) {
    let vault: LendingVault = fixture
        .ctx
        .read_anchor_account::<LendingVault>(&fixture.vault_pda)
        .expect("vault PDA initialized in setup");
    fuzz_assert_eq!(
        vault.total_assets as u128,
        fixture.expected_total_assets,
        "LendingVault.total_assets drift: on-chain={} expected={}",
        vault.total_assets,
        fixture.expected_total_assets
    );
}
