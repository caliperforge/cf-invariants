// invariant_amount_conservation
//
// Emitted by cf-invariants-anchor (IDL-driven codegen) for the balance_conservation class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// Source: Heuristic (suggester v0.3.0). No AI suggestion in this candidate.
//
// Vault.amount == fixture-tracked sum of deposits − sum of withdrawals
//
// Fixture-side bookkeeping field: `expected_amount: u128` — walked
// through every action and asserted against `Vault.amount`
// after each step.
//
// Every instruction shape below — names, discriminators, argument
// types, account lists, PDA seeds, fixed addresses — is rendered
// from this program's Anchor IDL. The target crate is NOT a
// dependency of this harness: the bindings are standalone, so the
// target's anchor-lang version and module layout do not constrain
// the build.

#![allow(unused_imports, dead_code, unused_variables, clippy::too_many_arguments)]

use crucible_fuzzer::anchor_lang::solana_program::instruction::AccountMeta;
use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::anchor_lang::{self};
use crucible_fuzzer::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;
use std::str::FromStr;

const INITIAL_BALANCE: u64 = 10_000_000_000;
/// Program id, from the IDL `address` field.
const PROGRAM_ID: &str = "Va111tRef1111111111111111111111111111111111";
/// Compiled target program. For a deployed program, produce this
/// with: `solana program dump Va111tRef1111111111111111111111111111111111 <name>.so`.
const PROGRAM_SO: &str = "../../target/deploy/vault_ref.so";

// ---------------------------------------------------------------------------
// IDL-derived client bindings (generated).
// ---------------------------------------------------------------------------

/// `initialize` instruction data (discriminator + args from the IDL).
pub struct IxInitialize {
}
impl anchor_lang::Discriminator for IxInitialize {
    const DISCRIMINATOR: &'static [u8] = &[175u8, 175, 109, 31, 13, 152, 155, 237];
}
impl anchor_lang::AnchorSerialize for IxInitialize {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}
impl anchor_lang::InstructionData for IxInitialize {}

/// `initialize` account metas (order + writable/signer flags from the IDL).
pub struct IxInitializeAccounts {
    pub vault: Pubkey,
    pub depositor: Pubkey,
    pub system_program: Pubkey,
}
impl anchor_lang::ToAccountMetas for IxInitializeAccounts {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.vault, false),
            AccountMeta::new(self.depositor, true),
            AccountMeta::new_readonly(self.system_program, false),
        ]
    }
}

/// `deposit` instruction data (discriminator + args from the IDL).
pub struct IxDeposit {
    pub amount: u64,
}
impl anchor_lang::Discriminator for IxDeposit {
    const DISCRIMINATOR: &'static [u8] = &[242u8, 35, 198, 137, 82, 225, 242, 182];
}
impl anchor_lang::AnchorSerialize for IxDeposit {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        anchor_lang::AnchorSerialize::serialize(&self.amount, writer)?;
        Ok(())
    }
}
impl anchor_lang::InstructionData for IxDeposit {}

/// `deposit` account metas (order + writable/signer flags from the IDL).
pub struct IxDepositAccounts {
    pub vault: Pubkey,
    pub depositor: Pubkey,
    pub system_program: Pubkey,
}
impl anchor_lang::ToAccountMetas for IxDepositAccounts {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.vault, false),
            AccountMeta::new(self.depositor, true),
            AccountMeta::new_readonly(self.system_program, false),
        ]
    }
}

/// `withdraw` instruction data (discriminator + args from the IDL).
pub struct IxWithdraw {
    pub amount: u64,
}
impl anchor_lang::Discriminator for IxWithdraw {
    const DISCRIMINATOR: &'static [u8] = &[183u8, 18, 70, 156, 148, 109, 161, 34];
}
impl anchor_lang::AnchorSerialize for IxWithdraw {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        anchor_lang::AnchorSerialize::serialize(&self.amount, writer)?;
        Ok(())
    }
}
impl anchor_lang::InstructionData for IxWithdraw {}

/// `withdraw` account metas (order + writable/signer flags from the IDL).
pub struct IxWithdrawAccounts {
    pub vault: Pubkey,
    pub depositor: Pubkey,
}
impl anchor_lang::ToAccountMetas for IxWithdrawAccounts {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.vault, false),
            AccountMeta::new(self.depositor, true),
        ]
    }
}

/// `Vault` state account layout, from the IDL `types` array.
pub struct Vault {
    pub depositor: Pubkey,
    pub amount: u64,
}
impl anchor_lang::AnchorDeserialize for Vault {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            depositor: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
            amount: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
        })
    }
}

