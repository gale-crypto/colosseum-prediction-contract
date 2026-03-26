use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;

use crate::errors::ErrorCode;
use crate::state::AdminConfig;
use crate::constants::{USDC_MINT_PUBKEY, KM_MINT_PUBKEY, PRICE_SCALE, CREDIT_TEAM_DAO_FEE, CREDIT_BURN_FEE, CREDIT_RESERVE_FEE};
use crate::events::BuyCreditEvent;

pub fn buy_credit_usdc(ctx: Context<BuyCreditUsdc>, data: Vec<u8>) -> Result<()> {
    let amount_in_usd: u64 = 10_000_000;

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

    // Build swap accounts from the remaining accounts
    // The swap accounts are passed in as remaining accounts
    let remaining_accounts = ctx.remaining_accounts;
    
    // Create account metas for the swap instruction
    let swap_account_metas: Vec<AccountMeta> = remaining_accounts
        .iter()
        .map(|acc| {
            AccountMeta {
                pubkey: *acc.key,
                is_signer: acc.is_signer,
                is_writable: acc.is_writable,
            }
        })
        .collect();  

    // Collect account infos for the swap instruction
    let swap_account_infos: Vec<AccountInfo> = remaining_accounts
    .iter()
    .map(|acc| acc.clone())
    .collect();

    // Invoke swap program to swap USDC to target token
    invoke_signed(
        &Instruction {
            program_id: admin_config.swap_program,
            accounts: swap_account_metas,
            data,
        },
        &swap_account_infos,
        &[], // No additional signers needed if the swap doesn't require PDAs
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
}

