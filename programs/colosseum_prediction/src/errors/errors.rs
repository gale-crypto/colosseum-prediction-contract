use anchor_lang::prelude::*;

// =======================
// Errors
// =======================

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid payment token")]
    InvalidPaymentToken,
    #[msg("Invalid token account provided")]
    InvalidTokenAccount,
    #[msg("Invalid vault account provided")]
    InvalidVaultAccount,
    #[msg("Invalid market account provided")]
    InvalidMarketAccount,
    #[msg("Invalid position account provided")]
    InvalidPositionAccount,
    #[msg("Insufficient shares")]
    InsufficientShares,
    #[msg("Market already resolved")]
    MarketAlreadyResolved,
    #[msg("Market not resolved")]
    MarketNotResolved,
    #[msg("Market was cancelled")]
    MarketCancelled,
    #[msg("No winnings to claim")]
    NoWinningsToClaim,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Admin already exists")]
    AdminAlreadyExists,
    #[msg("Insufficient funds for market creation fee")]
    InsufficientFunds,
    #[msg("Invalid market method")]
    InvalidMarketMethod,
    #[msg("Invalid option index")]
    InvalidOptionIndex,
    #[msg("Invalid number of options")]
    InvalidOptionsCount,
    #[msg("Invalid option prices")]
    InvalidOptionPrices,
    #[msg("Invalid mint address")]
    InvalidMintAddress,
    #[msg("Insufficient Market Liquidity")]
    InsufficientMarketLiquidity,
    #[msg("Settlement snapshot not initialized")]
    SettlementNotInitialized,
    #[msg("Invalid instruction data: expected 16 bytes (amount_in u64, min_amount_out u64)")]
    InvalidInstructionData,
    #[msg("Invalid fee recipient")]
    InvalidFeeRecipient,
    #[msg("Overflow")]
    Overflow,
}
