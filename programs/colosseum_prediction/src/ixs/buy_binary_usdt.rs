use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::ErrorCode;
use crate::state::{AdminConfig, Market, MarketMethod, Position};
use crate::constants::{USDT_MINT_PUBKEY, PRICE_SCALE, KM_MINT_PUBKEY};
use crate::events::BuyBinaryEvent;
use crate::utils::{prepare_market_id_seed, ensure_position_initialized, lmsr_buy_yes_from_amount, lmsr_buy_no_from_amount, calc_fee_split};

use raydium_amm_cpi::SwapBaseIn;

pub fn buy_yes_usdt(ctx: Context<BuySharesWithUSDT>, amount: u64) -> Result<()> {
    let (fee_total, amount_after_fee, fee_buyback, fee_referral, fee_treasury) = calc_fee_split(amount)?;

    if fee_treasury > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.fee_recipient_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_treasury,
        )?;
    }

    let position = &mut ctx.accounts.position;    

    let use_referrer = (position.referrer != Pubkey::default() && position.referrer.key() == ctx.accounts.referrer.as_ref().unwrap().key()) || ctx.accounts.referrer.is_some()
    && ctx.accounts.referrer.as_ref().unwrap().key() != Pubkey::default();
    let referrer = if position.user == Pubkey::default() {
        ctx.accounts.referrer.as_ref().unwrap().key() 
     } else { 
        Pubkey::default()
     };

    if fee_referral > 0 {
        let to_account = if use_referrer {
            ctx.accounts.referrer_usdt_ata.as_ref().unwrap().to_account_info()
        } else {
            ctx.accounts.fee_recipient_token_account.to_account_info()
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: to_account,
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_referral,
        )?;
    }

    // Buyback fee goes into market vault first (so PDA can swap it via CPI)
    // if fee_buyback > 0 {
    //     token::transfer(
    //         CpiContext::new(
    //             ctx.accounts.token_program.to_account_info(),
    //             Transfer {
    //                 from: ctx.accounts.user_token_account.to_account_info(),
    //                 to: ctx.accounts.market_vault.to_account_info(),
    //                 authority: ctx.accounts.user.to_account_info(),
    //             },
    //         ),
    //         fee_buyback,
    //     )?;
    // }    

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_after_fee,
    )?;

    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let b = market.virtual_liquidity;
    let (shares_out, new_yes_price, new_no_price) =
        lmsr_buy_yes_from_amount(amount_after_fee, market.yes_volume, market.no_volume, b)?;

    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, ctx.bumps.position, market, referrer);

    position.yes_shares = position.yes_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    position.total_deposited_usdt = position.total_deposited_usdt.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    position.yes_cost = position.yes_cost
    .checked_add(amount_after_fee)
    .ok_or(ErrorCode::MathOverflow)?;
    position.fees_paid = position.fees_paid
        .checked_add(fee_total)
        .ok_or(ErrorCode::MathOverflow)?;        

    market.total_yes_shares = market.total_yes_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.yes_volume = market.yes_volume.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.yes_price = new_yes_price;
    market.no_price = new_no_price;

    market.total_volume = market.total_volume.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;

    let avg_price = if shares_out > 0 {
        (amount_after_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares_out as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    let real_price = if shares_out > 0 {
        (amount as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares_out as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    // -----------------------------
    // 4) Inline buyback swap + burn (uses fee_buyback already in market_vault)
    // -----------------------------
    if fee_buyback > 0 {
        let cpi_accounts = SwapBaseIn {
            amm: ctx.accounts.amm.clone(),
            amm_authority: ctx.accounts.amm_authority.clone(),
            amm_open_orders: ctx.accounts.amm_open_orders.clone(),
            amm_coin_vault: ctx.accounts.amm_coin_vault.clone(),
            amm_pc_vault: ctx.accounts.amm_pc_vault.clone(),
    
            market_program: ctx.accounts.serum_program.clone(),
            market: ctx.accounts.serum.clone(),
            market_bids: ctx.accounts.serum_bids.clone(),
            market_asks: ctx.accounts.serum_asks.clone(),
            market_event_queue: ctx.accounts.serum_event_queue.clone(),
            market_coin_vault: ctx.accounts.serum_coin_vault.clone(),
            market_pc_vault: ctx.accounts.serum_pc_vault.clone(),
            market_vault_signer: ctx.accounts.serum_vault_signer.clone(),
    
            user_token_source: ctx.accounts.user_usdt_vault_unchecked.clone(),
            user_token_destination: ctx.accounts.user_km_vault_unchecked.clone(),
            user_source_owner: ctx.accounts.user.clone(), // see note below
    
            // IMPORTANT: your SwapBaseIn expects Program<'info, Token>
            token_program: ctx.accounts.token_program.clone(),
        };
    
        let cpi_program = ctx.accounts.amm_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
        raydium_amm_cpi::swap_base_in(cpi_ctx, fee_buyback, 0)?;
    
        // reload for fresh amount
        // ctx.accounts.km_market_vault.reload()?;
        // let km_amount = ctx.accounts.km_market_vault.amount;
    
        // if km_amount > 0 {
        //     let burn_accounts = token::Burn {
        //         mint: ctx.accounts.km_mint.to_account_info(),
        //         from: ctx.accounts.km_market_vault.to_account_info(), // from, not to
        //         authority: market.to_account_info(),
        //     };
    
        //     let burn_ctx = CpiContext::new_with_signer(
        //         ctx.accounts.token_program.to_account_info(),
        //         burn_accounts,
        //         signer_seeds,
        //     );
        //     token::burn(burn_ctx, km_amount)?;
        // }
    }

    emit!(BuyBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        is_usdt: true,
        side_yes: true,
        amount_in: amount,
        fee: fee_total,
        amount_after_fee,
        shares_out,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
    });        
    Ok(())
}


pub fn buy_no_usdt(ctx: Context<BuySharesWithUSDT>, amount: u64) -> Result<()> {
    let (fee_total, amount_after_fee, fee_buyback, fee_referral, fee_treasury) = calc_fee_split(amount)?;

    if fee_treasury > 0 {
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.fee_recipient_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_treasury,
        )?;
    }

    let position = &mut ctx.accounts.position;    

    let use_referrer = (position.referrer != Pubkey::default() && position.referrer.key() == ctx.accounts.referrer.as_ref().unwrap().key()) || ctx.accounts.referrer.is_some()
    && ctx.accounts.referrer.as_ref().unwrap().key() != Pubkey::default();
    let referrer = if position.user == Pubkey::default() {
        ctx.accounts.referrer.as_ref().unwrap().key() 
     } else { 
        Pubkey::default()
     };

    if fee_referral > 0 {
        let to_account = if use_referrer {
            ctx.accounts.referrer_usdt_ata.as_ref().unwrap().to_account_info()
        } else {
            ctx.accounts.fee_recipient_token_account.to_account_info()
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: to_account,
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee_referral,
        )?;
    }

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount_after_fee,
    )?;

    let market = &mut ctx.accounts.market;
    require!(market.market_method == MarketMethod::Binary, ErrorCode::InvalidMarketMethod);

    let b = market.virtual_liquidity;
    let (shares_out, new_yes_price, new_no_price) =
        lmsr_buy_no_from_amount(amount_after_fee, market.yes_volume, market.no_volume, b)?;

    ensure_position_initialized(position, ctx.accounts.user.key(), &market.market_id, ctx.bumps.position, market, referrer);

    position.no_shares = position.no_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    position.total_deposited_usdt = position.total_deposited_usdt.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    position.no_cost = position.no_cost
    .checked_add(amount_after_fee)
    .ok_or(ErrorCode::MathOverflow)?;
    position.fees_paid = position.fees_paid
    .checked_add(fee_total)
    .ok_or(ErrorCode::MathOverflow)?;        

    market.total_no_shares = market.total_no_shares.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.no_volume = market.no_volume.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    market.yes_price = new_yes_price;
    market.no_price = new_no_price;

    market.total_volume = market.total_volume.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;

    let avg_price = if shares_out > 0 {
        (amount_after_fee as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares_out as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };

    let real_price = if shares_out > 0 {
        (amount as u128)
            .checked_mul(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)?
            .checked_div(shares_out as u128).ok_or(ErrorCode::MathOverflow)? as u64
    } else {
        0
    };        

    emit!(BuyBinaryEvent {
        market: market.key(),
        payer: ctx.accounts.user.key(),
        is_usdt: true,
        side_yes: false,
        amount_in: amount,
        fee: fee_total,
        amount_after_fee,
        shares_out,
        yes_price_after: new_yes_price,
        no_price_after: new_no_price,
        avg_price,
        real_price,
    });
    
    Ok(())
}

pub fn buy_km_with_usdt(ctx: Context<ProxySwapBaseIn>, amount: u64) -> Result<()> {
    raydium_amm_cpi::swap_base_in(ctx.accounts.into(), amount, 0)?;

    Ok(())
}

#[derive(Accounts)]
pub struct BuySharesWithUSDT<'info> {
    #[account(
        mut,
        seeds = [b"market", &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub market: Box<Account<'info, Market>>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Position::LEN,
        seeds = [b"position", user.key().as_ref(), &prepare_market_id_seed(&market.market_id)],
        bump
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(seeds = [b"admin_config"], bump)]
    pub admin_config: Box<Account<'info, AdminConfig>>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = user
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: Safe
    pub amm_program: UncheckedAccount<'info>,
    /// CHECK: Safe. amm Account
    #[account(mut)]
    pub amm: UncheckedAccount<'info>,
    /// CHECK: Safe. Amm authority Account
    #[account()]
    pub amm_authority: UncheckedAccount<'info>,
    /// CHECK: Safe. amm open_orders Account
    #[account(mut)]
    pub amm_open_orders: UncheckedAccount<'info>,
    /// CHECK: Safe. amm_coin_vault Amm Account to swap FROM or To,
    #[account(mut)]
    pub amm_coin_vault: UncheckedAccount<'info>,
    /// CHECK: Safe. amm_pc_vault Amm Account to swap FROM or To,
    #[account(mut)]
    pub amm_pc_vault: UncheckedAccount<'info>,
    /// CHECK: Safe.OpenBook program id
    pub serum_program: UncheckedAccount<'info>,    
    /// CHECK: Safe. OpenBook market Account. OpenBook program is the owner.
    #[account(mut)]
    pub serum: UncheckedAccount<'info>,
    /// CHECK: Safe. bids Account
    #[account(mut)]
    pub serum_bids: UncheckedAccount<'info>,
    /// CHECK: Safe. asks Account
    #[account(mut)]
    pub serum_asks: UncheckedAccount<'info>,
    /// CHECK: Safe. event_q Account
    #[account(mut)]
    pub serum_event_queue: UncheckedAccount<'info>,
    /// CHECK: Safe. coin_vault Account
    #[account(mut)]
    pub serum_coin_vault: UncheckedAccount<'info>,
    /// CHECK: Safe. pc_vault Account
    #[account(mut)]
    pub serum_pc_vault: UncheckedAccount<'info>,
    /// CHECK: Safe. vault_signer Account
    #[account(mut)]
    pub serum_vault_signer: UncheckedAccount<'info>,   

    // ✅ add these (same accounts, but as UncheckedAccount views)
    /// CHECK: same as `user_token_account`, used for Raydium CPI which expects UncheckedAccount
    #[account(mut, address = user_token_account.key())]
    pub user_usdt_vault_unchecked: UncheckedAccount<'info>,

    /// CHECK: same as `km_user_vault`, used for Raydium CPI which expects UncheckedAccount
    #[account(mut, address = km_user_vault.key())]
    pub user_km_vault_unchecked: UncheckedAccount<'info>,     

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = market
    )]
    pub market_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        init_if_needed, 
        payer = user,
        associated_token::mint = km_mint,
        associated_token::authority = user
    )]
    pub km_user_vault: Box<Account<'info, TokenAccount>>,

    pub referrer: Option<SystemAccount<'info>>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdt_mint,
        associated_token::authority = referrer
    )]
    pub referrer_usdt_ata:  Option<Box<Account<'info, TokenAccount>>>,

    #[account(
        mut,
        associated_token::mint = usdt_mint,
        associated_token::authority = admin_config.fee_recipient
    )]
    pub fee_recipient_token_account: Box<Account<'info, TokenAccount>>,

    #[account(constraint = usdt_mint.key() == USDT_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub usdt_mint: Box<Account<'info, Mint>>,

    #[account(constraint = km_mint.key() == KM_MINT_PUBKEY @ ErrorCode::InvalidMintAddress)]
    pub km_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts, Clone)]
