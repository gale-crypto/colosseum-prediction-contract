use anchor_lang::prelude::*;
use raydium_amm_cpi::SwapBaseIn;

use crate::errors::ErrorCode;

// -----------------------
// Small math helpers
// -----------------------

#[inline(always)]
pub fn clamp_exp(x: f64) -> f64 {
    x.max(-EXP_CLAMP).min(EXP_CLAMP)
}

#[inline(always)]
pub fn u64_to_f64_units(x: u64) -> f64 {
    (x as f64) / (PRICE_SCALE as f64)
}

#[inline(always)]
pub fn f64_units_to_u64_floor(x: f64) -> Result<u64> {
    require!(x.is_finite() && x >= 0.0, ErrorCode::MathOverflow);
    let scaled = x * (PRICE_SCALE as f64);
    require!(scaled.is_finite() && scaled <= (u64::MAX as f64), ErrorCode::MathOverflow);
    Ok(scaled.floor() as u64)
}

#[inline(always)]
pub fn price_no_from_yes(yes_price: u64) -> Result<u64> {
    PRICE_SCALE
        .checked_sub(yes_price)
        .ok_or(ErrorCode::MathOverflow.into())
}

/// Prepare market_id for PDA seeds by taking first 32 bytes
pub fn prepare_market_id_seed(market_id: &str) -> [u8; 32] {
    let bytes = market_id.as_bytes();
    let len = 32.min(bytes.len());
    let mut result = [0u8; 32];
    result[..len].copy_from_slice(&bytes[..len]);
    result
}

/// ---------------------------
/// Helper: fee split
/// ---------------------------
pub fn calc_fee_split(amount: u64) -> Result<(u64, u64, u64, u64, u64)> {
    // returns (fee_total, after_fee, fee_buyback, fee_referral, fee_treasury)

    let fee_total = (amount as u128)
        .checked_mul(FEE_TOTAL as u128).ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)? as u64;

    let after_fee = amount.checked_sub(fee_total).ok_or(ErrorCode::MathOverflow)?;

    let fee_buyback = (amount as u128)
        .checked_mul(FEE_BUYBACK as u128).ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)? as u64;

    let fee_referral = (amount as u128)
        .checked_mul(FEE_REFERRAL as u128).ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)? as u64;

    let fee_treasury = fee_total
        .checked_sub(fee_buyback).ok_or(ErrorCode::MathOverflow)?
        .checked_sub(fee_referral).ok_or(ErrorCode::MathOverflow)?;

    Ok((fee_total, after_fee, fee_buyback, fee_referral, fee_treasury))
}

pub fn calc_fee(amount: u64) -> Result<(u64, u64)> {
    let fee_amount = (amount as u128)
        .checked_mul(FEE_TOTAL as u128).ok_or(ErrorCode::MathOverflow)?
        .checked_div(PRICE_SCALE as u128).ok_or(ErrorCode::MathOverflow)? as u64;

    let after_fee = amount.checked_sub(fee_amount).ok_or(ErrorCode::MathOverflow)?;

    Ok((fee_amount, after_fee))
}

/// Ensure a Position is initialized if created via init_if_needed.
pub fn ensure_position_initialized(
    position: &mut Position,
    user: Pubkey,
    market_id: &str,
    bump: u8,
    market: &Market,
) {
    if position.user == Pubkey::default() {
        position.user = user;
        position.market_id = market_id.to_string();
        position.referrer = Pubkey::default(); // NEW        
        position.yes_shares = 0;
        position.no_shares = 0;
        position.total_deposited_usdt = 0;
        position.total_deposited_usdc = 0;
        position.bump = bump;
        position.option_shares = match market.market_method {
            MarketMethod::Binary => Vec::new(),
            MarketMethod::MultiChoice => vec![0; market.options.len()],
        };

        position.yes_cost = 0;
        position.no_cost = 0;
        position.realized_pnl = 0;
        position.fees_paid = 0;
        position.total_withdrawn_usdt = 0;
        position.total_withdrawn_usdc = 0;
        
        position.option_costs = match market.market_method {
            MarketMethod::Binary => Vec::new(),
            MarketMethod::MultiChoice => vec![0; market.options.len()],
        };        
    } else if position.option_shares.is_empty() && market.market_method == MarketMethod::MultiChoice {
        position.option_shares = vec![0; market.options.len()];
        position.option_costs = vec![0; market.options.len()];
    }
}

