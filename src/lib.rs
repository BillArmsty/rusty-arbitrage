#[cfg(test)]
mod tests {
    use super::*;

    fn set_up_pool(
        mint: bool,
        lower_tick: i32,
        upper_tick: i32,
        liquidity: f64
    ) -> (Trader, uniswap_v3_pool) {
        let trader = Trader {
            id: 2,
            amt_eth: RwLock::new(10000000000.0),
            amt_dai: RwLock::new(10000000000.0),
        };
        let mut pool = uniswap_v3_pool {
            liquidity: RwLock::new(0.0),
            max_tick: math::get_max_tick(),
            min_tick: math::get_min_tick(),
            position_mapping: RwLock::new(HashMap::new()),
            tick_mapping: RwLock::new(HashMap::new()),
            liquidity_mapping: RwLock::new(HashMap::new()),
            sqrt_price_x96: RwLock::new(5602277097478614198912276234240.0),
            tick: RwLock::new(85176),
            token_0: Token::Eth,
            token_1: Token::Dai,
            balance_0: RwLock::new(0.0),
            balance_1: RwLock::new(0.0),
        };
        if mint {
            pool.mint(&trader, lower_tick, upper_tick, liquidity);
        }

        (trader, pool)
    }

    #[test]
    fn price_to_sqrt_price() {
        assert_eq!(price_to_sqrtp(5000.0), 5.602277097478614e30);
    }

    #[test]
    fn v3_test_mint() {
        let trader = Trader {
            id: 2,
            amt_eth: RwLock::new(2000.0),
            amt_dai: RwLock::new(10000.0),
        };
        let mut pool = uniswap_v3_pool {
            liquidity: RwLock::new(0.0),
            max_tick: math::get_max_tick(),
            min_tick: math::get_min_tick(),
            position_mapping: RwLock::new(HashMap::new()),
            tick_mapping: RwLock::new(HashMap::new()),
            liquidity_mapping: RwLock::new(HashMap::new()),
            sqrt_price_x96: RwLock::new(5602277097478614198912276234240.0),
            tick: RwLock::new(85176),
            token_0: Token::Eth,
            token_1: Token::Dai,
            balance_0: RwLock::new(0.0),
            balance_1: RwLock::new(0.0),
        };

        pool.mint(&trader, 84222, 86129, 1517882343751509868544.0);

        assert_eq!(*pool.sqrt_price_x96.read().unwrap(), 5602277097478614198912276234240.0);
    }
    #[test]
    fn v3_test_remove() {
        let trader = Trader {
            id: 2,
            amt_eth: RwLock::new(2000.0),
            amt_dai: RwLock::new(10000.0),
        };
        let mut pool = uniswap_v3_pool {
            liquidity: RwLock::new(0.0),
            max_tick: math::get_max_tick(),
            min_tick: math::get_min_tick(),
            position_mapping: RwLock::new(HashMap::new()),
            tick_mapping: RwLock::new(HashMap::new()),
            liquidity_mapping: RwLock::new(HashMap::new()),
            sqrt_price_x96: RwLock::new(5602277097478614198912276234240.0),
            tick: RwLock::new(85176),
            token_0: Token::Eth,
            token_1: Token::Dai,
            balance_0: RwLock::new(0.0),
            balance_1: RwLock::new(0.0),
        };

        pool.mint(&trader, 84222, 86129, 1517882343751509868544.0);

        let liq = *pool.liquidity.read().unwrap();

        assert_eq!(liq, 1517882343751509868544.0);

        pool.mint(&trader, 84222, 86129, -1517882343751509868544.0);

        assert_eq!(*pool.sqrt_price_x96.read().unwrap(), 5602277097478614198912276234240.0);
        let new_liquidity = *pool.liquidity.read().unwrap();
        assert_eq!(new_liquidity, 0.0)
    }

    #[test]
    fn test_swap_eth() {
        let (mut trader, pool) = set_up_pool(true, -86000, 86000, 100000000000.0);
        let original = *trader.amt_eth.read().unwrap();
        let og_dai = *trader.amt_dai.read().unwrap();

        v3_swap(&mut trader, &pool, Token::Eth, 1000000.0, 0.03);

        let post = *trader.amt_eth.read().unwrap();
        let post_dai = *trader.amt_dai.read().unwrap();

        assert_eq!(original > post, true);
        assert_eq!(post_dai > og_dai, true);
    }

    #[test]
    fn test_swap_dai() {
        let (mut trader, pool) = set_up_pool(true, -86000, 86000, 10000000000000.0);
        let original = *trader.amt_eth.read().unwrap();
        let og_dai = *trader.amt_dai.read().unwrap();

        v3_swap(&mut trader, &pool, Token::Dai, 100.0, 0.03);

        let post = *trader.amt_eth.read().unwrap();
        let post_dai = *trader.amt_dai.read().unwrap();

        assert_eq!(original < post, true);
        assert_eq!(post_dai < og_dai, true);
    }

    #[test]
    fn benchmark_search_for_arb() {
        main()
    }
}
