// use anchor_lang::prelude::*;
// use anchor_lang::system_program::{transfer, Transfer};
// use crate::errors::ErrorCode;
// use crate::state::{AdminConfig, Market, MarketMethod, Position};
// use crate::constants::{USDT_MINT_PUBKEY, PRICE_SCALE, CREDIT_FEE, CREDIT_BURN_FEE, SOL_USDT_FEED};
// use crate::events::BuyCreditEvent;
// use rust_decimal::Decimal;
// use rust_decimal::prelude::ToPrimitive;
// use crate::utils::{
//     prepare_market_id_seed, ensure_position_initialized, lmsr_buy_yes_from_amount, lmsr_buy_no_from_amount, calc_fee_split
// };

// use switchboard_on_demand::{
//     SlotHashes, Instructions, default_queue, SwitchboardQuoteExt, SwitchboardQuote
// };
// use std::str::FromStr;

// pub fn buy_credit_sol(ctx: Context<BuyCreditSol>) -> Result<()> {
//     let amount_in_usd: u64 = 10_000_000;

//     let feeds = &ctx.accounts.quote_account.feeds;

//     let mut total_cost_in_sol: u64 = 0;    

//     for (i, feed) in feeds.iter().enumerate() {
//         let feed_value = feed.value(); // Decimal
//         msg!("💰 Feed {} Value: {}", i, feed_value);

//         // ✅ Convert feed price to integer form (multiply by 1e9 for precision)
//         let price_sol_in_usd_with_decimals = feed_value
//         .checked_mul(Decimal::from(1_000_000_000u64))
//         .ok_or(ErrorCode::Overflow)?
//         .to_u64()
//         .ok_or(ErrorCode::Overflow)?;

//         msg!(
//         "💰 Feed {}: Value with decimals = {}",
//         i,
//         price_sol_in_usd_with_decimals
//         );

//         // ✅ Compute total SOL cost in lamports using Decimal for safety
//         let total_cost_decimal = Decimal::from(price_sol_in_usd_with_decimals)
//         .checked_mul(Decimal::from(amount_in_usd))
//         .ok_or(ErrorCode::Overflow)?
//         .checked_div(Decimal::from(1_000_000u64)) // normalize
//         .ok_or(ErrorCode::Overflow)?;

//         total_cost_in_sol = total_cost_decimal
//         .to_u64()
//         .ok_or(ErrorCode::Overflow)?;

//         msg!(
//         "Calculated total cost for {} keys: {} lamports",
//         amount_in_usd,
//         total_cost_in_sol
//         );

//         break;
//     }               

//     let fee = total_cost_in_sol.checked_mul(CREDIT_FEE).unwrap() / PRICE_SCALE;
//     let burn_fee = total_cost_in_sol.checked_mul(CREDIT_BURN_FEE).unwrap() / PRICE_SCALE;
//     let amount_after_fee = total_cost_in_sol.checked_sub(fee).unwrap().checked_sub(burn_fee).unwrap();

//     transfer(
//         CpiContext::new(
//             ctx.accounts.system_program.to_account_info(),
//             Transfer {
//                 from: ctx.accounts.user.to_account_info(),
//                 to: ctx.accounts.strike_reserve.to_account_info(),
//             },
//         ),
//         amount_after_fee,
//     )?;

//     transfer(
//         CpiContext::new(
//             ctx.accounts.system_program.to_account_info(),
//             Transfer {
//                 from: ctx.accounts.user.to_account_info(),
//                 to: ctx.accounts.fee_recipient.to_account_info(),
//             },
//         ),
//         fee,
//     )?;

//     // // Burn the credit fee
//     // burn(
//     //     CpiContext::new(
//     //         ctx.accounts.token_program.to_account_info(),
//     //         Burn {
//     //             from: ctx.accounts.fee_recipient.to_account_info(),
//     //         },
//     //     ),
//     //     burn_fee,
//     // )?;

//     msg!("Transferred {} USDT from user to strike reserve", amount_in_usd);

//     emit!(BuyCreditEvent {
//         option: 2,
//         user: ctx.accounts.user.key(),
//         amount_in: amount_in_usd,
//         amount_in_usd,
//         fee,
//         amount_after_fee,
//     });

//     Ok(())
// }

// #[derive(Accounts)]
// pub struct BuyCreditSol<'info> {
//     #[account(mut)]
//     pub user: Signer<'info>,

//     #[account(
//         mut,
//         seeds = [b"strike_reserve"],
//         bump
//     )]
//     pub strike_reserve: SystemAccount<'info>,

//     #[account(
//         mut,
//         constraint = fee_recipient.key() == admin_config.fee_recipient @ ErrorCode::InvalidFeeRecipient
//     )]
//     pub fee_recipient: SystemAccount<'info>,
    
//     #[account(mut)]
//     pub admin_config: Account<'info, AdminConfig>,    

//     #[account(address = Pubkey::from_str(SOL_USDT_FEED).unwrap())]
//     pub quote_account: Box<Account<'info, SwitchboardQuote>>,    

//     pub rent: Sysvar<'info, Rent>,
//     pub system_program: Program<'info, System>,

//     /// System variables required for quote verification
//     pub sysvars: Sysvars<'info>,              
// }

// #[derive(Accounts)]
// pub struct Sysvars<'info> {
//     pub clock: Sysvar<'info, Clock>,
//     pub slothashes: Sysvar<'info, SlotHashes>,
//     pub instructions: Sysvar<'info, Instructions>,
// }

