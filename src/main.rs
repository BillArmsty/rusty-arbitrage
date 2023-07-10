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
    Dai
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

fn liquidity0(amount: f64, pa: f64, p)