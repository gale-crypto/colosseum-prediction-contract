use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

pub fn initialize_admin_config(ctx: Context<InitializeAdminConfig>) -> Result<()> {
    let admin_config = &mut ctx.accounts.admin_config;
    admin_config.authority = ctx.accounts.authority.key();
    admin_config.fee_recipient = ctx.accounts.fee_recipient.key();
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

    #[account(mut, constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Account<'info, Mint>,

    #[account(mut, constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
