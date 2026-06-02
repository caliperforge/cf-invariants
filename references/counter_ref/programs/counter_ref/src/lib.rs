// counter_ref — clean variant (monotonicity reference).
//
// Anchor vault that tracks BOTH a spendable `amount` AND a
// `lifetime_deposited` ratchet — the latter is a lifetime cumulative
// counter that a correct implementation never decreases. `deposit`
// increments both fields; `withdraw` decrements `amount` only.
//
// Used by cf-invariants-anchor Phase-2 as the CLEAN side of the
// monotonicity reference pair. Its planted twin
// (`references/counter_ref_planted/`) breaks the ratchet inside
// `withdraw`, which the emitted monotonic invariant catches.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("Cn111tRef1111111111111111111111111111111111");

#[program]
pub mod counter_ref {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.depositor = ctx.accounts.depositor.key();
        vault.amount = 0;
        vault.lifetime_deposited = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, CounterError::InvalidAmount);

        // anchor-lang 1.0.x: CpiContext::new takes Pubkey, not AccountInfo
        // (changed from 0.30.x); see references/vault_ref/programs/vault_ref
        // for the upstream-verified signature note.
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
            .ok_or(CounterError::Overflow)?;
        // Ratchet: cumulative-deposited counter increments only.
        vault.lifetime_deposited = vault
            .lifetime_deposited
            .checked_add(amount)
            .ok_or(CounterError::Overflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, CounterError::InvalidAmount);
        let vault = &mut ctx.accounts.vault;
        require!(amount <= vault.amount, CounterError::InvalidAmount);

        **vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx
            .accounts
            .depositor
            .to_account_info()
            .try_borrow_mut_lamports()? += amount;
        vault.amount = vault
            .amount
            .checked_sub(amount)
            .ok_or(CounterError::Underflow)?;
        // Clean: `lifetime_deposited` is intentionally NOT touched —
        // it is a lifetime cumulative-deposited ratchet, not a live
        // balance. Any code path that decreases this field is the bug.
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
    pub lifetime_deposited: u64,
}

#[error_code]
pub enum CounterError {
    InvalidAmount,
    Overflow,
    Underflow,
}
