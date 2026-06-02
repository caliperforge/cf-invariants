// admin_ref — clean variant (access-control reference).
//
// Anchor vault with depositor-gated withdraw: only the recorded
// depositor (verified via Anchor's `has_one = depositor` AND the
// vault-PDA seeds derivation from `depositor.key()`) can move funds
// out. Used by cf-invariants-anchor Phase-2 as the CLEAN side of the
// access_control reference pair.
//
// The planted twin (`references/admin_ref_planted/`) drops both
// constraints from `Withdraw`, allowing any signer to drain any vault
// PDA — the emitted access-control invariant catches that on the
// first attacker probe.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("Ad111tRef1111111111111111111111111111111111");

#[program]
pub mod admin_ref {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.depositor = ctx.accounts.depositor.key();
        vault.amount = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, AdminError::InvalidAmount);

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
            .ok_or(AdminError::Overflow)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, AdminError::InvalidAmount);
        let vault = &mut ctx.accounts.vault;
        require!(amount <= vault.amount, AdminError::InvalidAmount);

        **vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx
            .accounts
            .depositor
            .to_account_info()
            .try_borrow_mut_lamports()? += amount;
        vault.amount = vault
            .amount
            .checked_sub(amount)
            .ok_or(AdminError::Underflow)?;
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
    // Clean: BOTH the PDA seeds derivation AND the has_one constraint
    // bind the vault to the recorded depositor. An attacker passing
    // their own pubkey as `depositor` would fail the seeds check
    // (vault PDA was derived from the real depositor's key); an
    // attacker passing the real depositor's pubkey but signing with
    // a different key would fail Anchor's Signer<'info> verification.
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
pub enum AdminError {
    InvalidAmount,
    Overflow,
    Underflow,
}