/// Split payout between USDT and USDC based on vault balances (1 USD stable assumption).
pub fn split_payout(
    payout_before_fee: u64,
    payout_after_fee: u64,
    fee_amount: u64,
    usdt_balance: u64,
    usdc_balance: u64,
) -> Result<(u64, u64, u64, u64)> {
    let total_balance = usdt_balance
        .checked_add(usdc_balance)
        .ok_or(ErrorCode::MathOverflow)?;
    require!(total_balance >= payout_before_fee, ErrorCode::InsufficientMarketLiquidity);

    let mut fee_usdt = 0u64;
    let mut fee_usdc = 0u64;
    let mut pay_usdt = 0u64;
    let mut pay_usdc = 0u64;

    if payout_before_fee <= usdt_balance {
        pay_usdt = payout_after_fee;
        fee_usdt = fee_amount;
    } else if fee_amount <= usdt_balance {
        pay_usdt = usdt_balance
            .checked_sub(fee_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        fee_usdt = fee_amount;

        pay_usdc = payout_after_fee
            .checked_sub(pay_usdt)
            .ok_or(ErrorCode::MathOverflow)?;
    } else {
        fee_usdt = usdt_balance;
        fee_usdc = fee_amount
            .checked_sub(fee_usdt)
            .ok_or(ErrorCode::MathOverflow)?;
        pay_usdc = payout_after_fee;
    }

    Ok((fee_usdt, fee_usdc, pay_usdt, pay_usdc))
}

// -----------------------
// P/L helpers
// -----------------------
pub fn avg_cost_remove(cost: u64, shares_total: u64, shares_sold: u64) -> Result<u64> {
    require!(shares_total > 0, ErrorCode::MathOverflow);
    require!(shares_sold <= shares_total, ErrorCode::MathOverflow);

    // cost_removed = cost * shares_sold / shares_total (u128 safe)
    let removed = (cost as u128)
        .checked_mul(shares_sold as u128)
        .ok_or(ErrorCode::MathOverflow)?
        .checked_div(shares_total as u128)
        .ok_or(ErrorCode::MathOverflow)? as u64;

    Ok(removed)
}

// -----------------------
// LMSR (Binary)
// -----------------------

/// LMSR spot price for YES:
/// p_yes = exp(qy/b) / (exp(qy/b) + exp(qn/b))
pub fn lmsr_price_yes(q_yes: u64, q_no: u64, b: u64) -> Result<u64> {
    require!(b > 0, ErrorCode::MathOverflow);

    let qy = u64_to_f64_units(q_yes);
    let qn = u64_to_f64_units(q_no);
    let bb = u64_to_f64_units(b);

    let ey = (clamp_exp(qy / bb)).exp();
    let en = (clamp_exp(qn / bb)).exp();
    let denom = ey + en;
    if denom == 0.0 {
        return Ok(PRICE_SCALE / 2);
    }

    let p = ey / denom;
    let p_scaled = (p * (PRICE_SCALE as f64)).round() as u64;

    Ok(p_scaled.max(MIN_PRICE).min(MAX_PRICE))
}

/// LMSR cost function (collateral whole units):
/// C = b * ln(exp(qy/b) + exp(qn/b))
pub fn lmsr_cost_units(q_yes: u64, q_no: u64, b: u64) -> Result<f64> {
    require!(b > 0, ErrorCode::MathOverflow);

    let qy = u64_to_f64_units(q_yes);
    let qn = u64_to_f64_units(q_no);
    let bb = u64_to_f64_units(b);

    // log-sum-exp for stability
    let ay = clamp_exp(qy / bb);
    let an = clamp_exp(qn / bb);
    let m = ay.max(an);
    let sum = (ay - m).exp() + (an - m).exp();

    let cost = bb * (m + sum.ln());
    require!(cost.is_finite(), ErrorCode::MathOverflow);
    Ok(cost)
}

/// Seed q_yes/q_no to match initial price exactly.
pub fn lmsr_seed_q_from_initial_prices(initial_yes_price: u64, initial_no_price: u64, b: u64) -> Result<(u64, u64)> {
    require!(
        initial_yes_price
            .checked_add(initial_no_price)
            .ok_or(ErrorCode::MathOverflow)?
            == PRICE_SCALE,
        ErrorCode::InvalidOptionPrices
    );
    require!(
        initial_yes_price >= MIN_PRICE && initial_yes_price <= MAX_PRICE,
        ErrorCode::InvalidOptionPrices
    );
    require!(b > 0, ErrorCode::MathOverflow);

    let p = (initial_yes_price as f64) / (PRICE_SCALE as f64);
    let q = (initial_no_price as f64) / (PRICE_SCALE as f64);
    require!(p > 0.0 && q > 0.0, ErrorCode::InvalidOptionPrices);

    let bb = u64_to_f64_units(b);

    let a_yes = bb * p.ln(); // can be negative
    let a_no = bb * q.ln(); // can be negative

    // choose offset so min becomes 0
    let offset = (-a_yes.min(a_no)).max(0.0);

    let q_yes_units = a_yes + offset;
    let q_no_units = a_no + offset;

    let q_yes_u = f64_units_to_u64_floor(q_yes_units)?;
    let q_no_u = f64_units_to_u64_floor(q_no_units)?;

    Ok((q_yes_u, q_no_u))
}

/// BUY YES with net spend x_net (scaled 1e6 collateral).
pub fn lmsr_buy_yes_from_amount(x_net: u64, q_yes: u64, q_no: u64, b: u64) -> Result<(u64, u64, u64)> {
    require!(b > 0, ErrorCode::MathOverflow);

    let x = u64_to_f64_units(x_net);
    let qy = u64_to_f64_units(q_yes);
    let qn = u64_to_f64_units(q_no);
    let bb = u64_to_f64_units(b);

    // Δ = (qn - qy)/b
    let delta = (qn - qy) / bb;
    let ed = (clamp_exp(delta)).exp();
    let ex = (clamp_exp(x / bb)).exp();

    // dq = b * ln( (e^{x/b}*(1+e^{Δ}) - 1) / e^{Δ} )
    let inside = ex * (1.0 + ed) - ed;
    require!(inside.is_finite() && inside > 0.0, ErrorCode::MathOverflow);

    let dq_units = bb * inside.ln();
    let shares_out = f64_units_to_u64_floor(dq_units)?;

    let new_q_yes = q_yes.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    let new_yes_price = lmsr_price_yes(new_q_yes, q_no, b)?;
    let new_no_price = price_no_from_yes(new_yes_price)?;
    Ok((shares_out, new_yes_price, new_no_price))
}

pub fn lmsr_buy_no_from_amount(x_net: u64, q_yes: u64, q_no: u64, b: u64) -> Result<(u64, u64, u64)> {
    require!(b > 0, ErrorCode::MathOverflow);

    let x = u64_to_f64_units(x_net);
    let qy = u64_to_f64_units(q_yes);
    let qn = u64_to_f64_units(q_no);
    let bb = u64_to_f64_units(b);

    // symmetric (swap roles)
    let delta = (qy - qn) / bb;
    let r  = (clamp_exp(delta)).exp();
    let ex = (clamp_exp(x / bb)).exp();

    // ✅ Correct closed form
    let inside = ex * (1.0 + r) - r;
    require!(inside.is_finite() && inside > 0.0, ErrorCode::MathOverflow);

    let dq_units = bb * inside.ln();
    let shares_out = f64_units_to_u64_floor(dq_units)?;

    let new_q_no = q_no.checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;
    let new_yes_price = lmsr_price_yes(q_yes, new_q_no, b)?;
    let new_no_price = price_no_from_yes(new_yes_price)?;
    Ok((shares_out, new_yes_price, new_no_price))
}

/// SELL YES shares -> payout_before_fee (scaled 1e6 collateral)
pub fn lmsr_sell_yes_to_amount(shares: u64, q_yes: u64, q_no: u64, b: u64) -> Result<(u64, u64, u64)> {
    require!(shares <= q_yes, ErrorCode::InsufficientShares);

    let c_before = lmsr_cost_units(q_yes, q_no, b)?;
    let new_q_yes = q_yes.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    let c_after = lmsr_cost_units(new_q_yes, q_no, b)?;

    let refund_units = c_before - c_after;
    require!(refund_units.is_finite() && refund_units >= 0.0, ErrorCode::MathOverflow);

    let payout_before_fee = f64_units_to_u64_floor(refund_units)?;
    let new_yes_price = lmsr_price_yes(new_q_yes, q_no, b)?;
    let new_no_price = price_no_from_yes(new_yes_price)?;
    Ok((payout_before_fee, new_yes_price, new_no_price))
}

pub fn lmsr_sell_no_to_amount(shares: u64, q_yes: u64, q_no: u64, b: u64) -> Result<(u64, u64, u64)> {
    require!(shares <= q_no, ErrorCode::InsufficientShares);

    let c_before = lmsr_cost_units(q_yes, q_no, b)?;
    let new_q_no = q_no.checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;
    let c_after = lmsr_cost_units(q_yes, new_q_no, b)?;

    let refund_units = c_before - c_after;
    require!(refund_units.is_finite() && refund_units >= 0.0, ErrorCode::MathOverflow);

    let payout_before_fee = f64_units_to_u64_floor(refund_units)?;
    let new_yes_price = lmsr_price_yes(q_yes, new_q_no, b)?;
    let new_no_price = price_no_from_yes(new_yes_price)?;
    Ok((payout_before_fee, new_yes_price, new_no_price))
}

// -----------------------
// LMSR (Multi-choice)
// We store q_i in market.option_volumes[i] (scaled 1e6 shares-outstanding).
// We store spot prices in market.option_prices[i] (scaled 1e6).
// -----------------------

pub fn lmsr_sum_exp_multi(qs: &Vec<u64>, b: u64) -> Result<(f64, Vec<f64>)> {
    require!(b > 0, ErrorCode::MathOverflow);
    let bb = u64_to_f64_units(b);

    let mut exps: Vec<f64> = Vec::with_capacity(qs.len());
    let mut sum = 0.0f64;

    for &q in qs.iter() {
        let qi = u64_to_f64_units(q);
        let e = (clamp_exp(qi / bb)).exp();
        require!(e.is_finite(), ErrorCode::MathOverflow);
        exps.push(e);
        sum += e;
    }
    require!(sum.is_finite() && sum > 0.0, ErrorCode::MathOverflow);
    Ok((sum, exps))
}

pub fn lmsr_prices_multi(qs: &Vec<u64>, b: u64) -> Result<Vec<u64>> {
    let (sum, exps) = lmsr_sum_exp_multi(qs, b)?;
    let mut prices: Vec<u64> = Vec::with_capacity(qs.len());

    // IMPORTANT: we ensure prices sum to PRICE_SCALE by rounding then fixing the remainder.
    let mut total: i64 = 0;
    for e in exps.iter() {
        let p = e / sum;
        let ps = (p * (PRICE_SCALE as f64)).round() as i64;
        total += ps;
        prices.push(ps.max(MIN_PRICE as i64) as u64); // MIN_PRICE guard (optional)
    }

    // Fix sum to exactly PRICE_SCALE (like your prior approach)
    let diff = (PRICE_SCALE as i64) - total;
    if !prices.is_empty() {
        let idx = 0usize;
        let adj = (prices[idx] as i64 + diff)
            .max(MIN_PRICE as i64)
            .min((PRICE_SCALE - MIN_PRICE) as i64) as u64;
        prices[idx] = adj;
    }

    Ok(prices)
}

/// Seed q vector from initial option prices (must roughly sum to 1.0).
/// q_i = b*ln(p_i) + offset, offset chosen so min(q_i)=0.
pub fn lmsr_seed_q_vec_from_initial_option_prices(initial_prices: &Vec<u64>, b: u64) -> Result<Vec<u64>> {
    require!(b > 0, ErrorCode::MathOverflow);
    require!(initial_prices.len() >= 2 && initial_prices.len() <= Market::MAX_OPTIONS, ErrorCode::InvalidOptionsCount);

    // allow small tolerance, but ensure strictly positive
    let mut sum = 0u64;
    for &p in initial_prices.iter() {
        require!(p > 0, ErrorCode::InvalidOptionPrices);
        sum = sum.checked_add(p).ok_or(ErrorCode::MathOverflow)?;
    }
    require!(sum >= 950_000 && sum <= 1_050_000, ErrorCode::InvalidOptionPrices);

    let bb = u64_to_f64_units(b);

    let mut a: Vec<f64> = Vec::with_capacity(initial_prices.len());
    let mut min_a = f64::INFINITY;

    for &p in initial_prices.iter() {
        let pf = (p as f64) / (PRICE_SCALE as f64);
        require!(pf.is_finite() && pf > 0.0, ErrorCode::InvalidOptionPrices);
        let v = bb * pf.ln(); // can be negative
        if v < min_a {
            min_a = v;
        }
        a.push(v);
    }

    let offset = (-min_a).max(0.0);

    let mut qs: Vec<u64> = Vec::with_capacity(initial_prices.len());
    for v in a.into_iter() {
        let q_units = v + offset;
        qs.push(f64_units_to_u64_floor(q_units)?);
    }

    Ok(qs)
}

/// Buy option `idx` with net collateral x_net; returns shares_out and new_prices.
/// Closed form for multi-LMSR when only q_idx changes:
/// Let S = sum exp(q_i/b), ek = exp(q_k/b), ex = exp(x/b)
/// Then exp(dq/b) = 1 + (S/ek)*(ex - 1)
pub fn lmsr_buy_option_from_amount(
    x_net: u64,
    qs: &Vec<u64>,
    idx: usize,
    b: u64,
) -> Result<(u64, Vec<u64>)> {
    require!(idx < qs.len(), ErrorCode::InvalidOptionIndex);
    require!(b > 0, ErrorCode::MathOverflow);

    let x = u64_to_f64_units(x_net);
    let bb = u64_to_f64_units(b);

    let (s, exps) = lmsr_sum_exp_multi(qs, b)?;
    let ek = exps[idx];
    require!(ek.is_finite() && ek > 0.0, ErrorCode::MathOverflow);

    let ex = (clamp_exp(x / bb)).exp();
    require!(ex.is_finite() && ex > 0.0, ErrorCode::MathOverflow);

    let inside = 1.0 + (s / ek) * (ex - 1.0);
    require!(inside.is_finite() && inside > 0.0, ErrorCode::MathOverflow);

    let dq_units = bb * inside.ln();
    let shares_out = f64_units_to_u64_floor(dq_units)?;

    // update q vector
    let mut new_qs = qs.clone();
    new_qs[idx] = new_qs[idx].checked_add(shares_out).ok_or(ErrorCode::MathOverflow)?;

    // compute new prices
    let new_prices = lmsr_prices_multi(&new_qs, b)?;
    Ok((shares_out, new_prices))
}

/// Sell option `idx` shares; returns payout_before_fee and new_prices.
/// Refund = C(q) - C(q - dq) = b*ln(S / S')
/// With only q_k changing: S' = S - ek + ek*exp(-dq/b)
pub fn lmsr_sell_option_to_amount(
    shares: u64,
    qs: &Vec<u64>,
    idx: usize,
    b: u64,
) -> Result<(u64, Vec<u64>)> {
    require!(idx < qs.len(), ErrorCode::InvalidOptionIndex);
    require!(shares <= qs[idx], ErrorCode::InsufficientShares);
    require!(b > 0, ErrorCode::MathOverflow);

    let bb = u64_to_f64_units(b);
    let dq = u64_to_f64_units(shares);

    let (s, exps) = lmsr_sum_exp_multi(qs, b)?;
    let ek = exps[idx];

    let e_neg = (clamp_exp(-dq / bb)).exp(); // exp(-dq/b)
    require!(e_neg.is_finite(), ErrorCode::MathOverflow);

    let s_prime = s - ek + ek * e_neg;
    require!(s_prime.is_finite() && s_prime > 0.0, ErrorCode::MathOverflow);

    let refund_units = bb * (s / s_prime).ln();
    require!(refund_units.is_finite() && refund_units >= 0.0, ErrorCode::MathOverflow);
    let payout_before_fee = f64_units_to_u64_floor(refund_units)?;

    // update q vector
    let mut new_qs = qs.clone();
    new_qs[idx] = new_qs[idx].checked_sub(shares).ok_or(ErrorCode::MathOverflow)?;

    let new_prices = lmsr_prices_multi(&new_qs, b)?;
    Ok((payout_before_fee, new_prices))
}

// Do the buyback swap + burn inside the buy instruction.
// - source_vault: market USDT/USDC vault holding fee_buyback
// - km_vault: market KM ATA receives swap output
// pub fn swap_buyback_and_burn<'info>(
//     market: &Account<'info, Market>,
//     market_id_seed: &[u8; 32],
//     market_bump: u8,
//     amount_in: u64,
//     min_amount_out: u64,
//     // Raydium accounts:
//     ray: &RaydiumSwapAccounts<'info>,
//     // token:
//     km_mint: &Account<'info, Mint>,
//     km_vault: &Account<'info, TokenAccount>,
//     token_program: &Program<'info, Token>,
// ) -> Result<()> {
//     if amount_in == 0 {
//         return Ok(());
//     }

//     // market PDA signer
//     let signer_seeds: &[&[&[u8]]] = &[&[b"market", market_id_seed, &[market_bump]]];

//     // 1) CPI swap fee_buyback stable -> KM (destination is km_vault)
//     // user_token_source is the market stable vault, owned by market PDA
//     // user_source_owner is the market PDA (signer via seeds)
//     let cpi_accounts = raydium_amm_cpi::SwapBaseIn {
//         amm_program: ray.amm_program.to_account_info().into(),
//         amm: ray.amm.to_account_info().into(),
//         amm_authority: ray.amm_authority.to_account_info().into(),
//         amm_open_orders: ray.amm_open_orders.to_account_info().into(),
//         amm_coin_vault: ray.amm_coin_vault.to_account_info().into(),
//         amm_pc_vault: ray.amm_pc_vault.to_account_info().into(),
//         market_program: ray.market_program.to_account_info().into(),
//         market: ray.market.to_account_info().into(),
//         market_bids: ray.market_bids.to_account_info().into(),
//         market_asks: ray.market_asks.to_account_info().into(),
//         market_event_queue: ray.market_event_queue.to_account_info().into(),
//         market_coin_vault: ray.market_coin_vault.to_account_info().into(),
//         market_pc_vault: ray.market_pc_vault.to_account_info().into(),
//         market_vault_signer: ray.market_vault_signer.to_account_info().into(),
//         user_token_source: ray.user_token_source.clone(),
//         user_token_destination: km_vault.clone(),
//         user_source_owner: market.to_account_info().into(),
//         token_program: token_program.clone(),
//     };

//     let cpi_ctx = CpiContext::new_with_signer(
//         token_program.to_account_info(), // NOTE: in a real raydium-cpi call, program is Raydium AMM program, not token_program
//         cpi_accounts,
//         signer_seeds,
//     );

//     // IMPORTANT:
//     // In real code, the CpiContext::new_with_signer "program" should be the Raydium AMM program account,
//     // not SPL token. The raydium-cpi crate usually exposes a `Program<'info, AmmV4>` or UncheckedAccount.
//     // Wire this according to the raydium-cpi crate you import.
//     raydium_amm_cpi::cpi_swap_base_in(cpi_ctx, amount_in, min_amount_out)?;

//     // 2) Burn all KM currently in km_vault (or burn delta if you prefer)
//     let km_received = km_vault.amount; // you may want to compute delta using pre/post balances in practice
//     if km_received > 0 {
//         token::burn(
//             CpiContext::new_with_signer(
//                 token_program.to_account_info(),
//                 Burn {
//                     mint: km_mint.to_account_info(),
//                     from: km_vault.to_account_info(),
//                     authority: market.to_account_info(),
//                 },
//                 signer_seeds,
//             ),
//             km_received,
//         )?;
//     }

//     Ok(())
// }