use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;

use crate::errors::ErrorCode;
use crate::state::AdminConfig;
use crate::constants::{USDC_MINT_PUBKEY, KM_MINT_PUBKEY, PRICE_SCALE, CREDIT_TEAM_DAO_FEE};
use crate::events::BuyCreditEvent;
use crate::jupiter_aggregator::program::Jupiter;

use std::str::FromStr;

pub fn jupiter_program_id() -> Pubkey {
    Pubkey::from_str("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4").unwrap()
}

pub fn buy_credit_usdc(ctx: Context<BuyCreditUsdc>, data: Vec<u8>) -> Result<()> {
    let amount_in_usd: u64 = 10_000_000;

    require_keys_eq!(*ctx.accounts.swap_program.key, jupiter_program_id());

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.strike_reserve_usdc_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_in_usd,
    )?;

    msg!("Buy Credit With USDC: {}", amount_in_usd);

    let team_dao_fee = amount_in_usd.checked_mul(CREDIT_TEAM_DAO_FEE).ok_or(ErrorCode::Overflow)?.checked_div(PRICE_SCALE).ok_or(ErrorCode::Overflow)?;

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.strike_reserve_usdc_account.to_account_info(),
                to: ctx.accounts.fee_recipient_usdc_account.to_account_info(),
                authority: ctx.accounts.strike_reserve.to_account_info(),
            },
        ),
        team_dao_fee,
    )?;

    let accounts: Vec<AccountMeta> = ctx
    .remaining_accounts
    .iter()
    .map(|acc| {
        let is_signer = acc.key == &ctx.accounts.strike_reserve.key();
        AccountMeta {
            pubkey: *acc.key,
            is_signer,
            is_writable: acc.is_writable,
        }
    })
    .collect();

    let accounts_infos: Vec<AccountInfo> = ctx
        .remaining_accounts
        .iter()
        .map(|acc| AccountInfo { ..acc.clone() })
        .collect();

    let signer_seeds: &[&[&[u8]]] = &[&[b"strike_reserve", &[ctx.bumps.strike_reserve]]];

    invoke_signed(
        &Instruction {
            program_id: ctx.accounts.swap_program.key(),
            accounts,
            data,
        },
        &accounts_infos,
        signer_seeds,
    )?;

    emit!(BuyCreditEvent {
        user: ctx.accounts.user.key(),
        amount_in: amount_in_usd
    });

    Ok(())
}

#[derive(Accounts)]
pub struct BuyCreditUsdc<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"admin_config"],
        bump
    )]
    pub admin_config: Account<'info, AdminConfig>,    

    #[account(
        mut,
        seeds = [b"strike_reserve"],
        bump
    )]
    pub strike_reserve: SystemAccount<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = strike_reserve
    )]
    pub strike_reserve_usdc_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = km_mint,
        associated_token::authority = strike_reserve
    )]
    pub strike_reserve_km_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_usdc_account: Account<'info, TokenAccount>,

    #[account(constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Account<'info, Mint>,

    #[account(constraint = km_mint.key() == KM_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub km_mint: Account<'info, Mint>, 

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    pub swap_program: Program<'info, Jupiter>,
}

