use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::events::BuyBinaryEvent;
use crate::state::{AdminConfig, Market, MarketMethod, Position};
use crate::constants::{USDC_MINT_PUBKEY, PRICE_SCALE};
use crate::utils::{
    prepare_market_id_seed, ensure_position_initialized, lmsr_buy_yes_from_amount, lmsr_buy_no_from_amount, calc_fee_split
};

pub fn buy_yes_usdc(ctx: Context<BuySharesWithUSDC>, amount: u64) -> Result<()> {
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
            fee_treasury,
        )?;
    }

    let position = &mut ctx.accounts.position;    

    let use_referrer = (position.referrer != Pubkey::default() && position.referrer.key() == ctx.accounts.referrer.as_ref().unwrap().key()) || ctx.accounts.referrer.is_some()
    && ctx.accounts.referrer.as_ref().unwrap().key() != Pubkey::default();
    let referrer = if position.user == Pubkey::default() {
        ctx.accounts.referrer.as_ref().unwrap().key() 
     } else { 
        Pubkey::default()
     };

    if fee_referral > 0 {
        let to_account = if use_referrer {
            ctx.accounts.referrer_usdt_ata.as_ref().unwrap().to_account_info()
        } else {
            ctx.accounts.fee_recipient_token_account.to_account_info()
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: to_account,
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_referral,
        )?;
    }

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_after_fee,
    )?;

    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let b = market.virtual_liquidity;
    let (shares_out, new_yes_price, new_no_price) =
        lmsr_buy_yes_from_amount(amount_after_fee, market.yes_volume, market.no_volume, b)?;

    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, ctx.bumps.position, market, referrer);

    position.yes_shares = position.yes_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    position.total_deposited_usdc = position.total_deposited_usdc.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    position.yes_cost = position.yes_cost
    .checked_add(amount_after_fee)
    .ok_or(ErrorCode::MathOverflow)?;
    position.fees_paid = position.fees_paid
    .checked_add(fee_total)
    .ok_or(ErrorCode::MathOverflow)?;        

    market.total_yes_shares = market.total_yes_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.yes_volume = market.yes_volume.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.yes_price = new_yes_price;
    market.no_price = new_no_price;

    market.total_volume = market.total_volume.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;

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

    emit!(BuyBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        is_usdt: false,
        side_yes: true,
        amount_in: amount,
        fee: fee_total,
        amount_after_fee,
        shares_out,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
    });      
    
    Ok(())
}

pub fn buy_no_usdc(ctx: Context<BuySharesWithUSDC>, amount: u64) -> Result<()> {
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
            fee_treasury,
        )?;
    }

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_after_fee,
    )?;    

    let position = &mut ctx.accounts.position;    

    let use_referrer = (position.referrer != Pubkey::default() && position.referrer.key() == ctx.accounts.referrer.as_ref().unwrap().key()) || ctx.accounts.referrer.is_some()
    && ctx.accounts.referrer.as_ref().unwrap().key() != Pubkey::default();
    let referrer = if position.user == Pubkey::default() {
        ctx.accounts.referrer.as_ref().unwrap().key() 
     } else { 
        Pubkey::default()
     };

    if fee_referral > 0 {
        let to_account = if use_referrer {
            ctx.accounts.referrer_usdt_ata.as_ref().unwrap().to_account_info()
        } else {
            ctx.accounts.fee_recipient_token_account.to_account_info()
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: to_account,
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_referral,
        )?;
    }

    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let b = market.virtual_liquidity;
    let (shares_out, new_yes_price, new_no_price) =
        lmsr_buy_no_from_amount(amount_after_fee, market.yes_volume, market.no_volume, b)?;

    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, ctx.bumps.position, market, referrer);

    position.no_shares = position.no_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    position.total_deposited_usdc = position.total_deposited_usdc.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    position.no_cost = position.no_cost
    .checked_add(amount_after_fee)
    .ok_or(ErrorCode::MathOverflow)?;
    position.fees_paid = position.fees_paid
    .checked_add(fee_total)
    .ok_or(ErrorCode::MathOverflow)?;           

    market.total_no_shares = market.total_no_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.no_volume = market.no_volume.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.yes_price = new_yes_price;
    market.no_price = new_no_price;

    market.total_volume = market.total_volume.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;

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

    emit!(BuyBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        is_usdt: false,
        side_yes: false,
        amount_in: amount,
        fee: fee_total,
        amount_after_fee,
        shares_out,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct BuySharesWithUSDC<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Position::LEN,
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

    pub referrer: Option<SystemAccount<'info>>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = referrer
    )]
    pub referrer_usdt_ata:  Option<Box<Account<'info, TokenAccount>>>,    

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
