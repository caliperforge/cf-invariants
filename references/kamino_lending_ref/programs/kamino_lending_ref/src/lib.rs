// kamino_lending_ref — clean variant.
//
// Faithful Kamino-class lending vault reference: share-based accounting
// (deposit→mint shares, withdraw→burn shares), monotonic interest
// accrual, and an admin-gated config update. Modeled structurally on
// Kamino's kvault (https://github.com/Kamino-Finance/kvault) — same
// shape, clean-room re-authored under Apache-2.0; we do not vendor BUSL
// upstream source.
//
// Used by cf-invariants-anchor Phase-3 (Kamino build-to-win artifact)
// as the CLEAN side of the lending-vault reference pair. Its planted
// twin (`references/kamino_lending_ref_planted/`) plants a withdraw
// rounding bug that drains the vault by 1 lamport per call — caught by
// the balance_conservation invariant.
//
// Note: this is single-depositor for harness simplicity (matches the
// vault_ref / counter_ref / admin_ref pattern). The structural shape
// — total_assets / total_shares / cumulative_yield / admin gate — is
// what the invariant classes key off, and is what kvault exposes too.
#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("Km111tRef1111111111111111111111111111111111");

/// Share-price math scale. 1 share = 1 underlying at vault genesis.
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

    /// Deposit `amount` underlying lamports; mint shares pro-rata against
    /// the current share price. At genesis (total_shares == 0), shares
    /// are minted 1:1 with the underlying.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, LendingError::InvalidAmount);

        // anchor-lang 1.0.x CpiContext::new signature, see vault_ref.
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
            // shares = amount * total_shares / total_assets
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

    /// Withdraw by burning `shares`; pay out the pro-rata underlying.
    /// Integer-divides in favor of the vault (residual lamport stays in
    /// the vault, i.e. accrues to remaining shareholders) — this is the
    /// audit-safe rounding direction.
    /// Withdraw `amount` units (at the genesis 1:1 vault ratio, units
    /// are simultaneously shares-to-burn and underlying-lamports-out).
    /// We name the arg `amount` to align with the cf-invariants-anchor
    /// movement-class emit convention (which deposits a u64 named
    /// `amount` against the fixture-side ledger field).
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, LendingError::InvalidAmount);
        let vault = &mut ctx.accounts.vault;
        require!(amount <= vault.total_shares, LendingError::InvalidAmount);

        // underlying = amount * total_assets / total_shares
        // Round DOWN (favors vault, never user) — the audit-safe direction.
        let underlying: u64 = ((amount as u128)
            .checked_mul(vault.total_assets as u128)
            .ok_or(LendingError::Overflow)?
            / (vault.total_shares as u128))
            .try_into()
            .map_err(|_| LendingError::Overflow)?;
        require!(underlying > 0, LendingError::InvalidAmount);

        **vault.to_account_info().try_borrow_mut_lamports()? -= underlying;
        **ctx
            .accounts
            .depositor
            .to_account_info()
            .try_borrow_mut_lamports()? += underlying;
        vault.total_assets = vault
            .total_assets
            .checked_sub(underlying)
            .ok_or(LendingError::Underflow)?;
        vault.total_shares = vault
            .total_shares
            .checked_sub(amount)
            .ok_or(LendingError::Underflow)?;
        // cumulative_yield is a lifetime ratchet — never touched here.
        Ok(())
    }

    /// Accrue `interest` lamports into the vault. Caller is expected to
    /// be the admin (rate model lives upstream); for this reference we
    /// model interest as an externally-injected number. Each call:
    ///   - Increases total_assets (vault is now worth more per share)
    ///   - Increases cumulative_yield (lifetime ratchet, audit field)
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

    /// Admin-gated: rotate the admin authority on the vault. Reference
    /// for the access_control class — `has_one = admin` ties the
    /// instruction to the current `vault.admin`, so a non-admin signer
    /// hits an Anchor account-validation error.
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
        // Authority-gate: the CURRENT admin must sign. Anchor produces
        // a validation error on mismatch — the access_control invariant
        // probes this with a freshly-generated attacker keypair.
        has_one = admin,
    )]
    pub vault: Account<'info, LendingVault>,
    /// CHECK: read-only — the depositor identifies which vault PDA to
    /// load (PDA derivation only). Not required to be a signer.
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
    /// Lifetime cumulative interest paid into the vault. Ratchet field —
    /// the monotonic_accounting invariant catches any decrement.
    pub cumulative_yield: u64,
}

#[error_code]
pub enum LendingError {
    InvalidAmount,
    Overflow,
    Underflow,
}
