// kamino_lending_ref — PLANTED variant.
//
// Identical to the clean variant EXCEPT for the planted bug inside
// `withdraw`: the underlying paid out is `underlying + 1` (vault "tips"
// the user one lamport per withdraw) while `total_assets` is only
// decremented by `underlying`. After the first withdraw, the on-chain
// `total_assets` value disagrees with the fixture-side ledger (which
// tracks the actual lamport-movement total), and the
// `balance_conservation` invariant fires immediately.
//
// This is a realistic class of Solana lending bug — the kvault
// withdraw handler does `safe_sub` math against the reserve balance and
// is the highest-value attack surface called out in the bounty hunter
// dossier (collateral / share-accounting drift on Critical tier).
//
// Same `declare_id!` as the clean variant — Crucible's TestContext
// loads each `.so` independently into its own LiteSVM and the
// program-id collision is fine because the two never co-exist in one
// fixture run.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("Km111tRef1111111111111111111111111111111111");

pub const SHARE_SCALE: u128 = 1_000_000_000;

#[program]
pub mod kamino_lending_ref {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.depositor = ctx.accounts.depositor.key();
        vault.admin = ctx.accounts.depositor.key();
        vault.total_assets = 0;
        vault.total_shares = 0;
        vault.cumulative_yield = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, LendingError::InvalidAmount);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.depositor.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, amount)?;

        let vault = &mut ctx.accounts.vault;
        let shares_to_mint: u64 = if vault.total_shares == 0 {
            amount
        } else {
            ((amount as u128)
                .checked_mul(vault.total_shares as u128)
                .ok_or(LendingError::Overflow)?
                / (vault.total_assets as u128))
                .try_into()
                .map_err(|_| LendingError::Overflow)?
        };
        vault.total_assets = vault
            .total_assets
            .checked_add(amount)
            .ok_or(LendingError::Overflow)?;
        vault.total_shares = vault
            .total_shares
            .checked_add(shares_to_mint)
            .ok_or(LendingError::Overflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, LendingError::InvalidAmount);
        let vault = &mut ctx.accounts.vault;
        require!(amount <= vault.total_shares, LendingError::InvalidAmount);

        let underlying: u64 = ((amount as u128)
            .checked_mul(vault.total_assets as u128)
            .ok_or(LendingError::Overflow)?
            / (vault.total_shares as u128))
            .try_into()
            .map_err(|_| LendingError::Overflow)?;
        require!(underlying > 0, LendingError::InvalidAmount);

        // Lamport movement matches the underlying payout.
        **vault.to_account_info().try_borrow_mut_lamports()? -= underlying;
        **ctx
            .accounts
            .depositor
            .to_account_info()
            .try_borrow_mut_lamports()? += underlying;

        // PLANTED BUG: bookkeeping debits `underlying + 1` from the
        // accounting field, even though only `underlying` lamports
        // actually left. This is the share-price-drift class of bug:
        // the on-chain `total_assets` field decreases faster than the
        // real holdings, eventually pushing reported share price below
        // the real share price (and, downstream, the next deposit's
        // share-mint math under-pays the depositor).
        // The fuzz fixture's conservation ledger walks
        // `expected_total_assets -= amount`; the on-chain field drifts
        // DOWN by an extra +1 per withdraw, so the
        // balance_conservation invariant fires on the first withdraw.
        let bug_debit = underlying.saturating_add(1);
        vault.total_assets = vault.total_assets.saturating_sub(bug_debit);
        vault.total_shares = vault
            .total_shares
            .checked_sub(amount)
            .ok_or(LendingError::Underflow)?;
        Ok(())
    }

    pub fn accrue_interest(ctx: Context<AccrueInterest>, interest: u64) -> Result<()> {
        require!(interest > 0, LendingError::InvalidAmount);
        let vault = &mut ctx.accounts.vault;
        vault.total_assets = vault
            .total_assets
            .checked_add(interest)
            .ok_or(LendingError::Overflow)?;
        vault.cumulative_yield = vault
            .cumulative_yield
            .checked_add(interest)
            .ok_or(LendingError::Overflow)?;
        Ok(())
    }

    pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.admin = new_admin;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = depositor,
        space = 8 + LendingVault::INIT_SPACE,
        seeds = [b"vault", depositor.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, LendingVault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault", depositor.key().as_ref()],
        bump,
        has_one = depositor,
    )]
    pub vault: Account<'info, LendingVault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", depositor.key().as_ref()],
        bump,
        has_one = depositor,
    )]
    pub vault: Account<'info, LendingVault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
}

#[derive(Accounts)]
pub struct AccrueInterest<'info> {
    #[account(
        mut,
        seeds = [b"vault", depositor.key().as_ref()],
        bump,
        has_one = depositor,
    )]
    pub vault: Account<'info, LendingVault>,
    pub depositor: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetAdmin<'info> {
    #[account(
        mut,
        seeds = [b"vault", depositor.key().as_ref()],
        bump,
        has_one = admin,
    )]
    pub vault: Account<'info, LendingVault>,
    /// CHECK: read-only, identifies which vault PDA to load.
    pub depositor: UncheckedAccount<'info>,
    pub admin: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct LendingVault {
    pub depositor: Pubkey,
    pub admin: Pubkey,
    pub total_assets: u64,
    pub total_shares: u64,
    pub cumulative_yield: u64,
}

#[error_code]
pub enum LendingError {
    InvalidAmount,
    Overflow,
    Underflow,
}
