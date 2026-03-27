use anchor_lang::prelude::*;
use anchor_spl::token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::events::DistributeStrikeRewardsEvent;
use crate::state::AdminConfig;
use crate::constants::{USDC_MINT_PUBKEY, KM_MINT_PUBKEY, CREDIT_RESERVE_FEE, WINNER_1_BPS, WINNER_2_BPS, WINNER_3_BPS};

const ONE_WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;

pub fn distribute_strike_rewards(ctx: Context<DistributeStrikeRewards>, total_usdc_amount: u64, total_burn_amount: u64) -> Result<()> {
    let admin_config = &mut ctx.accounts.admin_config;
    let admin_key = ctx.accounts.authority.key();
    let now = Clock::get()?.unix_timestamp;


    let is_admin = admin_config.admins.contains(&admin_key);
    require!(is_admin, ErrorCode::Unauthorized);
    require!(
        WINNER_1_BPS + WINNER_2_BPS + WINNER_3_BPS <= CREDIT_RESERVE_FEE,
        ErrorCode::InvalidWinnerPayoutSplit
    );

    if admin_config.last_strike_distribution_ts > 0 {
        let next_allowed = admin_config
            .last_strike_distribution_ts
            .checked_add(ONE_WEEK_SECONDS)
            .ok_or(ErrorCode::Overflow)?;
        require!(now >= next_allowed, ErrorCode::WeeklyDistributionTooSoon);
    }

    let winner_1_amount = total_usdc_amount
        .checked_mul(WINNER_1_BPS)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(CREDIT_RESERVE_FEE)
        .ok_or(ErrorCode::Overflow)?;
    let winner_2_amount = total_usdc_amount
        .checked_mul(WINNER_2_BPS)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(CREDIT_RESERVE_FEE)
        .ok_or(ErrorCode::Overflow)?;
    let winner_3_amount = total_usdc_amount
        .checked_sub(winner_1_amount)
        .ok_or(ErrorCode::Overflow)?
        .checked_sub(winner_2_amount)
        .ok_or(ErrorCode::Overflow)?;
    
    let burn_amount = total_burn_amount;

    let bump = ctx.bumps.strike_reserve;
    let signer_seeds: &[&[u8]] = &[b"strike_reserve", &[bump]];
    let signer = &[signer_seeds];

    if winner_1_amount > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.strike_reserve_usdc_account.to_account_info(),
                    to: ctx.accounts.winner_1_token_account.to_account_info(),
                    authority: ctx.accounts.strike_reserve.to_account_info(),
                },
                signer,
            ),
            winner_1_amount,
        )?;
    }

    if winner_2_amount > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.strike_reserve_usdc_account.to_account_info(),
                    to: ctx.accounts.winner_2_token_account.to_account_info(),
                    authority: ctx.accounts.strike_reserve.to_account_info(),
                },
                signer,
            ),
            winner_2_amount,
        )?;
    }

    if winner_3_amount > 0 {
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.strike_reserve_usdc_account.to_account_info(),
                    to: ctx.accounts.winner_3_token_account.to_account_info(),
                    authority: ctx.accounts.strike_reserve.to_account_info(),
                },
                signer,
            ),
            winner_3_amount,
        )?;
    }

    msg!("Winner 1: {}", winner_1_amount);
    msg!("Winner 2: {}", winner_2_amount);
    msg!("Winner 3: {}", winner_3_amount);

    // Burn the received tokens
    burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.km_mint.to_account_info(),
                from: ctx.accounts.strike_reserve_km_account.to_account_info(),
                authority: ctx.accounts.strike_reserve.to_account_info(),
            },
        ).with_signer(signer),
        burn_amount,
    )?;

    msg!("Burned {} tokens", burn_amount);    

    admin_config.last_strike_distribution_ts = now;

    emit!(DistributeStrikeRewardsEvent {
        distributed_by: admin_key,
        total_usdc_amount: total_usdc_amount,
        winner_1: ctx.accounts.winner_1.key(),
        winner_2: ctx.accounts.winner_2.key(),
        winner_3: ctx.accounts.winner_3.key(),
        burn_amount: burn_amount,
        winner_1_amount: winner_1_amount,
        winner_2_amount: winner_2_amount,
        winner_3_amount: winner_3_amount,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct DistributeStrikeRewards<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"admin_config"],
        bump = admin_config.bump
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
        associated_token::mint = km_mint,
        associated_token::authority = strike_reserve
    )]
    pub strike_reserve_km_account: Account<'info, TokenAccount>,    

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = strike_reserve
    )]
    pub strike_reserve_usdc_account: Account<'info, TokenAccount>,

    #[account(constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]    
    pub usdc_mint: Account<'info, Mint>,

    #[account(constraint = km_mint.key() == KM_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub km_mint: Account<'info, Mint>,

    pub winner_1: SystemAccount<'info>,
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = winner_1
    )]
    pub winner_1_token_account: Account<'info, TokenAccount>,

    pub winner_2: SystemAccount<'info>,
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = winner_2
    )]
    pub winner_2_token_account: Account<'info, TokenAccount>,

    pub winner_3: SystemAccount<'info>,
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = winner_3
    )]
    pub winner_3_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

