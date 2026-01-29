use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

use crate::errors::ErrorCode;
use crate::state::{AdminConfig, Market, MarketOutcome, ResolutionStatus};
use crate::constants::{USDT_MINT_PUBKEY, USDC_MINT_PUBKEY, PRICE_SCALE};
use crate::utils::prepare_market_id_seed;

pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: MarketOutcome) -> Result<()> {
    require!(
        ctx.accounts.admin_config.admins.contains(&ctx.accounts.authority.key()),
        ErrorCode::Unauthorized
    );

    let market = &mut ctx.accounts.market;
    require!(market.resolution_status == ResolutionStatus::Open, ErrorCode::MarketAlreadyResolved);

    // Set outcome
    market.resolution_status = ResolutionStatus::Resolved;
    market.outcome = outcome;

    // -----------------------------
    // Freeze settlement snapshot
    // -----------------------------
    // Snapshot pool total from vault balances at resolution time
    let usdt_balance = ctx.accounts.market_usdt_vault.amount;
    let usdc_balance = ctx.accounts.market_usdc_vault.amount;

    let pool_total = usdt_balance
        .checked_add(usdc_balance)
        .ok_or(ErrorCode::MathOverflow)?;

    // Determine total winning shares at resolution time
    let total_winning_shares: u64 = match market.outcome {
        MarketOutcome::Yes => market.total_yes_shares,
        MarketOutcome::No => market.total_no_shares,
        MarketOutcome::Option { index } => {
            let i = index as usize;
            require!(i < market.total_option_shares.len(), ErrorCode::InvalidOptionIndex);
            market.total_option_shares[i]
        }
        MarketOutcome::Cancelled => 0, // cancelled handled elsewhere (claim should error)
    };

    require!(market.outcome != MarketOutcome::Cancelled, ErrorCode::MarketCancelled);
    require!(total_winning_shares > 0, ErrorCode::NoWinningsToClaim);

    // payout_per_share = pool_total * 1e6 / total_winning_shares
    let payout_per_share = (pool_total as u128)
        .checked_mul(PRICE_SCALE as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(total_winning_shares as u128)
        .ok_or(ErrorCode::MathOverflow)? as u64;

    market.settle_total_pool = pool_total;
    market.settle_total_winning_shares = total_winning_shares;
    market.settle_payout_per_share = payout_per_share;
    market.settle_initialized = true;

    Ok(())
}    


#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump = market.bump
    )]
    pub market: Box<Account<'info, Market>>,
    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Box<Account<'info, AdminConfig>>,
    pub authority: Signer<'info>,

    // vaults to snapshot total pool at resolve time
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

    #[account(constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Box<Account<'info, Mint>>,

    #[account(constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Box<Account<'info, Mint>>,    
}
