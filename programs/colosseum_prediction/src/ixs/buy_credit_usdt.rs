use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, transfer, Transfer};

use crate::errors::ErrorCode;
use crate::state::{AdminConfig, Market, MarketMethod, Position};
use crate::constants::{USDT_MINT_PUBKEY, PRICE_SCALE, CREDIT_FEE, CREDIT_BURN_FEE};
use crate::events::BuyCreditEvent;
use crate::utils::{
    prepare_market_id_seed, ensure_position_initialized, lmsr_buy_yes_from_amount, lmsr_buy_no_from_amount, calc_fee_split
};

pub fn buy_credit_usdt(ctx: Context<BuyCreditUsdt>) -> Result<()> {
    let amount_in_usd: u64 = 10_000_000;

    let fee = amount_in_usd.checked_mul(CREDIT_FEE).unwrap() / PRICE_SCALE;
    let burn_fee = amount_in_usd.checked_mul(CREDIT_BURN_FEE).unwrap() / PRICE_SCALE;
    let amount_after_fee = amount_in_usd.checked_sub(fee).unwrap().checked_sub(burn_fee).unwrap();

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.strike_reserve_usdt_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_after_fee,
    )?;

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.fee_recipient_usdt_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        fee,
    )?;

    // // Burn the credit fee
    // burn(
    //     CpiContext::new(
    //         ctx.accounts.token_program.to_account_info(),
    //         Burn {
    //             from: ctx.accounts.fee_recipient_usdt_account.to_account_info(),
    //         },
    //     ),
    //     burn_fee,
    // )?;

    msg!("Transferred {} USDT from user to strike reserve", amount_in_usd);

    emit!(BuyCreditEvent {
        option: 0,
        user: ctx.accounts.user.key(),
        amount_in: amount_in_usd,
        amount_in_usd,
        fee,
        amount_after_fee,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct BuyCreditUsdt<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"strike_reserve"],
        bump
    )]
    pub strike_reserve: SystemAccount<'info>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = strike_reserve
    )]
    pub strike_reserve_usdt_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_usdt_account: Account<'info, TokenAccount>,

    #[account(constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Account<'info, Mint>,

    #[account(mut)]
    pub admin_config: Account<'info, AdminConfig>,    

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

