use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

mod math;

#[derive(PartialEq, Copy, Clone)]
enum Token {
    Eth,
    Dai,
}

fn price_to_tick(price: f64) -> f64 {
    price.log(1.001).floor()
}

fn tick_to_price(tick: i32) -> f64 {
    let base: f64 = 1.001;
    let num: f64 = base.powi(tick);
    num.sqrt() * math::get_q96()
}

fn price_to_sqrtp(price: f64) -> f64 {
    price.sqrt() * math::get_q96()
}

fn liquidity0(amount: f64, pa: f64, pb: f64) -> f64 {
    let q96 = math::get_q96();
    if pa > pb {
        return (amount * (pa * pb)) / q96 / (pb - pa);
    } else {
        return (amount * (pa * pb)) / q96 / (pa - pb);
    }
}

fn liquidity1(amount: f64, pa: f64, pb: f64) -> f64 {
    let q96 = math::get_q96();
    if pa > pb {
        return (amount * q96) / (pb - pa);
    } else {
        return (amount * q96) / (pa - pb);
    }
}

fn calc_amount0(liq: f64, lower_tick: f64, upper_tick: f64) -> f64 {
    let q96 = math::get_q96();
    if upper_tick > lower_tick {
        return (liq * q96 * (upper_tick - lower_tick)) / lower_tick / upper_tick;
    } else {
        return (liq * q96 * (lower_tick - upper_tick)) / upper_tick / lower_tick;
    }
}

fn calc_amount1(liq: f64, lower_tick: f64, upper_tick: f64) -> f64 {
    let q96 = math::get_q96();
    if upper_tick > lower_tick {
        return (liq * (upper_tick - lower_tick)) / q96;
    } else {
        return (liq * (lower_tick - upper_tick)) / q96;
    }
}

fn calc_price_diff(amount_in: f64, liquidity: f64) -> f64 {
    (amount_in * math::get_q96()) / liquidity
}

fn get_next_sqrt_price_from_input(
    sqrt_price_current_x96: f64,
    liquidity: f64,
    amount_remaining: f64,
    zero_for_one: bool
) -> f64 {
    let q96 = math::get_q96();
    if zero_for_one {
        return (
            (liquidity * q96 * sqrt_price_current_x96) /
            (liquidity * q96 + amount_remaining * sqrt_price_current_x96)
        );
    } else {
        return sqrt_price_current_x96 + (amount_remaining * q96) / liquidity;
    }
}

fn compute_swap_step(
    sqrt_price_current_x96: f64,
    sqrt_price_target_x96: f64,
    liquidity: f64,
    amount_remaining: f64
) -> (f64, f64, f64) {
    let zero_for_one = sqrt_price_target_x96 >= sqrt_price_current_x96;

    let amount_in_pre_calc = if zero_for_one {
        calc_amount0(liquidity, sqrt_price_current_x96, sqrt_price_target_x96)
    } else {
        calc_amount1(liquidity, sqrt_price_current_x96, sqrt_price_target_x96)
    };

    let sqrt_price_next_x96: f64;

    if amount_remaining >= amount_in_pre_calc {
        sqrt_price_next_x96 = sqrt_price_target_x96;
    } else {
        sqrt_price_next_x96 = get_next_sqrt_price_from_input(
            sqrt_price_current_x96,
            liquidity,
            amount_remaining,
            zero_for_one
        );
    }

    let amount_in = calc_amonut0(liquidity, sqrt_price_current_x96, sqrt_price_next_x96);

    let amount_out = calc_amount1(liquidity, sqrt_price_current_x96, sqrt_price_next_x96);

    if zero_for_one {
        (sqrt_price_next_x96, amount_in, amount_out)
    } else {
        (sqrt_price_next_x96, amount_out, amount_in)
    }
}
