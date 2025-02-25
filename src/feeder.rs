use crate::{
    ray::{fetch_pool_info_by_id, PoolId},
    token_registry::Token,
};

#[derive(Default, Debug, Clone)]
pub struct PriceInfo {
    pub price: Option<f64>,
    pub ui_price: String,
    pub updated_at: u64,
}

#[derive(Default, Debug, Clone)]
pub struct TokenPriceInfo {
    pub token: Token,
    pub price_info: PriceInfo,
}

#[derive(Default, Debug, Clone)]
pub struct PairPriceInfo {
    pub token_a: Token,
    pub token_b: Token,
    pub price_info: PriceInfo,
}

#[derive(Default, Debug, Clone)]
pub struct PerpValueInfo {
    // e.g. SOL_PERPS
    pub id: String,
    pub token: Token,
    // TODO: we need better name, e.g. ValueUsdInfo.
    pub pnl_after_fees_usd: PriceInfo,
}

#[derive(Debug, Clone)]
pub enum TokenOrPairPriceInfo {
    Pair(PairPriceInfo),
    Token(TokenPriceInfo),
    Perp(PerpValueInfo),
}

pub type TokenOrPairAddress = String;

pub async fn get_price_by_token_id(pool_id: PoolId) -> anyhow::Result<f64> {
    let pool_info = fetch_pool_info_by_id(pool_id).await?;

    // Get price from pool that match id
    let price = pool_info.price;

    Ok(price)
}
