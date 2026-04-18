use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::state::{Market, MarketMethod, Position, AdminConfig};
use crate::constants::{USDC_MINT_PUBKEY, PRICE_SCALE};
use crate::events::BuyOptionEvent;
use crate::utils::{prepare_market_id_seed, ensure_position_initialized, lmsr_buy_option_from_amount, calc_fee_split};

pub fn buy_option_usdc(ctx: Context<BuyOptionUSDC>, option_index: u8, amount: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::MultiChoice, ErrorCode::InvalidMarketMethod);

    let idx = option_index as usize;
    require!(idx < market.options.len(), ErrorCode::InvalidOptionIndex);

    let (fee_total, amount_after_fee, fee_buyback, fee_referral, fee_treasury) = calc_fee_split(amount)?;

    if fee_treasury > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.fee_recipient_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_treasury + fee_buyback,
        )?;
    }

    let position = &mut ctx.accounts.position;    

    let use_referrer = (position.referrer != Pubkey::default() && position.referrer.key() == ctx.accounts.referrer.key()) || (position.referrer == Pubkey::default());
    let referrer = if position.user == Pubkey::default() {
        ctx.accounts.referrer.key() 
     } else { 
        Pubkey::default()
     };

    if fee_referral > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.referrer_usdc_ata.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_referral,
        )?;
    }

    let b = market.virtual_liquidity;
    let (shares_out, new_prices) = lmsr_buy_option_from_amount(amount_after_fee, &market.option_volumes, idx, b)?;

    let position = &mut ctx.accounts.position;
    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, ctx.bumps.position, market, referrer);

    require!(position.option_shares.len() == market.options.len(), ErrorCode::InvalidPositionAccount);
    position.option_shares[idx] = position.option_shares[idx].checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    position.total_deposited_usdc = position.total_deposited_usdc.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    position.option_costs[idx] = position.option_costs[idx]
    .checked_add(amount_after_fee)
    .ok_or(ErrorCode::MathOverflow)?;
    position.fees_paid = position.fees_paid
    .checked_add(fee_total)
    .ok_or(ErrorCode::MathOverflow)?;         

    market.total_option_shares[idx] = market.total_option_shares[idx].checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.option_volumes[idx] = market.option_volumes[idx].checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    
    market.option_prices = new_prices;

    let avg_price = if shares_out > 0 {
        (amount_after_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares_out as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    let real_price = if shares_out > 0 {
        (amount as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares_out as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };        
    
    market.total_volume = market.total_volume.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    emit!(BuyOptionEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        is_usdt: false, // set false in USDC variant
        option_index,
        amount_in: amount,
        fee: fee_total,
        amount_after_fee,
        shares_out,
        option_prices_after: market.option_prices.clone(), // already updated to new_prices
        avg_price,
        real_price,
    });
    Ok(())
}

#[derive(Accounts)]
#[instruction(option_index: u8, amount: u64)]
pub struct BuyOptionUSDC<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Position::space_for_option_count(
            if market.market_method == MarketMethod::Binary {
                0
            } else {
                market.options.len()
            }
        ),
        seeds = [b"position", user.key().as_ref(), &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Box<Account<'info, AdminConfig>>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = market
    )]
    pub market_vault: Account<'info, TokenAccount>,

    pub referrer: SystemAccount<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = referrer
    )]
    pub referrer_usdc_ata: Box<Account<'info, TokenAccount>>,      

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_token_account: Account<'info, TokenAccount>,

    #[account(constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
