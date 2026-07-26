// invariant_cumulative_yield_monotonic
//
// Emitted by cf-invariants-anchor (IDL-driven codegen) for the monotonic_accounting class.
// Target: Crucible v0.2.0 (asymmetric-research/crucible).
// Source: Heuristic (suggester v0.2.0). No AI suggestion in this candidate.
//
// LendingVault.cumulative_yield never decreases across successive observations
//
// Fixture-side snapshot field: `last_seen_cumulative_yield: u128` — refreshed on
// every action AFTER the invariant check; the invariant asserts the
// current on-chain value is >= the previously-snapshotted value.
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
const PROGRAM_ID: &str = "Km111tRef1111111111111111111111111111111111";
/// Compiled target program. For a deployed program, produce this
/// with: `solana program dump Km111tRef1111111111111111111111111111111111 <name>.so`.
const PROGRAM_SO: &str = "../../target/deploy/kamino_lending_ref.so";

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

/// `accrue_interest` instruction data (discriminator + args from the IDL).
pub struct IxAccrueInterest {
    pub interest: u64,
}
impl anchor_lang::Discriminator for IxAccrueInterest {
    const DISCRIMINATOR: &'static [u8] = &[88u8, 88, 88, 88, 88, 88, 88, 88];
}
impl anchor_lang::AnchorSerialize for IxAccrueInterest {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        anchor_lang::AnchorSerialize::serialize(&self.interest, writer)?;
        Ok(())
    }
}
impl anchor_lang::InstructionData for IxAccrueInterest {}

/// `accrue_interest` account metas (order + writable/signer flags from the IDL).
pub struct IxAccrueInterestAccounts {
    pub vault: Pubkey,
    pub depositor: Pubkey,
}
impl anchor_lang::ToAccountMetas for IxAccrueInterestAccounts {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.vault, false),
            AccountMeta::new_readonly(self.depositor, true),
        ]
    }
}

/// `set_admin` instruction data (discriminator + args from the IDL).
pub struct IxSetAdmin {
    pub new_admin: Pubkey,
}
impl anchor_lang::Discriminator for IxSetAdmin {
    const DISCRIMINATOR: &'static [u8] = &[77u8, 77, 77, 77, 77, 77, 77, 77];
}
impl anchor_lang::AnchorSerialize for IxSetAdmin {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        anchor_lang::AnchorSerialize::serialize(&self.new_admin, writer)?;
        Ok(())
    }
}
impl anchor_lang::InstructionData for IxSetAdmin {}

/// `set_admin` account metas (order + writable/signer flags from the IDL).
pub struct IxSetAdminAccounts {
    pub vault: Pubkey,
    pub depositor: Pubkey,
    pub admin: Pubkey,
}
impl anchor_lang::ToAccountMetas for IxSetAdminAccounts {
    fn to_account_metas(&self, _is_signer: Option<bool>) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.vault, false),
            AccountMeta::new_readonly(self.depositor, false),
            AccountMeta::new_readonly(self.admin, true),
        ]
    }
}

/// `LendingVault` state account layout, from the IDL `types` array.
pub struct LendingVault {
    pub depositor: Pubkey,
    pub admin: Pubkey,
    pub total_assets: u64,
    pub total_shares: u64,
    pub cumulative_yield: u64,
}
impl anchor_lang::AnchorDeserialize for LendingVault {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            depositor: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
            admin: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
            total_assets: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
            total_shares: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
            cumulative_yield: anchor_lang::AnchorDeserialize::deserialize_reader(reader)?,
        })
    }
}

impl anchor_lang::Discriminator for LendingVault {
    const DISCRIMINATOR: &'static [u8] = &[211u8, 8, 232, 43, 2, 152, 117, 119];
}

#[derive(Clone)]
struct KaminolendingrefMonotonicFixture {
    ctx: TestContext,
    program_id: Pubkey,
    a_system_program: Pubkey,
    kp_admin: Rc<Keypair>,
    kp_depositor: Rc<Keypair>,
    a_vault: Pubkey,
    /// Last-observed value of `LendingVault.cumulative_yield`. The invariant
    /// asserts the current observation is >= this; it is then
    /// refreshed to the current observation.
    last_seen_cumulative_yield: u128,
}

#[fuzz_fixture]
impl KaminolendingrefMonotonicFixture {
    pub fn setup() -> Self {
        let mut ctx = TestContext::new();
        let program_id = Pubkey::from_str(PROGRAM_ID)
            .expect("IDL `address` is valid base58");
        ctx.add_program(&program_id, PROGRAM_SO).unwrap();

        // Addresses the IDL pins.
        let a_system_program = Pubkey::from_str("11111111111111111111111111111111")
            .expect("IDL-pinned address is valid base58");

        // One funded Keypair per IDL signer account name.
        let kp_admin = Rc::new(Keypair::new());
        ctx.create_account()
            .pubkey(kp_admin.pubkey())
            .lamports(INITIAL_BALANCE)
            .owner(system_program::ID)
            .create()
            .unwrap();
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
            kp_admin,
            kp_depositor,
            a_vault,
            last_seen_cumulative_yield: 0,
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
        ok
    }

    pub fn action_accrue_interest(&mut self, #[range(1..1_000_000)] interest: u64) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(IxAccrueInterest { interest })
            .accounts(IxAccrueInterestAccounts {
                vault: self.a_vault,
                depositor: self.kp_depositor.pubkey(),
            })
            .signers(&[&*self.kp_depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        ok
    }

    pub fn action_set_admin(&mut self) -> bool {
        let ok = self.ctx
            .program(self.program_id)
            .call(IxSetAdmin { new_admin: self.kp_admin.pubkey() })
            .accounts(IxSetAdminAccounts {
                vault: self.a_vault,
                depositor: self.kp_depositor.pubkey(),
                admin: self.kp_admin.pubkey(),
            })
            .signers(&[&*self.kp_admin])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        ok
    }
}

// Monotonic-accounting invariant.
//
// `LendingVault.cumulative_yield` is a lifetime/cumulative counter — a correct
// implementation never decreases it. The invariant snapshots the
// last observed value, asserts the current value is >= the
// snapshot, then refreshes the snapshot.
#[invariant_test]
fn invariant_cumulative_yield_monotonic(fixture: &mut KaminolendingrefMonotonicFixture) {
    let acct: LendingVault = match fixture
        .ctx
        .read_anchor_account::<LendingVault>(&fixture.a_vault)
    {
        Ok(a) => a,
        Err(_) => return,
    };
    let current = acct.cumulative_yield as u128;
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
