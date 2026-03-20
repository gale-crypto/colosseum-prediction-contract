use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

// =======================
// State
// =======================

#[account]
pub struct Market {
    pub market_id: String,
    pub market_method: MarketMethod,

    // Binary (scaled 1e6)
    pub yes_price: u64,
    pub no_price: u64,

    // Binary: we store q_yes/q_no here (shares outstanding, scaled 1e6)
    pub yes_volume: u64,
    pub no_volume: u64,

    // LMSR b
    pub virtual_liquidity: u64,

    // Real traded collateral (after-fee) bookkeeping (optional)
    pub total_volume: u64,

    // Multi-choice
    pub options: Vec<String>,
    pub option_prices: Vec<u64>,  // spot prices (scaled 1e6)
    pub option_volumes: Vec<u64>, // q_i (shares outstanding, scaled 1e6)

    // Totals for pro-rata claim
    pub total_yes_shares: u64,
    pub total_no_shares: u64,
    pub total_option_shares: Vec<u64>,

    // Resolution
    pub resolution_status: ResolutionStatus,
    pub outcome: MarketOutcome,

    pub creator: Pubkey,
    pub creation_fee_paid: bool,
    pub bump: u8,

    // -----------------------------
    // Settlement snapshot (freeze at resolve time)
    // -----------------------------
    pub settle_total_pool: u64,          // snapshot: usdt + usdc vault balances at resolution
    pub settle_total_winning_shares: u64, // snapshot: total winning shares at resolution
    pub settle_payout_per_share: u64,    // settle_total_pool * PRICE_SCALE / settle_total_winning_shares
    pub settle_initialized: bool,        // true once snapshot is taken    
}

impl Market {
    pub const MAX_OPTIONS: usize = 10;
    pub const MAX_OPTION_LENGTH: usize = 100;
    pub const MAX_MARKET_ID: usize = 64;

    pub const LEN: usize =
        4 + Self::MAX_MARKET_ID + // market_id String
        1 + // market_method

        8 + 8 + // yes_price, no_price
        8 + 8 + // yes_volume, no_volume
        8 + // virtual_liquidity
        8 + // total_volume

        4 + (Self::MAX_OPTIONS * (4 + Self::MAX_OPTION_LENGTH)) + // options
        4 + (Self::MAX_OPTIONS * 8) + // option_prices
        4 + (Self::MAX_OPTIONS * 8) + // option_volumes

        8 + 8 + // total_yes_shares, total_no_shares
        4 + (Self::MAX_OPTIONS * 8) + // total_option_shares

        1 + // resolution_status
        1 + 1 + // outcome
        32 + // creator
        1 + // creation_fee_paid
        1 + // bump

        // settlement snapshot fields
        8 + // settle_total_pool
        8 + // settle_total_winning_shares
        8 + // settle_payout_per_share
        1; // settle_initialized        
}

#[account]
pub struct AdminConfig {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub admins: Vec<Pubkey>,
    pub bump: u8,
}

impl AdminConfig {
    pub const MAX_ADMINS: usize = 100;
    pub const LEN: usize =
        32 + // authority
        32 + // fee_recipient
        4 + (32 * Self::MAX_ADMINS) + // admins
        1; // bump
}

#[account]
pub struct Position {
    pub user: Pubkey,
    pub market_id: String,

    pub referrer: Pubkey,

    pub yes_shares: u64,
    pub no_shares: u64,
    pub option_shares: Vec<u64>,

    // NEW: cost basis in "collateral units" (same 1e6 scale as amounts)
    pub yes_cost: u64,
    pub no_cost: u64,
    pub option_costs: Vec<u64>,

    // NEW: realized pnl in collateral units (can be negative)
    pub realized_pnl: i64,

    // Existing
    pub total_deposited_usdt: u64,
    pub total_deposited_usdc: u64,

    // Optional bookkeeping
    pub fees_paid: u64,
    pub total_withdrawn_usdt: u64,
    pub total_withdrawn_usdc: u64,

    pub bump: u8,
}

impl Position {
    pub const MAX_MARKET_ID: usize = 64;
    pub const MAX_OPTIONS: usize = 10;

    pub const LEN: usize =
        32 + // user
        4 + Self::MAX_MARKET_ID + // market_id
        32 + // referrer
        8 + 8 + // yes_shares, no_shares
        4 + (Self::MAX_OPTIONS * 8) + // option_shares
        8 + 8 + // yes_cost, no_cost
        4 + (Self::MAX_OPTIONS * 8) + // option_costs
        8 + // realized_pnl
        8 + 8 + // total_deposited_usdt, total_deposited_usdc
        8 + // fees_paid
        8 + 8 + // total_withdrawn_usdt, total_withdrawn_usdc
        1; // bump
}

#[account]
pub struct UserInfo {
    pub user: Pubkey,
    pub referrer: Pubkey,
    pub total_referred_fees: u64,
    pub bump: u8,
}

impl UserInfo {
    pub const LEN: usize =
        32 + // user
        32 + // referrer
        8 + // total_referred_fees
        1; // bump
}

// =======================
// Enums
// =======================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResolutionStatus {
    Open,
    Resolved,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketMethod {
    Binary,
    MultiChoice,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketOutcome {
    Yes,
    No,
    Option { index: u8 },
    Cancelled,
}

// // Helper account bundle for Raydium swap (keep it tidy)
// #[derive(Clone)]
// pub struct RaydiumSwapAccounts<'info> {
//     pub amm_program: AccountInfo<'info>,
//     pub amm: AccountInfo<'info>,
//     pub amm_authority: AccountInfo<'info>,
//     pub amm_open_orders: AccountInfo<'info>,
//     pub amm_coin_vault: AccountInfo<'info>,
//     pub amm_pc_vault: AccountInfo<'info>,
//     pub market_program: AccountInfo<'info>,
//     pub market: AccountInfo<'info>,
//     pub market_bids: AccountInfo<'info>,
//     pub market_asks: AccountInfo<'info>,
//     pub market_event_queue: AccountInfo<'info>,
//     pub market_coin_vault: AccountInfo<'info>,
//     pub market_pc_vault: AccountInfo<'info>,
//     pub market_vault_signer: AccountInfo<'info>,
//     pub user_token_source: Account<'info, TokenAccount>,
// }