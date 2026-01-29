use anchor_lang::prelude::*;

pub fn sell_option(ctx: Context<SellOption>, option_index: u8, shares: u64) -> Result<()> {
    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::MultiChoice, ErrorCode::InvalidMarketMethod);

    let idx = option_index as usize;
    require!(idx < market.options.len(), ErrorCode::InvalidOptionIndex);

    let position = &mut ctx.accounts.position;
    let pos_bump = position.bump;
    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, pos_bump, market);
    require!(position.option_shares.len() == market.options.len(), ErrorCode::InvalidPositionAccount);
    require!(position.option_shares[idx] >= shares, ErrorCode::InsufficientShares);

    // LMSR refund
    let b = market.virtual_liquidity;
    let (payout_before_fee, new_prices) = lmsr_sell_option_to_amount(shares, &market.option_volumes, idx, b)?;

    let (fee_amount, payout_after_fee) = calc_fee(payout_before_fee)?;

    let market_id_seed = prepare_market_id_seed(&market.market_id);
    let signer_seeds: &[&[&[u8]]] = &[&[b"market", &market_id_seed, &[market.bump]]];

    let usdt_balance = ctx.accounts.market_usdt_vault.amount;
    let usdc_balance = ctx.accounts.market_usdc_vault.amount;

    let (fee_usdt, fee_usdc, pay_usdt, pay_usdc) =
        split_payout(payout_before_fee, payout_after_fee, fee_amount, usdt_balance, usdc_balance)?;

    // fee transfers
    if fee_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_usdt_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            fee_usdt,
        )?;
    }
    if fee_usdc > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdc_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_usdc_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            fee_usdc,
        )?;
    }

    // payout transfers
    if pay_usdt > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdt_vault.to_account_info(),
                    to: ctx.accounts.user_usdt_token_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            pay_usdt,
        )?;
    }
    if pay_usdc > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_usdc_vault.to_account_info(),
                    to: ctx.accounts.user_usdc_token_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds,
            ),
            pay_usdc,
        )?;
    }

    let shares_before = position.option_shares[idx];
    let cost_before = position.option_costs[idx];
    
    let cost_removed = avg_cost_remove(cost_before, shares_before, shares)?;
    position.option_costs[idx] = position.option_costs[idx].checked_sub(cost_removed).ok_or(ErrorCode::MathOverflow)?;
    
    position.realized_pnl = position.realized_pnl
        .checked_add(payout_after_fee as i64).ok_or(ErrorCode::MathOverflow)?
        .checked_sub(cost_removed as i64).ok_or(ErrorCode::MathOverflow)?;

    // position + market shares
    position.option_shares[idx] = position.option_shares[idx].checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    market.total_option_shares[idx] = market.total_option_shares[idx].checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;

    // q update
    market.option_volumes[idx] = market.option_volumes[idx].checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;

    // price update
    market.option_prices = new_prices;

    position.fees_paid = position.fees_paid
    .checked_add(fee_amount)
    .ok_or(ErrorCode::MathOverflow)?;

    position.total_withdrawn_usdt = position.total_withdrawn_usdt
        .checked_add(pay_usdt)
        .ok_or(ErrorCode::MathOverflow)?;
    position.total_withdrawn_usdc = position.total_withdrawn_usdc
        .checked_add(pay_usdc)
        .ok_or(ErrorCode::MathOverflow)?;        


    let avg_price = if shares > 0 {
        (payout_before_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    let real_price = if shares > 0 {
        (payout_after_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };


    emit!(SellOptionEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        option_index,
        shares_in: shares,
        payout_before_fee,
        fee: fee_amount,
        payout_after_fee,
        option_prices_after: market.option_prices.clone(), // already set to new_prices
        avg_price,
        real_price,
        pay_usdt,
        pay_usdc,
    });
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(option_index: u8, shares: u64)]
pub struct SellOption<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        mut,
        seeds = [b"position", user.key().as_ref(), &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Box<Account<'info, AdminConfig>>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdt_mint,
        associated_token::authority = user
    )]
    pub user_usdt_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdc_mint,
        associated_token::authority = user
    )]
    pub user_usdc_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = market
    )]
    pub market_usdt_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = market
    )]
    pub market_usdc_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_usdt_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_usdc_account: Box<Account<'info, TokenAccount>>,

    #[account(constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Box<Account<'info, Mint>>,

    #[account(constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