impl anchor_lang::Discriminator for Vault {
    const DISCRIMINATOR: &'static [u8] = &[211u8, 8, 232, 43, 2, 152, 117, 119];
}

#[derive(Clone)]
struct VaultrefFixture {
    ctx: TestContext,
    program_id: Pubkey,
    a_system_program: Pubkey,
    kp_depositor: Rc<Keypair>,
    a_vault: Pubkey,
    /// Fixture-side ledger. Walked through every action; asserted
    /// against on-chain `Vault.amount` after each step.
    expected_amount: u128,
}

#[fuzz_fixture]
impl VaultrefFixture {
    pub fn setup() -> Self {
        let mut ctx = TestContext::new();
        let program_id = Pubkey::from_str(PROGRAM_ID)
            .expect("IDL `address` is valid base58");
        ctx.add_program(&program_id, PROGRAM_SO).unwrap();

        // Addresses the IDL pins.
        let a_system_program = Pubkey::from_str("11111111111111111111111111111111")
            .expect("IDL-pinned address is valid base58");

        // One funded Keypair per IDL signer account name.
        let kp_depositor = Rc::new(Keypair::new());
        ctx.create_account()
            .pubkey(kp_depositor.pubkey())
            .lamports(INITIAL_BALANCE)
            .owner(system_program::ID)
            .create()
            .unwrap();

        // PDA addresses, derived from the IDL seed specs.
        let a_vault = Pubkey::find_program_address(&[&[118u8, 97, 117, 108, 116][..], kp_depositor.pubkey().as_ref()], &program_id).0;

        // Best-effort state bootstrap: initializer-flavored
        // instructions once each, minimal argument values. Failures
        // are tolerated — preconditions the IDL cannot express (e.g.
        // pre-existing mints or config accounts) make them fail loudly
        // at run time, never silently at emit time.
        let _ = ctx
            .program(program_id)
            .call(IxInitialize {})
            .accounts(IxInitializeAccounts {
                vault: a_vault,
                depositor: kp_depositor.pubkey(),
                system_program: a_system_program,
            })
            .signers(&[&*kp_depositor])
            .send();

        Self {
            ctx,
            program_id,
            a_system_program,
            kp_depositor,
            a_vault,
            expected_amount: 0,
        }
    }

    pub fn action_initialize(&mut self) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(IxInitialize {})
            .accounts(IxInitializeAccounts {
                vault: self.a_vault,
                depositor: self.kp_depositor.pubkey(),
                system_program: self.a_system_program,
            })
            .signers(&[&*self.kp_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        ok
    }

    pub fn action_deposit(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(IxDeposit { amount })
            .accounts(IxDepositAccounts {
                vault: self.a_vault,
                depositor: self.kp_depositor.pubkey(),
                system_program: self.a_system_program,
            })
            .signers(&[&*self.kp_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {
            // Mirror the on-chain bookkeeping move (suggester hint:
            // `amount` moves the tracked field on `deposit`).
            self.expected_amount = self.expected_amount.saturating_add(amount as u128);
        }
        ok
    }

    pub fn action_withdraw(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(IxWithdraw { amount })
            .accounts(IxWithdrawAccounts {
                vault: self.a_vault,
                depositor: self.kp_depositor.pubkey(),
            })
            .signers(&[&*self.kp_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if ok {
            // Mirror the on-chain bookkeeping move (suggester hint:
            // `amount` moves the tracked field on `withdraw`).
            self.expected_amount = self.expected_amount.saturating_sub(amount as u128);
        }
        ok
    }
}

// Balance-conservation invariant.
//
// After every action, the on-chain `Vault.amount` must equal
// the fixture-side ledger (`expected_amount`). Any drift indicates
// the program's bookkeeping has decoupled from the value it
// actually moved — the classic conservation violation.
#[invariant_test]
fn invariant_amount_conservation(fixture: &mut VaultrefFixture) {
    // Discriminator-checked read; before the account exists (or if
    // the name-binding was wrong) the read fails and the invariant
    // is vacuous rather than a false alarm.
    let acct: Vault = match fixture
        .ctx
        .read_anchor_account::<Vault>(&fixture.a_vault)
    {
        Ok(a) => a,
        Err(_) => return,
    };
    fuzz_assert_eq!(
        acct.amount as u128,
        fixture.expected_amount,
        "Vault.amount drift: on-chain={} expected={}",
        acct.amount,
        fixture.expected_amount
    );
}
