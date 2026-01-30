use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::state::{AdminConfig, Market, MarketMethod, Position};
use crate::constants::{USDT_MINT_PUBKEY, USDC_MINT_PUBKEY, PRICE_SCALE};
use crate::events::SellBinaryEvent;
use crate::utils::{
    prepare_market_id_seed,
    ensure_position_initialized,
    lmsr_sell_yes_to_amount,
    lmsr_sell_no_to_amount,
    calc_fee,
    split_payout,
    avg_cost_remove,
};

pub fn sell_yes(ctx: Context<SellShares>, shares: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let position = &mut ctx.accounts.position;
    let pos_bump = position.bump;
    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, pos_bump, market, Pubkey::default());
    require!(position.yes_shares >= shares, ErrorCode::InsufficientShares);

    let b = market.virtual_liquidity;
    let (payout_before_fee, new_yes_price, new_no_price) =
        lmsr_sell_yes_to_amount(shares, market.yes_volume, market.no_volume, b)?;

    let (fee_amount, payout_after_fee) = calc_fee(payout_before_fee)?;

    let market_id_seed = prepare_market_id_seed(&market.market_id);
    let signer_seeds: &[&[&[u8]]] = &[&[b"market", &market_id_seed, &[market.bump]]];

    let usdt_balance = ctx.accounts.market_usdt_vault.amount;
    let usdc_balance = ctx.accounts.market_usdc_vault.amount;

    let (fee_usdt, fee_usdc, pay_usdt, pay_usdc) =
        split_payout(payout_before_fee, payout_after_fee, fee_amount, usdt_balance, usdc_balance)?;

    if fee_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_usdt_account.to_account_info(),
                    authority: market.to_account_info(),
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
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            fee_usdc,
        )?;
    }
    if pay_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.user_usdt_token_account.to_account_info(),
                    authority: market.to_account_info(),
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
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            pay_usdc,
        )?;
    }

    let shares_before = position.yes_shares;
    let cost_removed = avg_cost_remove(position.yes_cost, shares_before, shares)?;
    
    position.yes_cost = position.yes_cost
        .checked_sub(cost_removed)
        .ok_or(ErrorCode::MathOverflow)?;
    
    position.realized_pnl = position.realized_pnl
        .checked_add(payout_after_fee as i64)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_sub(cost_removed as i64)
        .ok_or(ErrorCode::MathOverflow)?;
    
    position.fees_paid = position.fees_paid
        .checked_add(fee_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // optional receipts
    position.total_withdrawn_usdt = position.total_withdrawn_usdt.checked_add(pay_usdt).ok_or(ErrorCode::MathOverflow)?;
    position.total_withdrawn_usdc = position.total_withdrawn_usdc.checked_add(pay_usdc).ok_or(ErrorCode::MathOverflow)?;    
        
    position.yes_shares = position.yes_shares.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    market.total_yes_shares = market.total_yes_shares.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;

    // q_yes decreases by shares (NOT by payout)
    market.yes_volume = market.yes_volume.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    market.yes_price = new_yes_price;
    market.no_price = new_no_price;

    let avg_price = if shares > 0 {
        (payout_before_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };
    
    let real_price = if shares > 0 {
        (payout_after_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    emit!(SellBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        side_yes: true,
        shares_in: shares,
        payout_before_fee,
        fee: fee_amount,
        payout_after_fee,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
        pay_usdt,
        pay_usdc,
    });
    
    Ok(())
}

pub fn sell_no(ctx: Context<SellShares>, shares: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let position = &mut ctx.accounts.position;
    let pos_bump = position.bump;
    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, pos_bump, market, Pubkey::default());
    require!(position.no_shares >= shares, ErrorCode::InsufficientShares);

    let b = market.virtual_liquidity;
    let (payout_before_fee, new_yes_price, new_no_price) =
        lmsr_sell_no_to_amount(shares, market.yes_volume, market.no_volume, b)?;

    let (fee_amount, payout_after_fee) = calc_fee(payout_before_fee)?;

    let market_id_seed = prepare_market_id_seed(&market.market_id);
    let signer_seeds: &[&[&[u8]]] = &[&[b"market", &market_id_seed, &[market.bump]]];

    let usdt_balance = ctx.accounts.market_usdt_vault.amount;
    let usdc_balance = ctx.accounts.market_usdc_vault.amount;

    let (fee_usdt, fee_usdc, pay_usdt, pay_usdc) =
        split_payout(payout_before_fee, payout_after_fee, fee_amount, usdt_balance, usdc_balance)?;

    if fee_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_usdt_account.to_account_info(),
                    authority: market.to_account_info(),
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
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            fee_usdc,
        )?;
    }
    if pay_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.user_usdt_token_account.to_account_info(),
                    authority: market.to_account_info(),
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
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            pay_usdc,
        )?;
    }

    let shares_before = position.no_shares;
    let cost_removed = avg_cost_remove(position.no_cost, shares_before, shares)?;
    
    position.no_cost = position.no_cost
        .checked_sub(cost_removed)
        .ok_or(ErrorCode::MathOverflow)?;
    
    position.realized_pnl = position.realized_pnl
        .checked_add(payout_after_fee as i64)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_sub(cost_removed as i64)
        .ok_or(ErrorCode::MathOverflow)?;
    
    position.fees_paid = position.fees_paid
        .checked_add(fee_amount)
        .ok_or(ErrorCode::MathOverflow)?;
    
    // optional receipts
    position.total_withdrawn_usdt = position.total_withdrawn_usdt.checked_add(pay_usdt).ok_or(ErrorCode::MathOverflow)?;
    position.total_withdrawn_usdc = position.total_withdrawn_usdc.checked_add(pay_usdc).ok_or(ErrorCode::MathOverflow)?;  

    position.no_shares = position.no_shares.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    market.total_no_shares = market.total_no_shares.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;

    // q_no decreases by shares
    market.no_volume = market.no_volume.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    market.yes_price = new_yes_price;
    market.no_price = new_no_price;

    let avg_price = if shares > 0 {
        (payout_before_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };
    
    let real_price = if shares > 0 {
        (payout_after_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    emit!(SellBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        side_yes: false,
        shares_in: shares,
        payout_before_fee,
        fee: fee_amount,
        payout_after_fee,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
        pay_usdt,
        pay_usdc,
    });
    
    Ok(())
}

#[derive(Accounts)]
pub struct SellShares<'info> {
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

    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Box<Account<'info, AdminConfig>>,

    #[account(mut)]
    pub user: Signer<'info>,

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
