// invariant_set_admin_rejects_unauthorized
//
// Emitted by cf-invariants-anchor v0.2.0 for the access_control class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// Source: Heuristic (suggester v0.2.0). No AI suggestion in this candidate.
//
// set_admin rejects when invoked by anyone other than the authorized signer
//
// The fixture pre-deposits a small amount on behalf of the real
// `depositor`, then in every `action_attack_*` arm probes the
// privileged instruction with a freshly-minted attacker `Keypair`
// signing instead of the depositor. The invariant fails iff the
// program returned success on any attacker call.

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
struct KaminolendingrefAccessFixture {
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    vault_pda: Pubkey,
    /// Set to `true` on any successful attacker call. The invariant
    /// asserts this stays `false` for the lifetime of the run.
    unauthorized_success_observed: bool,
}

#[fuzz_fixture]
impl KaminolendingrefAccessFixture {
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

        // Initialize + small deposit so a successful unauthorized
        // set_admin is observable as a state change (not just a no-op).
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

        let _ = ctx
            .program(program_id)
            .call(instruction::Deposit { amount: 1_000_000 })
            .accounts(accounts::Deposit {
                vault: vault_pda,
                depositor: depositor.pubkey(),
                system_program: system_program::ID,
            })
            .signers(&[&*depositor])
            .send();

        Self {
            ctx,
            program_id,
            depositor,
            vault_pda,
            unauthorized_success_observed: false,
        }
    }

    /// Attacker arm — probes `set_admin` with a freshly-minted
    /// `Keypair` that has never deposited to this vault. A correct
    /// program rejects the call; if the call succeeds, the fixture
    /// trips its sticky flag and the invariant fails.
    pub fn action_attack_set_admin(
        &mut self,
        #[range(1..1_000_000)] amount: u64,
    ) -> bool {
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
            .call(instruction::SetAdmin { amount })
            .accounts(accounts::SetAdmin {
                vault: self.vault_pda,
                depositor: attacker.pubkey(),
            })
            .signers(&[&attacker])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if attempted {
            self.unauthorized_success_observed = true;
        }
        // Always return true so the fuzzer keeps generating actions.
        true
    }
}

// Access-control invariant.
//
// If the program ever accepted a `set_admin` call signed by anyone
// other than the recorded depositor, the sticky flag is `true` and
// this assertion fails.
#[invariant_test]
fn invariant_set_admin_rejects_unauthorized(fixture: &mut KaminolendingrefAccessFixture) {
    fuzz_assert_eq!(
        fixture.unauthorized_success_observed, false,
        "unauthorized set_admin succeeded on vault {}",
        fixture.vault_pda
    );
}
