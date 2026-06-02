// invariant_lifetime_deposited_monotonic
//
// Emitted by cf-invariants-anchor v0.2.0 for the monotonic_accounting class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// Source: Heuristic (suggester v0.2.0). No AI suggestion in this candidate.
//
// Vault.lifetime_deposited never decreases across successive observations
//
// Fixture-side snapshot field: `last_seen_lifetime_deposited: u128` — refreshed on
// every action AFTER the invariant check; the invariant asserts the
// current on-chain value is >= the previously-snapshotted value.

#![allow(unused_imports)]

use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
use ::counter_ref::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

const INITIAL_BALANCE: u64 = 10_000_000_000;

#[derive(Clone)]
struct CounterrefMonotonicFixture {
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Last-observed value of `Vault.lifetime_deposited`. The invariant
    /// asserts the current observation is >= this; it is then
    /// refreshed to the current observation.
    last_seen_lifetime_deposited: u128,
}

#[fuzz_fixture]
impl CounterrefMonotonicFixture {
    pub fn setup() -> Self {
        let mut ctx = TestContext::new();
        let program_id = Pubkey::new_from_array(ID.to_bytes());
        ctx.add_program(&program_id, "../../target/deploy/counter_ref.so")
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
            last_seen_lifetime_deposited: 0,
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
// `Vault.lifetime_deposited` is a lifetime/cumulative counter — a correct
// implementation never decreases it. The invariant snapshots the
// last observed value in `fixture.last_seen_lifetime_deposited`, asserts the current
// value is >= the snapshot, then refreshes the snapshot.
#[invariant_test]
fn invariant_lifetime_deposited_monotonic(fixture: &mut CounterrefMonotonicFixture) {
    let vault: Vault = fixture
        .ctx
        .read_anchor_account::<Vault>(&fixture.vault_pda)
        .expect("vault PDA initialized in setup");
    let current = vault.lifetime_deposited as u128;
    fuzz_assert_le!(
        fixture.last_seen_lifetime_deposited,
        current,
        "Vault.lifetime_deposited regressed: snapshot={} current={}",
        fixture.last_seen_lifetime_deposited,
        current
    );
    // Ratchet the snapshot forward.
    fixture.last_seen_lifetime_deposited = current;
}
