// invariant_cumulative_yield_monotonic
//
// Emitted by cf-invariants-anchor v0.2.0 for the monotonic_accounting class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// Source: Heuristic (suggester v0.2.0). No AI suggestion in this candidate.
//
// LendingVault.cumulative_yield never decreases across successive observations
//
// Fixture-side snapshot field: `last_seen_cumulative_yield: u128` — refreshed on
// every action AFTER the invariant check; the invariant asserts the
// current on-chain value is >= the previously-snapshotted value.

#![allow(unused_imports)]

use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
use ::kamino_lending_ref::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

const INITIAL_BALANCE: u64 = 10_000_000_000;

#[derive(Clone)]
struct KaminolendingrefMonotonicFixture {
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Last-observed value of `LendingVault.cumulative_yield`. The invariant
    /// asserts the current observation is >= this; it is then
    /// refreshed to the current observation.
    last_seen_cumulative_yield: u128,
}

#[fuzz_fixture]
impl KaminolendingrefMonotonicFixture {
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
            last_seen_cumulative_yield: 0,
        }
    }

    pub fn action_deposit(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        self.ctx
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
            .unwrap_or(false)
    }

    pub fn action_withdraw(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        self.ctx
            .program(self.program_id)
            .call(instruction::Withdraw { amount })
            .accounts(accounts::Withdraw {
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
            })
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }
}

// Monotonic-accounting invariant.
//
// `LendingVault.cumulative_yield` is a lifetime/cumulative counter — a correct
// implementation never decreases it. The invariant snapshots the
// last observed value in `fixture.last_seen_cumulative_yield`, asserts the current
// value is >= the snapshot, then refreshes the snapshot.
#[invariant_test]
fn invariant_cumulative_yield_monotonic(fixture: &mut KaminolendingrefMonotonicFixture) {
    let vault: LendingVault = fixture
        .ctx
        .read_anchor_account::<LendingVault>(&fixture.vault_pda)
        .expect("vault PDA initialized in setup");
    let current = vault.cumulative_yield as u128;
    fuzz_assert_le!(
        fixture.last_seen_cumulative_yield,
        current,
        "LendingVault.cumulative_yield regressed: snapshot={} current={}",
        fixture.last_seen_cumulative_yield,
        current
    );
    // Ratchet the snapshot forward.
    fixture.last_seen_cumulative_yield = current;
}
