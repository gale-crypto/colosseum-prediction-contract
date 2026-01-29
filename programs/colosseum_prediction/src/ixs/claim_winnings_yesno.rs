use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::state::{Market, MarketOutcome, Position, ResolutionStatus, AdminConfig};
use crate::constants::{USDT_MINT_PUBKEY, USDC_MINT_PUBKEY, PRICE_SCALE};
use crate::events::ClaimWinningsEvent;
use crate::utils::{prepare_market_id_seed, calc_fee, split_payout};

pub fn claim_winnings_yesno(ctx: Context<ClaimWinnings>) -> Result<()> {
    let market = &ctx.accounts.market;
    require!(
        market.resolution_status == ResolutionStatus::Resolved,
        ErrorCode::MarketNotResolved
    );

    let position = &mut ctx.accounts.position;

    let (user_winning_shares, total_winning_shares) = match market.outcome.clone() {
        MarketOutcome::Yes => (position.yes_shares, market.total_yes_shares),
        MarketOutcome::No => (position.no_shares, market.total_no_shares),
        MarketOutcome::Option { index } => {
            let i = index as usize;
            (
                position.option_shares.get(i).copied().unwrap_or(0),
                market.total_option_shares.get(i).copied().unwrap_or(0),
            )
        }
        MarketOutcome::Cancelled => return Err(ErrorCode::MarketCancelled.into()),
    };

    require!(user_winning_shares > 0, ErrorCode::NoWinningsToClaim);
    require!(total_winning_shares > 0, ErrorCode::NoWinningsToClaim);

    require!(market.settle_initialized, ErrorCode::SettlementNotInitialized);

    let usdt_balance = ctx.accounts.market_usdt_vault.amount;
    let usdc_balance = ctx.accounts.market_usdc_vault.amount;

    // payout_before_fee = user_winning_shares * settle_payout_per_share / 1e6
    let payout_before_fee = (user_winning_shares as u128)
        .checked_mul(market.settle_payout_per_share as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRICE_SCALE as u128)
        .ok_or(ErrorCode::MathOverflow)? as u64;

    require!(payout_before_fee > 0, ErrorCode::NoWinningsToClaim);

    // IMPORTANT:
    // We still must ensure current vault balances can cover this payout (they should, unless something drained them)
    let total_balance = usdt_balance
        .checked_add(usdc_balance)
        .ok_or(ErrorCode::MathOverflow)?;
    require!(total_balance >= payout_before_fee, ErrorCode::InsufficientMarketLiquidity);    

    require!(payout_before_fee > 0, ErrorCode::NoWinningsToClaim);

    let (fee_amount, payout_after_fee) = calc_fee(payout_before_fee)?;

    // -----------------------------
    // REALIZE P/L (settlement)
    // -----------------------------
    let mut cost_removed: u64 = 0;

    match market.outcome.clone() {
        MarketOutcome::Yes => {
            cost_removed = position.yes_cost;
            position.yes_cost = 0;
        }
        MarketOutcome::No => {
            cost_removed = position.no_cost;
            position.no_cost = 0;
        }
        MarketOutcome::Option { index } => {
            let i = index as usize;
            if position.option_costs.len() > i {
                cost_removed = position.option_costs[i];
                position.option_costs[i] = 0;
            }
        }
        MarketOutcome::Cancelled => {}
    }

    position.realized_pnl = position.realized_pnl
        .checked_add(payout_after_fee as i64)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_sub(cost_removed as i64)
        .ok_or(ErrorCode::MathOverflow)?;

    position.fees_paid = position.fees_paid
        .checked_add(fee_amount)
        .ok_or(ErrorCode::MathOverflow)?;

    // -----------------------------
    // Token payout split
    // -----------------------------
    let market_id_seed = prepare_market_id_seed(&market.market_id);
    let signer_seeds: &[&[&[u8]]] = &[&[b"market", &market_id_seed, &[market.bump]]];

    let (fee_usdt, fee_usdc, pay_usdt, pay_usdc) =
        split_payout(
            payout_before_fee,
            payout_after_fee,
            fee_amount,
            usdt_balance,
            usdc_balance,
        )?;

    // -----------------------------
    // Fee transfers
    // -----------------------------
    if fee_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_usdt_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer_seeds,
            ),
            fee_usdt,
        )?;
    }

    if fee_usdc > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdc_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_usdc_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer_seeds,
            ),
            fee_usdc,
        )?;
    }

    // -----------------------------
    // User payout transfers
    // -----------------------------
    if pay_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.user_usdt_token_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer_seeds,
            ),
            pay_usdt,
        )?;
    }

    if pay_usdc > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdc_vault.to_account_info(),
                    to: ctx.accounts.user_usdc_token_account.to_account_info(),
                    authority: ctx.accounts.market.to_account_info(),
                },
                signer_seeds,
            ),
            pay_usdc,
        )?;
    }

    position.total_withdrawn_usdt = position.total_withdrawn_usdt
        .checked_add(pay_usdt)
        .ok_or(ErrorCode::MathOverflow)?;

    position.total_withdrawn_usdc = position.total_withdrawn_usdc
        .checked_add(pay_usdc)
        .ok_or(ErrorCode::MathOverflow)?;

    // -----------------------------
    // Clear shares
    // -----------------------------
    match market.outcome.clone() {
        MarketOutcome::Yes => position.yes_shares = 0,
        MarketOutcome::No => position.no_shares = 0,
        MarketOutcome::Option { index } => {
            let i = index as usize;
            if position.option_shares.len() > i {
                position.option_shares[i] = 0;
            }
        }
        MarketOutcome::Cancelled => {}
    }

    emit!(ClaimWinningsEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        payout_before_fee,
        fee: fee_amount,
        payout_after_fee,
        pay_usdt,
        pay_usdc,
        outcome: market.outcome.clone(),
    });
    
    Ok(())
}


#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        mut,
        seeds = [b"position", user.key().as_ref(), &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Box<Account<'info, AdminConfig>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdt_mint,
        associated_token::authority = user
    )]
    pub user_usdt_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdc_mint,
        associated_token::authority = user
    )]
    pub user_usdc_token_account: Box<Account<'info, TokenAccount>>,

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

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_usdt_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_usdc_account: Box<Account<'info, TokenAccount>>,

    #[account(constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Box<Account<'info, Mint>>,

    #[account(constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
