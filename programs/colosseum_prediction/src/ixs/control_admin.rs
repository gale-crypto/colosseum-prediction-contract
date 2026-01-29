use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::state::AdminConfig;


pub fn add_admin(ctx: Context<ManageAdmin>, admin_address: Pubkey) -> Result<()> {
    let admin_config = &mut ctx.accounts.admin_config;
    require!(admin_config.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
    require!(!admin_config.admins.contains(&admin_address), ErrorCode::AdminAlreadyExists);
    admin_config.admins.push(admin_address);
    Ok(())
}

pub fn remove_admin(ctx: Context<ManageAdmin>, admin_address: Pubkey) -> Result<()> {
    let admin_config = &mut ctx.accounts.admin_config;
    require!(admin_config.authority == ctx.accounts.authority.key(), ErrorCode::Unauthorized);
    admin_config.admins.retain(|&x| x != admin_address);
    Ok(())
}

#[derive(Accounts)]
pub struct ManageAdmin<'info> {
    #[account(
        mut,
        seeds = [b"admin_config"],
        bump = admin_config.bump
    )]
    pub admin_config: Account<'info, AdminConfig>,
    pub authority: Signer<'info>,
}