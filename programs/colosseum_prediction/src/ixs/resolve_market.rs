use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

use crate::errors::ErrorCode;
use crate::state::{AdminConfig, Market, MarketOutcome, ResolutionStatus};
use crate::constants::{USDT_MINT_PUBKEY, USDC_MINT_PUBKEY, PRICE_SCALE};
use crate::utils::prepare_market_id_seed;

// -----------------------------
// EVENT
// -----------------------------
#[event]
pub struct MarketResolved {
    pub market: Pubkey,
    pub outcome: MarketOutcome,
    pub pool_total: u64,
    pub total_winning_shares: u64,
    pub payout_per_share: u64,
    pub remainder: u64,
}

// -----------------------------
// INSTRUCTION
// -----------------------------
pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: MarketOutcome) -> Result<()> {
    let market = &mut ctx.accounts.market;

    // -----------------------------
    // AUTHORIZATION
    // -----------------------------
    require!(
        ctx.accounts.admin_config.admins.contains(&ctx.accounts.authority.key()),
        ErrorCode::Unauthorized
    );

    // -----------------------------
    // STATE CHECKS
    // -----------------------------
    require!(
        market.resolution_status == ResolutionStatus::Open,
        ErrorCode::MarketAlreadyResolved
    );

    require!(
        !market.settle_initialized,
        ErrorCode::AlreadySettled
    );

    // Prevent late deposits manipulation
    require!(
        market.trading_paused,
        ErrorCode::MarketStillActive
    );

    // -----------------------------
    // VALIDATE VAULT OWNERSHIP
    // -----------------------------
    require!(
        ctx.accounts.market_usdt_vault.owner == market.key(),
        ErrorCode::InvalidVault
    );

    require!(
        ctx.accounts.market_usdc_vault.owner == market.key(),
        ErrorCode::InvalidVault
    );

    // -----------------------------
    // SET OUTCOME
    // -----------------------------
    require!(outcome != MarketOutcome::Cancelled, ErrorCode::MarketCancelled);

    market.resolution_status = ResolutionStatus::Resolved;
    market.outcome = outcome;

    // -----------------------------
    // SNAPSHOT VAULT BALANCES
    // -----------------------------
    let usdt_balance = ctx.accounts.market_usdt_vault.amount;
    let usdc_balance = ctx.accounts.market_usdc_vault.amount;

    // Optional: enforce same decimals
    require!(
        ctx.accounts.usdt_mint.decimals == ctx.accounts.usdc_mint.decimals,
        ErrorCode::InvalidMintDecimals
    );

    let pool_total = usdt_balance
        .checked_add(usdc_balance)
        .ok_or(ErrorCode::MathOverflow)?;

    // -----------------------------
    // DETERMINE WINNING SHARES
    // -----------------------------
    let total_winning_shares: u64 = match market.outcome {
        MarketOutcome::Yes => market.total_yes_shares,
        MarketOutcome::No => market.total_no_shares,
        MarketOutcome::Option { index } => {
            let i = index as usize;
            require!(i < market.total_option_shares.len(), ErrorCode::InvalidOptionIndex);
            market.total_option_shares[i]
        }
        _ => return err!(ErrorCode::InvalidOutcome),
    };

    // -----------------------------
    // CALCULATE PAYOUT
    // -----------------------------
    // Zero winning-side shares: resolve with no per-share distribution (`settle_payout_per_share` = 0,
    // `settle_remainder` = 0 in the same units as the pro-rata dust field). Stranded vault balance is
    // captured in `settle_total_pool`; claims on the winning path remain unavailable (see claim).
    let (payout_per_share, remainder) = if total_winning_shares == 0 {
        (0u64, 0u64)
    } else {
        let numerator = (pool_total as u128)
            .checked_mul(PRICE_SCALE as u128)
            .ok_or(ErrorCode::MathOverflow)?;
    
        let payout_per_share_u128 = numerator
            .checked_div(total_winning_shares as u128)
            .ok_or(ErrorCode::MathOverflow)?;
    
        let remainder = numerator
            .checked_rem(total_winning_shares as u128)
            .ok_or(ErrorCode::MathOverflow)?;
    
        require!(
            payout_per_share_u128 <= u64::MAX as u128,
            ErrorCode::MathOverflow
        );
    
        (payout_per_share_u128 as u64, remainder as u64)
    };

    // -----------------------------
    // STORE SNAPSHOT
    // -----------------------------
    market.settle_total_pool = pool_total;
    market.settle_total_winning_shares = total_winning_shares;
    market.settle_payout_per_share = payout_per_share;
    market.settle_remainder = remainder;
    market.settle_initialized = true;

    // -----------------------------
    // EMIT EVENT
    // -----------------------------
    emit!(MarketResolved {
        market: market.key(),
        outcome,
        pool_total,
        total_winning_shares,
        payout_per_share,
        remainder,
    });

    Ok(())
}

// -----------------------------
// ACCOUNTS
// -----------------------------
#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump = market.bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        seeds = [b"admin_config"],
        bump
    )]
    pub admin_config: Box<Account<'info, AdminConfig>>,

    pub authority: Signer<'info>,

    // -----------------------------
    // VAULTS
    // -----------------------------
    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = market
    )]
    pub market_usdt_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = market
    )]
    pub market_usdc_vault: Box<Account<'info, TokenAccount>>,

    // -----------------------------
    // MINTS
    // -----------------------------
    #[account(
        constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress
    )]
    pub usdt_mint: Box<Account<'info, Mint>>,

    #[account(
        constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress
    )]
    pub usdc_mint: Box<Account<'info, Mint>>,
}