pub struct ProxySwapBaseIn<'info> {
    /// CHECK: Safe
    pub amm_program: UncheckedAccount<'info>,
    /// CHECK: Safe. amm Account
    #[account(mut)]
    pub amm: UncheckedAccount<'info>,
    /// CHECK: Safe. Amm authority Account
    #[account()]
    pub amm_authority: UncheckedAccount<'info>,
    /// CHECK: Safe. amm open_orders Account
    #[account(mut)]
    pub amm_open_orders: UncheckedAccount<'info>,
    /// CHECK: Safe. amm_coin_vault Amm Account to swap FROM or To,
    #[account(mut)]
    pub amm_coin_vault: UncheckedAccount<'info>,
    /// CHECK: Safe. amm_pc_vault Amm Account to swap FROM or To,
    #[account(mut)]
    pub amm_pc_vault: UncheckedAccount<'info>,
    /// CHECK: Safe.OpenBook program id
    pub market_program: UncheckedAccount<'info>,
    /// CHECK: Safe. OpenBook market Account. OpenBook program is the owner.
    #[account(mut)]
    pub market: UncheckedAccount<'info>,
    /// CHECK: Safe. bids Account
    #[account(mut)]
    pub market_bids: UncheckedAccount<'info>,
    /// CHECK: Safe. asks Account
    #[account(mut)]
    pub market_asks: UncheckedAccount<'info>,
    /// CHECK: Safe. event_q Account
    #[account(mut)]
    pub market_event_queue: UncheckedAccount<'info>,
    /// CHECK: Safe. coin_vault Account
    #[account(mut)]
    pub market_coin_vault: UncheckedAccount<'info>,
    /// CHECK: Safe. pc_vault Account
    #[account(mut)]
    pub market_pc_vault: UncheckedAccount<'info>,
    /// CHECK: Safe. vault_signer Account
    #[account(mut)]
    pub market_vault_signer: UncheckedAccount<'info>,
    /// CHECK: Safe. user source token Account. user Account to swap from.
    #[account(mut)]
    pub user_token_source: UncheckedAccount<'info>,
    /// CHECK: Safe. user destination token Account. user Account to swap to.
    #[account(mut)]
    pub user_token_destination: UncheckedAccount<'info>,
    /// CHECK: Safe. user owner Account
    #[account(mut)]
    pub user_source_owner: Signer<'info>,
    /// CHECK: Safe. The spl token program
    pub token_program: Program<'info, Token>,
}

