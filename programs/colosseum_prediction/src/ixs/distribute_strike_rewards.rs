use anchor_lang::prelude::*;
use anchor_spl::token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;

use crate::errors::ErrorCode;
use crate::events::DistributeStrikeRewardsEvent;
use crate::state::AdminConfig;
use crate::constants::{USDC_MINT_PUBKEY, KM_MINT_PUBKEY, CREDIT_RESERVE_FEE, CREDIT_TEAM_DAO_FEE, CREDIT_BURN_FEE, WINNER_1_BPS, WINNER_2_BPS, WINNER_3_BPS, PRICE_SCALE};

const ONE_WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;

pub fn distribute_strike_rewards(ctx: Context<DistributeStrikeRewards>, total_pool: u64, data: Vec<u8>) -> Result<()> {
    let admin_config = &mut ctx.accounts.admin_config;
    let admin_key = ctx.accounts.authority.key();
    let now = Clock::get()?.unix_timestamp;


    let is_admin = admin_config.admins.contains(&admin_key);
    require!(is_admin, ErrorCode::Unauthorized);
    require!(
        WINNER_1_BPS + WINNER_2_BPS + WINNER_3_BPS <= PRICE_SCALE,
        ErrorCode::InvalidWinnerPayoutSplit
    );

    if admin_config.last_strike_distribution_ts > 0 {
        let next_allowed = admin_config
            .last_strike_distribution_ts
            .checked_add(ONE_WEEK_SECONDS)
            .ok_or(ErrorCode::Overflow)?;
        require!(now >= next_allowed, ErrorCode::WeeklyDistributionTooSoon);
    }

    let winner_1_amount = total_pool
        .checked_mul(WINNER_1_BPS)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ErrorCode::Overflow)?;
    let winner_2_amount = total_pool
        .checked_mul(WINNER_2_BPS)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ErrorCode::Overflow)?;
    let winner_3_amount = total_pool
        .checked_mul(WINNER_3_BPS)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ErrorCode::Overflow)?;

    let burn_amount = total_pool
        .checked_mul(CREDIT_BURN_FEE)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ErrorCode::Overflow)?;
    let fee_amount = total_pool
        .checked_mul(CREDIT_TEAM_DAO_FEE)
        .ok_or(ErrorCode::Overflow)?
        .checked_div(PRICE_SCALE)
        .ok_or(ErrorCode::Overflow)?;

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

    // Record pre-swap balances
    let pre_swap_token_balance = ctx.accounts.strike_reserve_km_account.amount;
    let pre_swap_usdc_balance = ctx.accounts.strike_reserve_usdc_account.amount;
    
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

    // Verify the swap occurred correctly
    let post_swap_token_balance = ctx.accounts.strike_reserve_km_account.amount;
    let post_swap_usdc_balance = ctx.accounts.strike_reserve_usdc_account.amount;

    // Calculate how much token was received
    let received_token_amount = post_swap_token_balance
    .checked_sub(pre_swap_token_balance)
    .ok_or(ErrorCode::MathOverflow)?;

    // Ensure USDC was actually spent from the reserve
    require!(
    post_swap_usdc_balance < pre_swap_usdc_balance,
    ErrorCode::SwapFailed
    );

    let spent_usdc_amount = pre_swap_usdc_balance
    .checked_sub(post_swap_usdc_balance)
    .ok_or(ErrorCode::MathOverflow)?;

    msg!("Swapped {} USDC for {} tokens", spent_usdc_amount, received_token_amount);

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
        received_token_amount,
    )?;

    msg!("Burned {} tokens", received_token_amount);    

    admin_config.last_strike_distribution_ts = now;

    emit!(DistributeStrikeRewardsEvent {
        distributed_by: admin_key,
        total_pool: total_pool,
        winner_1: ctx.accounts.winner_1.key(),
        winner_2: ctx.accounts.winner_2.key(),
        winner_3: ctx.accounts.winner_3.key(),
        burn_amount: burn_amount,
        fee_amount: fee_amount,
        winner_1_amount: winner_1_amount,
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

