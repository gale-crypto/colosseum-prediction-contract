use anchor_lang::prelude::*;

use crate::errors::ErrorCode;
use crate::state::{AdminConfig, Market, ResolutionStatus};
use crate::utils::prepare_market_id_seed;

pub fn set_market_trading_paused(ctx: Context<SetMarketTradingPaused>, paused: bool) -> Result<()> {
    require!(
        ctx.accounts
            .admin_config
            .admins
            .contains(&ctx.accounts.authority.key()),
        ErrorCode::Unauthorized
    );

    let market = &mut ctx.accounts.market;
    require!(
        market.resolution_status == ResolutionStatus::Open,
        ErrorCode::MarketAlreadyResolved
    );

    market.trading_paused = paused;
    Ok(())
}

#[derive(Accounts)]
pub struct SetMarketTradingPaused<'info> {
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
}
