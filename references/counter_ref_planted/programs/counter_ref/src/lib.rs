// counter_ref — PLANTED-BUG variant (monotonicity violation).
//
// Identical to the clean counter_ref except `withdraw` ALSO decrements
// `vault.lifetime_deposited` by `amount`. `lifetime_deposited` is a
// lifetime cumulative-deposited counter — a ratchet that a correct
// implementation never decreases. Any withdraw call here regresses
// the counter; cf-invariants-anchor's monotonic_accounting invariant
// should detect this in the first withdraw.
//
// The planted line is marked with `// PLANTED-BUG`.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("CntrRef11111111111111111111111111111111111");

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
        // PLANTED-BUG: lifetime_deposited is a ratchet — a lifetime
        // cumulative counter — and must never decrease. The clean
        // variant leaves it alone on withdraw. This planted line
        // regresses it on every withdraw, which cf-invariants-anchor's
        // monotonic_accounting invariant catches in 1 withdraw call.
        vault.lifetime_deposited = vault
            .lifetime_deposited
            .saturating_sub(amount);
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