impl<'a, 'b, 'c, 'info> From<&mut ProxySwapBaseIn<'info>>
    for CpiContext<'a, 'b, 'c, 'info, SwapBaseIn<'info>>
{
    fn from(
        accounts: &mut ProxySwapBaseIn<'info>,
    ) -> CpiContext<'a, 'b, 'c, 'info, SwapBaseIn<'info>> {
        let cpi_accounts = SwapBaseIn {
            amm: accounts.amm.clone(),
            amm_authority: accounts.amm_authority.clone(),
            amm_open_orders: accounts.amm_open_orders.clone(),
            amm_coin_vault: accounts.amm_coin_vault.clone(),
            amm_pc_vault: accounts.amm_pc_vault.clone(),
            market_program: accounts.market_program.clone(),
            market: accounts.market.clone(),
            market_bids: accounts.market_bids.clone(),
            market_asks: accounts.market_asks.clone(),
            market_event_queue: accounts.market_event_queue.clone(),
            market_coin_vault: accounts.market_coin_vault.clone(),
            market_pc_vault: accounts.market_pc_vault.clone(),
            market_vault_signer: accounts.market_vault_signer.clone(),
            user_token_source: accounts.user_token_source.clone(),
            user_token_destination: accounts.user_token_destination.clone(),
            user_source_owner: accounts.user_source_owner.clone(),
            token_program: accounts.token_program.clone(),
        };
        let cpi_program = accounts.amm_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}