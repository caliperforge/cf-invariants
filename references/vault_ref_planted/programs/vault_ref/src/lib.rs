// vault_ref — PLANTED-BUG variant.
//
// Identical to the clean vault_ref except `withdraw` decrements
// `vault.amount` by `amount - 1` (saturating) instead of `amount`. The
// real lamport balance still moves by `amount`, but the bookkeeping
// field over-counts by 1 per withdraw — after N withdraws the contract
// believes it holds N more lamports than it actually does. This is the
// balance-conservation violation cf-invariants-anchor Phase 0 detects.
//
// The planted line is marked with `// PLANTED-BUG`.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("Va111tRef1111111111111111111111111111111111");

#[program]
pub mod vault_ref {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.depositor = ctx.accounts.depositor.key();
        vault.amount = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        // anchor-lang 1.0.x: CpiContext::new takes Pubkey, not AccountInfo
        // (changed from the 0.30.x signature). The system_program's
        // AccountInfo lives in the Accounts struct for the runtime; only
        // the program_id flows through CpiContext now.
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.depositor.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, amount)?;

        let vault = &mut ctx.accounts.vault;
        vault.amount = vault
            .amount
            .checked_add(amount)
            .ok_or(VaultError::Overflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        let vault = &mut ctx.accounts.vault;
        require!(amount <= vault.amount, VaultError::InvalidAmount);

        **vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx
            .accounts
            .depositor
            .to_account_info()
            .try_borrow_mut_lamports()? += amount;
        // PLANTED-BUG: bookkeeping decrements by amount-1, not amount.
        // Real lamports still moved by `amount`; vault.amount over-counts
        // by 1 per withdraw call. cf-invariants-anchor's conservation
        // invariant should find this.
        vault.amount = vault.amount.saturating_sub(amount.saturating_sub(1));
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = depositor,
        space = 8 + Vault::INIT_SPACE,
        seeds = [b"vault", depositor.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, Vault>,
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
    pub vault: Account<'info, Vault>,
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
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub depositor: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum VaultError {
    InvalidAmount,
    Overflow,
    Underflow,
}
