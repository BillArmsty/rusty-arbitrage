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

    let amount_in = calc_amount0(liquidity, sqrt_price_current_x96, sqrt_price_next_x96);

    let amount_out = calc_amount1(liquidity, sqrt_price_current_x96, sqrt_price_next_x96);

    if zero_for_one {
        (sqrt_price_next_x96, amount_in, amount_out)
    } else {
        (sqrt_price_next_x96, amount_out, amount_in)
    }
}

struct Tick {
    liquidity: RwLock<f64>,
    initialized: RwLock<bool>,
}

struct Position {
    liquidity: RwLock<f64>,
}

struct uniswap_v3_pool {
    token_0: Token,
    token_1: Token,
    min_tick: i32,
    max_tick: i32,
    balance_0: RwLock<f64>,
    balance_1: RwLock<f64>,
    tick_mapping: RwLock<HashMap<i32, Tick>>,
    liquidity_mapping: RwLock<HashMap<i32, f64>>,
    position_mapping: RwLock<HashMap<i32, Position>>,
    sqrt_price_x96: RwLock<f64>,
    tick: RwLock<i32>,
    liquidity: RwLock<f64>,
}

impl uniswap_v3_pool {
    fn update(&mut self, tick: i32, liquidity_delta: f64) -> bool {
        let default_tick = Tick {
            liquidity: RwLock::new(0.0),
            initialized: RwLock::new(false),
        };

        let tick_map = &mut self.tick_mapping.write().unwrap();

        let info = tick_map.entry(tick).or_insert(default_tick);

        let liquidity_before = *info.liquidity.read().unwrap();

        let liquidity_after = liquidity_before + liquidity_delta;

        if liquidity_before == 0.0 {
            *info.initialized.write().unwrap() = true;
            self.liquidity_mapping.write().unwrap().insert(tick, liquidity_after);
        }

        *info.liquidity.write().unwrap() = liquidity_after;

        let flipped = (liquidity_after == 0.0) != (liquidity_before == 0.0);

        flipped
    }

    fn _update_position(
        &mut self,
        owner: &Trader,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: f64
    ) {
        let flipped_lower = self.update(lower_tick, liquidity_delta);
        let flipped_upper = self.update(upper_tick, liquidity_delta);

        if flipped_lower {
            self.liquidity_mapping.write().unwrap().insert(lower_tick, 1.0);
        }
        if flipped_upper {
            self.liquidity_mapping.write().unwrap().insert(upper_tick, 1.0);
        }

        let default_position = Position {
            liquidity: RwLock::new(0.0),
        };

        let position_map = &mut self.position_mapping.write().unwrap();

        let position = position_map.entry(owner.id).or_insert(default_position);

        *position.liquidity.write().unwrap() += liquidity_delta;

        if liquidity_delta < 0.0 {
            if flipped_lower {
                self.liquidity_mapping.write().unwrap().remove(&lower_tick);
            }
            if flipped_upper {
                self.liquidity_mapping.write().unwrap().remove(&upper_tick);
            }
        }
    }

    fn _modify_position(
        &mut self,
        owner: &Trader,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: f64
    ) -> (f64, f64) {
        let mut amount0: f64 = 0.0;
        let mut amount1: f64 = 0.0;
        let sqrt_price_x96 = *self.sqrt_price_x96.read().unwrap();
        let tick = *self.tick.read().unwrap();
        self._update_position(owner, lower_tick, upper_tick, liquidity_delta);
        if liquidity_delta != 0.0 {
            if tick < lower_tick {
                amount0 = calc_amount0(
                    liquidity_delta,
                    tick_to_price(lower_tick),
                    tick_to_price(upper_tick)
                );
            } else if tick < upper_tick {
                amount0 = calc_amount0(liquidity_delta, sqrt_price_x96, tick_to_price(upper_tick));

                amount1 = calc_amount1(liquidity_delta, tick_to_price(lower_tick), sqrt_price_x96);
                *self.liquidity.write().unwrap() += liquidity_delta;
            } else {
                amount1 = calc_amount1(
                    liquidity_delta,
                    tick_to_price(lower_tick),
                    tick_to_price(upper_tick)
                );
            }
        }
        (amount0, amount1)
    }

    fn mint(&mut self, owner: &Trader, lower_tick: i32, upper_tick: i32, liquidity_delta: f64) {
        if !(lower_tick >= upper_tick || lower_tick < self.min_tick || upper_tick > self.max_tick)
        & liquidity_delta != 0.
        {
            let (amount0, amount1) = self._modify_position(owner, lower_tick, upper_tick, liquidity_delta);
            if amount0 > 0. {
                *self.balance_0.write().unwrap() += amount0;
            }
            if amount1 > 0. {
                *self.balance_1.write().unwrap() += amount1;
            }

            if self.token_0 == Token::Eth {
                *owner.amt_eth.write().unwrap() -= amount0;
                *owner.amt_dai.write().unwrap() -= amount1;
            } else {
                *owner.amt_eth.write().unwrap() -= amount1;
                *owner.amt_dai.write().unwrap() -= amount0;
            }
        }
    }
}
