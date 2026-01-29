use anchor_lang::prelude::*;

pub fn initialize_market(
    ctx: Context<InitializeMarket>,
    market_id: String,
    market_method: MarketMethod,
    initial_yes_price: u64,
    initial_no_price: u64,
    options: Vec<String>,
    initial_option_prices: Vec<u64>,
) -> Result<()> {
    let admin_config = &ctx.accounts.admin_config;
    let creator = ctx.accounts.creator.key();

    match market_method {
        MarketMethod::Binary => {
            require!(options.is_empty() && initial_option_prices.is_empty(), ErrorCode::InvalidMarketMethod);
            require!(
                initial_yes_price
                    .checked_add(initial_no_price)
                    .ok_or(ErrorCode::MathOverflow)?
                    == PRICE_SCALE,
                ErrorCode::InvalidOptionPrices
            );
        }
        MarketMethod::MultiChoice => {
            require!(options.len() >= 2 && options.len() <= Market::MAX_OPTIONS, ErrorCode::InvalidOptionsCount);
            require!(initial_option_prices.len() == options.len(), ErrorCode::InvalidOptionPrices);
            let total_price: u64 = initial_option_prices.iter().sum();
            require!(total_price >= 950_000 && total_price <= 1_050_000, ErrorCode::InvalidOptionPrices);
        }
    }

    let is_admin = admin_config.admins.contains(&creator);
    if !is_admin {
        require!(ctx.accounts.creator.lamports() >= MARKET_CREATION_FEE, ErrorCode::InsufficientFunds);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.creator.to_account_info(),
                to: ctx.accounts.fee_recipient.to_account_info(),
            },
        );
        system_program::transfer(cpi_context, MARKET_CREATION_FEE)?;
    }

    let market = &mut ctx.accounts.market;

    market.market_id = market_id.clone();
    market.market_method = market_method;
    market.bump = ctx.bumps.market;

    market.virtual_liquidity = DEFAULT_VIRTUAL_LIQUIDITY;

    market.creator = creator;
    market.creation_fee_paid = !is_admin;

    market.resolution_status = ResolutionStatus::Open;
    market.outcome = MarketOutcome::Cancelled; // default until resolved

    market.total_volume = 0;

    market.options = options;
    market.option_prices = initial_option_prices.clone();
    market.option_volumes = vec![0; initial_option_prices.len()];
    market.total_option_shares = vec![0; initial_option_prices.len()];

    market.settle_total_pool = 0;
    market.settle_total_winning_shares = 0;
    market.settle_payout_per_share = 0;
    market.settle_initialized = false;

    if market_method == MarketMethod::Binary {
        market.yes_price = initial_yes_price;
        market.no_price = initial_no_price;

        // Binary LMSR: store q_yes/q_no in yes_volume/no_volume
        let (q_yes, q_no) = lmsr_seed_q_from_initial_prices(
            initial_yes_price,
            initial_no_price,
            market.virtual_liquidity, // b
        )?;

        market.yes_volume = q_yes;
        market.no_volume = q_no;

        market.total_yes_shares = 0;
        market.total_no_shares = 0;
        market.total_volume = 0;
    } else {
        // Multi LMSR: store q_i in option_volumes
        market.yes_price = 0;
        market.no_price = 0;
        market.yes_volume = 0;
        market.no_volume = 0;

        let qs = lmsr_seed_q_vec_from_initial_option_prices(&initial_option_prices, market.virtual_liquidity)?;
        market.option_volumes = qs;

        // Set prices from seeded qs (exactly consistent)
        market.option_prices = lmsr_prices_multi(&market.option_volumes, market.virtual_liquidity)?;

        market.total_yes_shares = 0;
        market.total_no_shares = 0;
    }

    Ok(())
}


pub struct InitializeMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + Market::LEN,
        seeds = [b"market", &market_id.as_bytes()[..32.min(market_id.len())]],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = usdt_mint,
        associated_token::authority = market
    )]
    pub market_usdt_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = usdc_mint,
        associated_token::authority = market
    )]
    pub market_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [b"admin_config"],
        bump = admin_config.bump
    )]
    pub admin_config: Account<'info, AdminConfig>,

    #[account(mut, constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Account<'info, Mint>,

    #[account(mut, constraint = usdc_mint.key() == USDC_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdc_mint: Account<'info, Mint>,

    /// CHECK:
    #[account(mut)]
    pub fee_recipient: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
