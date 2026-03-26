use anchor_lang::prelude::*;

#[constant]
// pub const USDT_MINT_PUBKEY: Pubkey = pubkey!("2mfQgc4tf8vzcBeMKzEYMvWwgA3zt2Zf5v2QCeyaCtT7");
// pub const USDC_MINT_PUBKEY: Pubkey = pubkey!("BRYjq2hyLJsTEZfxmDZMjrpFDvptNSRyaqgyQD9HmQ7Z");
// pub const KM_MINT_PUBKEY: Pubkey = pubkey!("DqHczfUDH6d83aSZ9eez1TrJW3sGzBpmU9HyVyjrmGFv");

pub const USDT_MINT_PUBKEY: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const USDC_MINT_PUBKEY: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const KM_MINT_PUBKEY: Pubkey = pubkey!("FThrNpdic79XRV6i9aCWQ2UTp7oRQuCXAgUWtZR2cs42");

pub const SOL_USDT_FEED: &'static str = "71ScFiunXHAM75huu8dSbLRMNsZrs87DhdTJEcny5nya";

// Market creation fee: 0.005 SOL = 5_000_000 lamports
// pub const MARKET_CREATION_FEE: u64 = 5_000_000;
pub const MARKET_CREATION_FEE: u64 = 420_000_000;

/// Raydium Legacy AMM v4 program id (MAINNET).
/// If you are on devnet, replace with the devnet Raydium v4 id.
/// (Hard-checking is important for safety.)
/// Mainnet: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
/// Devnet: DRaya7Kj3aMWQSy19kSjvmuwq9docCHofyP9kanQGaav
pub const RAYDIUM_AMM_V4: Pubkey = pubkey!("DRaya7Kj3aMWQSy19kSjvmuwq9docCHofyP9kanQGaav");

/// ---------------------------
/// FEE CONFIG (1e6 precision)
/// ---------------------------
/// 1.42% total fee split:
/// 0.42% buyback + 0.30% referral + 0.70% treasury
pub const PRICE_SCALE: u64 = 1_000_000;
pub const FEE_TOTAL: u64 = 14_200;      // 1.42%
pub const FEE_BUYBACK: u64 = 4_200;     // 0.42%
pub const FEE_REFERRAL: u64 = 3_000;    // 0.30%
pub const FEE_TREASURY: u64 = 7_000;    // 0.70%

pub const CREDIT_RESERVE_FEE: u64 = 900_000;
pub const CREDIT_TEAM_DAO_FEE: u64 = 58_000;
pub const CREDIT_BURN_FEE: u64 = 42_000;

pub const WINNER_1_BPS: u64 = 500_000; // 50%
pub const WINNER_2_BPS: u64 = 300_000; // 30%
pub const WINNER_3_BPS: u64 = 100_000; // 10%

// LMSR liquidity parameter "b" (stored in Market.virtual_liquidity)
pub const DEFAULT_VIRTUAL_LIQUIDITY: u64 = 1_000_000_000; // 1000.000000

// Bounds to avoid extreme prices (0.01 .. 0.99) for binary; multi-choice still clamped via exp safety
pub const MIN_PRICE: u64 = 10_000;
pub const MAX_PRICE: u64 = 990_000;

// exp clamp to avoid overflow in f64 exp()
pub const EXP_CLAMP: f64 = 50.0; // exp(50) ~ 3e21