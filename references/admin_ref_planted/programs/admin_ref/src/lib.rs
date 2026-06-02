// admin_ref — PLANTED-BUG variant (access_control violation).
//
// Identical to the clean admin_ref except `Withdraw` drops BOTH the
// `has_one = depositor` constraint AND the `seeds` derivation from
// `depositor.key()`. With both gone, Anchor will accept any account
// owned by the program in the `vault` slot regardless of who signs:
// an attacker can pass the real vault PDA (well-known: it's
// `find_program_address(&[b"vault", real_depositor.key().as_ref()], pid)`),
// plug their own pubkey into `depositor`, sign with their own
// keypair, and walk away with the lamports.
//
// cf-invariants-anchor's access_control invariant detects this by
// probing `withdraw` with a freshly-minted attacker Keypair every
// fuzz round — any successful unauthorized call sets a sticky flag
// the invariant asserts against.
//
// The planted change is in `#[derive(Accounts)] pub struct Withdraw`.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("AdmnRef11111111111111111111111111111111111");

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
    // PLANTED-BUG: neither the PDA seeds derivation nor the
    // `has_one = depositor` constraint is present. Anchor will accept
    // any Vault-typed account owned by this program in the `vault`
    // slot, regardless of which keypair signs as `depositor`. An
    // attacker that knows the real vault PDA can drain it.
    #[account(mut)]
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
