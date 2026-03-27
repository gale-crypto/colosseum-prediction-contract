use anchor_lang::prelude::*;

use crate::state::MarketOutcome;

#[event]
pub struct BuyBinaryEvent {
    pub market: Pubkey,
    pub payer: Pubkey,
    pub is_usdt: bool,
    pub side_yes: bool,
    pub amount_in: u64,
    pub fee: u64,
    pub amount_after_fee: u64,
    pub shares_out: u64,
    pub yes_price_after: u64,
    pub no_price_after: u64,
    pub avg_price: u64,
    pub real_price: u64,
}

#[event]
pub struct SellBinaryEvent {
    pub market: Pubkey,
    pub payer: Pubkey,
    pub side_yes: bool,
    pub shares_in: u64,
    pub payout_before_fee: u64,
    pub fee: u64,
    pub payout_after_fee: u64,
    pub yes_price_after: u64,
    pub no_price_after: u64,    
    pub avg_price: u64,
    pub real_price: u64,
    pub pay_usdt: u64,
    pub pay_usdc: u64,
}

#[event]
pub struct ClaimWinningsEvent {
    pub market: Pubkey,
    pub payer: Pubkey,
    pub payout_before_fee: u64,
    pub fee: u64,
    pub payout_after_fee: u64,
    pub pay_usdt: u64,
    pub pay_usdc: u64,
    pub outcome: MarketOutcome,
}

#[event]
pub struct BuyOptionEvent {
    pub market: Pubkey,
    pub payer: Pubkey,
    pub is_usdt: bool,
    pub option_index: u8,
    pub amount_in: u64,
    pub fee: u64,
    pub amount_after_fee: u64,
    pub shares_out: u64,
    pub option_prices_after: Vec<u64>,
    pub avg_price: u64,
    pub real_price: u64,
}

#[event]
pub struct SellOptionEvent {
    pub market: Pubkey,
    pub payer: Pubkey,
    pub option_index: u8,
    pub shares_in: u64,
    pub payout_before_fee: u64,
    pub fee: u64,
    pub payout_after_fee: u64,
    pub option_prices_after: Vec<u64>,
    pub avg_price: u64,
    pub real_price: u64,
    pub pay_usdt: u64,
    pub pay_usdc: u64,
}


#[event]
pub struct BuyCreditEvent {
    pub user: Pubkey,
    // pub option: u64,
    pub amount_in: u64
}

#[event]
pub struct DistributeStrikeRewardsEvent {
    pub distributed_by: Pubkey,
    pub total_usdc_amount: u64,
    pub winner_1: Pubkey,
    pub winner_2: Pubkey,
    pub winner_3: Pubkey,
    pub burn_amount: u64,
    pub winner_1_amount: u64,
    pub winner_2_amount: u64,
    pub winner_3_amount: u64,
}