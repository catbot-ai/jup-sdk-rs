use crate::fetcher::Fetcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString}; // Import Fetcher

#[derive(EnumString, Display, Debug, Clone)]
pub enum PoolId {
    #[strum(to_string = "3d8ksMPuLpaQAUbuRr74tmovmyFFXgAsC3iE5NhsgvnH")]
    #[allow(non_camel_case_types)]
    SOL_JLP,
}

pub const RAYDIUM_BASE_API: &str = "https://api-v3.raydium.io";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolInfoResponse {
    pub id: String,
    pub success: bool,
    pub data: Vec<PoolData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolData {
    pub r#type: String,
    pub program_id: String,
    pub id: String,
    pub mint_a: Mint,
    pub mint_b: Mint,
    pub reward_default_pool_infos: String,
    pub reward_default_infos: Vec<String>,
    pub price: f64,
    pub mint_amount_a: f64,
    pub mint_amount_b: f64,
    pub fee_rate: f64,
    pub open_time: String,
    pub tvl: f64,
    pub day: TimeFrameData,
    pub week: TimeFrameData,
    pub month: TimeFrameData,
    pub pooltype: Vec<String>,
    pub farm_upcoming_count: u32,
    pub farm_ongoing_count: u32,
    pub farm_finished_count: u32,
    pub settings: Option<Settings>,
    pub burn_percent: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Mint {
    pub chain_id: u32,
    pub address: String,
    pub program_id: String,
    #[serde(rename = "logoURI")]
    pub logo_uri: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub tags: Vec<String>,
    pub extensions: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimeFrameData {
    pub volume: f64,
    pub volume_quote: f64,
    pub volume_fee: f64,
    pub apr: f64,
    pub fee_apr: f64,
    pub price_min: f64,
    pub price_max: f64,
    pub reward_apr: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub id: String,
    pub index: u32,
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub tick_spacing: u32,
    pub fund_fee_rate: u32,
    pub default_range: f64,
    pub default_range_point: Vec<f64>,
}

// Use the Fetcher for consistency and retry logic
async fn fetch_pool_info_internal(url: &str) -> anyhow::Result<PoolInfoResponse> {
    // Create a default fetcher instance. Consider passing it if needed elsewhere.
    let fetcher = Fetcher::default();
    fetcher.fetch_with_retry::<PoolInfoResponse>(url).await
}

// Public function name remains the same
#[allow(dead_code)]
pub async fn fetch_pool_info_by_id(id: PoolId) -> anyhow::Result<PoolData> {
    let url = format!("{RAYDIUM_BASE_API}/pools/info/ids?ids={id}");
    let pool_info = fetch_pool_info_internal(&url).await?;

    // Assuming the API always returns data if successful and the ID exists
    // Need to handle potential empty data list if ID not found
    pool_info
        .data
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Pool data not found for ID: {}", id))
}

#[allow(dead_code)]
pub fn get_token_logo_url_by_mint_address(mint_address: &str) -> String {
    format!("https://img.raydium.io/icon/{mint_address}.png")
}

#[cfg(all(test, feature = "native"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{prices::MainTokenSymbol, ray, token_registry::TokenRegistry};

    // Requires tokio runtime, so only build/run with 'native' feature
    #[tokio::test]
    async fn test_fetch_pool_info_by_id() {
        let id = ray::PoolId::SOL_JLP;
        let pool_info_result = fetch_pool_info_by_id(id).await;

        // Result
        println!("{pool_info_result:#?}");

        // Assert result is Ok before unwrapping
        assert!(pool_info_result.is_ok());

        // Get price from pool that match id
        let price = pool_info_result.unwrap().price;

        println!("{price:#?}");
        assert!(price > 0.0);
    }

    #[test]
    fn test_get_logo_by_mint_address() {
        let registry = TokenRegistry::new();
        let usdc_token = registry.get_by_symbol(&MainTokenSymbol::USDC).unwrap();
        let mint_address = usdc_token.address.clone();

        let logo = get_token_logo_url_by_mint_address(&mint_address);

        assert_eq!(
            logo,
            "https://img.raydium.io/icon/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v.png"
        )
    }
}
