use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
// use jupiter_aggregator::program::Jupiter;

use crate::errors::ErrorCode;
use crate::state::AdminConfig;
use crate::constants::{USDT_MINT_PUBKEY, USDC_MINT_PUBKEY};

pub fn initialize_admin_config(ctx: Context<InitializeAdminConfig>) -> Result<()> {
    let admin_config = &mut ctx.accounts.admin_config;
    admin_config.authority = ctx.accounts.authority.key();
    admin_config.fee_recipient = ctx.accounts.fee_recipient.key();
    // admin_config.swap_program = ctx.accounts.swap_program.key();
    admin_config.admins = Vec::new();
    admin_config.bump = ctx.bumps.admin_config;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeAdminConfig<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + AdminConfig::LEN,
        seeds = [b"admin_config"],
        bump
    )]
    pub admin_config: Account<'info, AdminConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub fee_recipient: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = usdt_mint,
        associated_token::authority = fee_recipient
    )]
    pub fee_recipient_usdt_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = usdc_mint,
        associated_token::authority = fee_recipient
    )]
    pub fee_recipient_usdc_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"strike_reserve"],
        bump
    )]
    pub strike_reserve: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = usdt_mint,
        associated_token::authority = strike_reserve
    )]  
    pub strike_reserve_usdt_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = usdc_mint,
        associated_token::authority = strike_reserve
    )]
    pub strike_reserve_usdc_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Account<'info, Mint>,

    #[account(mut, constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // pub swap_program: Program<'info, Jupiter>,
}
