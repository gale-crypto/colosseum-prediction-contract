use anchor_lang::prelude::*;

use crate::errors::ErrorCode;
use crate::events::{BuyBinaryEvent, SellBinaryEvent, BuyOptionEvent, SellOptionEvent};
use crate::state::{Market, MarketMethod};
use crate::constants::{PRICE_SCALE};
use crate::utils::{
    lmsr_buy_yes_from_amount, lmsr_buy_no_from_amount,
    lmsr_sell_yes_to_amount, lmsr_sell_no_to_amount,
    lmsr_buy_option_from_amount, lmsr_sell_option_to_amount,
    calc_fee, require_tradeable_market,
};

pub fn simulate_buy_binary(
    ctx: Context<SimulateMarketReadOnly>,
    side_yes: bool,
    amount: u64,
) -> Result<()> {
    require_tradeable_market(&ctx.accounts.market)?;
    let market = &ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let (fee_amount, amount_after_fee) = calc_fee(amount)?;

    let b = market.virtual_liquidity;

    let (shares_out, new_yes_price, new_no_price) = if side_yes {
        lmsr_buy_yes_from_amount(amount_after_fee, market.yes_volume, market.no_volume, b)?
    } else {
        lmsr_buy_no_from_amount(amount_after_fee, market.yes_volume, market.no_volume, b)?
    };

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

    // Emit SAME event as real buy (payer = caller, is_usdt is unknown in simulation -> choose false OR add param)
    emit!(BuyBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.caller.key(),
        is_usdt: false, // NOTE: if you want exact, add param is_usdt to simulate fn
        side_yes,
        amount_in: amount,
        fee: fee_amount,
        amount_after_fee,
        shares_out,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
    });

    Ok(())
}

pub fn simulate_sell_binary(
    ctx: Context<SimulateMarketReadOnly>,
    side_yes: bool,
    shares: u64,
) -> Result<()> {
    require_tradeable_market(&ctx.accounts.market)?;
    let market = &ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let b = market.virtual_liquidity;

    // NOTE: This assumes "shares" are in the SAME SCALE as your stored q values.
    // In your code, shares_out is produced in 1e6-scaled units and stored into yes_volume/no_volume.
    // So simulation should be passed that same unit.
    let (payout_before_fee, new_yes_price, new_no_price) = if side_yes {
        lmsr_sell_yes_to_amount(shares, market.yes_volume, market.no_volume, b)?
    } else {
        lmsr_sell_no_to_amount(shares, market.yes_volume, market.no_volume, b)?
    };

    let (fee_amount, payout_after_fee) = calc_fee(payout_before_fee)?;

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
        payer: ctx.accounts.caller.key(),
        side_yes,
        shares_in: shares,
        payout_before_fee,
        fee: fee_amount,
        payout_after_fee,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
        pay_usdt: 0, // simulation has no vault split
        pay_usdc: 0,
    });

    Ok(())
}

pub fn simulate_buy_option(
    ctx: Context<SimulateMarketReadOnly>,
    option_index: u8,
    amount: u64,
) -> Result<()> {
    require_tradeable_market(&ctx.accounts.market)?;
    let market = &ctx.accounts.market;
    require!(market.market_method == MarketMethod::MultiChoice, ErrorCode::InvalidMarketMethod);

    let idx = option_index as usize;
    require!(idx < market.options.len(), ErrorCode::InvalidOptionIndex);

    let (fee_amount, amount_after_fee) = calc_fee(amount)?;

    let b = market.virtual_liquidity;
    let (shares_out, new_prices) =
        lmsr_buy_option_from_amount(amount_after_fee, &market.option_volumes, idx, b)?;

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

    emit!(BuyOptionEvent {
        market: market.key(),
        payer: ctx.accounts.caller.key(),
        is_usdt: false, // NOTE: add param if you want exact
        option_index,
        amount_in: amount,
        fee: fee_amount,
        amount_after_fee,
        shares_out,
        option_prices_after: new_prices,
        avg_price,
        real_price,
    });

    Ok(())
}

pub fn simulate_sell_option(
    ctx: Context<SimulateMarketReadOnly>,
    option_index: u8,
    shares: u64,
) -> Result<()> {
    require_tradeable_market(&ctx.accounts.market)?;
    let market = &ctx.accounts.market;
    require!(market.market_method == MarketMethod::MultiChoice, ErrorCode::InvalidMarketMethod);

    let idx = option_index as usize;
    require!(idx < market.options.len(), ErrorCode::InvalidOptionIndex);

    let b = market.virtual_liquidity;
    let (payout_before_fee, new_prices) =
        lmsr_sell_option_to_amount(shares, &market.option_volumes, idx, b)?;

    let (fee_amount, payout_after_fee) = calc_fee(payout_before_fee)?;

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

    emit!(SellOptionEvent {
        market: market.key(),
        payer: ctx.accounts.caller.key(),
        option_index,
        shares_in: shares,
        payout_before_fee,
        fee: fee_amount,
        payout_after_fee,
        option_prices_after: new_prices,
        avg_price,
        real_price,
        pay_usdt: 0,
        pay_usdc: 0,
    });

    Ok(())
}    


#[derive(Accounts)]
pub struct SimulateMarketReadOnly<'info> {
    pub market: Account<'info, Market>,
    pub caller: Signer<'info>,
}