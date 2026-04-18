use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod ixs;
pub mod state;
pub mod utils;
pub mod events;
/// Embeds `security.txt` metadata in the on-chain `.so` (see `security_contact.rs`).
mod security_contact;

use ixs::*;

use crate::state::{MarketMethod, MarketOutcome};

declare_program!(jupiter_aggregator);
declare_id!("8q3gx2TZ26ne8sETacwW8p7JLMeF4p4K7yEx9VGMRUiH");

#[program]
pub mod colosseum_prediction {
    use super::*;

    pub fn initialize_admin_config(ctx: Context<InitializeAdminConfig>) -> Result<()> {
        return ixs::initialize_admin_config::initialize_admin_config(ctx);
    }

    pub fn add_admin(ctx: Context<ManageAdmin>, admin_address: Pubkey) -> Result<()> {
        return ixs::control_admin::add_admin(ctx, admin_address);
    }

    pub fn remove_admin(ctx: Context<ManageAdmin>, admin_address: Pubkey) -> Result<()> {
        return ixs::control_admin::remove_admin(ctx, admin_address);
    }

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        market_id: String,
        market_method: MarketMethod,
        initial_yes_price: u64,
        initial_no_price: u64,
        options: Vec<String>,
        initial_option_prices: Vec<u64>,
    ) -> Result<()> {
        return ixs::initialize_market::initialize_market(ctx, market_id, market_method, initial_yes_price, initial_no_price, options, initial_option_prices);
    }

    // -----------------------
    // BUY (Binary)
    // -----------------------

    pub fn buy_yes_usdt(ctx: Context<BuySharesWithUSDT>, amount: u64) -> Result<()> {
        return ixs::buy_binary_usdt::buy_yes_usdt(ctx, amount);
    }

    pub fn buy_yes_usdc(ctx: Context<BuySharesWithUSDC>, amount: u64) -> Result<()> {
        return ixs::buy_binary_usdc::buy_yes_usdc(ctx, amount);
    }

    pub fn buy_no_usdt(ctx: Context<BuySharesWithUSDT>, amount: u64) -> Result<()> {
        return ixs::buy_binary_usdt::buy_no_usdt(ctx, amount);
    }

    pub fn buy_no_usdc(ctx: Context<BuySharesWithUSDC>, amount: u64) -> Result<()> {
        return ixs::buy_binary_usdc::buy_no_usdc(ctx, amount);
    }

    // pub fn buy_km_with_usdt(
    //     ctx: Context<ProxySwapBaseIn>,
    //     amount_in: u64
    // ) -> Result<()> {
    //     return ixs::buy_binary_usdt::buy_km_with_usdt(ctx, amount_in);
    // }
    
    // -----------------------
    // SELL (Binary)
    // -----------------------

    pub fn sell_yes(ctx: Context<SellShares>, shares: u64) -> Result<()> {
        return ixs::sell_binary::sell_yes(ctx, shares);
    }

    pub fn sell_no(ctx: Context<SellShares>, shares: u64) -> Result<()> {
        return ixs::sell_binary::sell_no(ctx, shares);
    }

    // -----------------------
    // Resolve + Claim (kept same)
    // -----------------------

    pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: MarketOutcome) -> Result<()> {
        return ixs::resolve_market::resolve_market(ctx, outcome);
    }    

    pub fn claim_winnings_yesno(ctx: Context<ClaimWinnings>) -> Result<()> {
      return ixs::claim_winnings_yesno::claim_winnings_yesno(ctx);
    }

    // -----------------------
    // Multi-choice trades (UPDATED to LMSR)
    // -----------------------

    pub fn buy_option_usdt(ctx: Context<BuyOptionUSDT>, option_index: u8, amount: u64) -> Result<()> {
        return ixs::buy_option_usdt::buy_option_usdt(ctx, option_index, amount);
    }

    pub fn buy_option_usdc(ctx: Context<BuyOptionUSDC>, option_index: u8, amount: u64) -> Result<()> {
        return ixs::buy_option_usdc::buy_option_usdc(ctx, option_index, amount);
    }

    pub fn sell_option(ctx: Context<SellOption>, option_index: u8, shares: u64) -> Result<()> {
        return ixs::sell_option::sell_option(ctx, option_index, shares);
    }

    pub fn simulate_buy_binary(
        ctx: Context<SimulateMarketReadOnly>,
        side_yes: bool,
        amount: u64,
    ) -> Result<()> {
        return ixs::simulate_market_read_only::simulate_buy_binary(ctx, side_yes, amount);
    }
    
    pub fn simulate_sell_binary(
        ctx: Context<SimulateMarketReadOnly>,
        side_yes: bool,
        shares: u64,
    ) -> Result<()> {
        return ixs::simulate_market_read_only::simulate_sell_binary(ctx, side_yes, shares);
    }
    
    pub fn simulate_buy_option(
        ctx: Context<SimulateMarketReadOnly>,
        option_index: u8,
        amount: u64,
    ) -> Result<()> {
        return ixs::simulate_market_read_only::simulate_buy_option(ctx, option_index, amount);
    }
    
    pub fn simulate_sell_option(
        ctx: Context<SimulateMarketReadOnly>,
        option_index: u8,
        shares: u64,
    ) -> Result<()> {
        return ixs::simulate_market_read_only::simulate_sell_option(ctx, option_index, shares);
    }    

    // Credit trades
    // pub fn buy_credit_usdt(ctx: Context<BuyCreditUsdt>) -> Result<()> {
    //     return ixs::buy_credit_usdt::buy_credit_usdt(ctx);
    // }

    pub fn buy_credit_usdc(ctx: Context<BuyCreditUsdc>, data: Vec<u8>) -> Result<()> {
        return ixs::buy_credit_usdc::buy_credit_usdc(ctx, data);
    }

    pub fn distribute_strike_reward(ctx: Context<DistributeStrikeRewards>, total_usdc_amount: u64, total_burn_amount: u64/*, data: Vec<u8>*/) -> Result<()> {
        return ixs::distribute_strike_rewards::distribute_strike_rewards(ctx, total_usdc_amount, total_burn_amount);
    }

    // pub fn buy_credit_sol(ctx: Context<BuyCreditSol>) -> Result<()> {
    //     return ixs::buy_credit_sol::buy_credit_sol(ctx);
    // }
}

