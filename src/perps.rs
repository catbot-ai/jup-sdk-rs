use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;
use strum_macros::Display;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PositionsResponse {
    pub count: i32,
    pub data_list: Vec<PositionData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PositionData {
    pub borrow_fees: String,
    pub borrow_fees_usd: String,
    pub close_fees: String,
    pub close_fees_usd: String,
    pub collateral: String,
    pub collateral_mint: String,
    pub created_time: i64,
    pub entry_price: String,
    pub leverage: String,
    pub liquidation_price: String,
    pub market_mint: String,
    pub open_fees: String,
    pub open_fees_usd: String,
    pub pnl_after_fees: String,
    pub pnl_after_fees_usd: String,
    pub pnl_before_fees: String,
    pub pnl_before_fees_usd: String,
    pub pnl_change_pct_after_fees: String,
    pub pnl_change_pct_before_fees: String,
    pub position_pubkey: String,
    #[serde(deserialize_with = "deserialize_side")]
    pub side: Side,
    pub size: String,
    pub size_token_amount: String,
    pub total_fees: String,
    pub total_fees_usd: String,
    pub tpsl_requests: TpslRequests,
    pub updated_time: i64,
    pub value: String,
}

fn deserialize_side<'de, D>(deserializer: D) -> Result<Side, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Side::from_str(&s).map_err(|_| serde::de::Error::custom(format!("Invalid side: {}", s)))
}

// Ensure camelCase naming aligns with JSON
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TpslRequests {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tp: Option<TpslRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sl: Option<TpslRequest>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TpslRequest {
    pub desired_mint: String,
    pub position_request_pubkey: String,
    pub trigger_price: String,
    pub trigger_price_usd: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PositionPNLs {
    pub total_pnl_usd: f64,
    pub total_pnl_percent: f64,
    pub position_pnls: Vec<PositionPNL>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PositionPNL {
    pub position_pubkey: String,
    pub side: Side,
    pub pnl_usd: f64,
    pub pnl_percent: f64,
}

#[derive(Clone, Serialize, Deserialize, Debug, EnumString, Display, PartialEq)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Side {
    Long,
    Short,
}

const PERPS_API_BASE: &str = "https://perps-api.jup.ag/v1";

pub struct PerpsFetcher {
    client: Client,
}

impl Default for PerpsFetcher {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl PerpsFetcher {
    pub async fn fetch_positions(&self, wallet_address: &str) -> Result<PositionsResponse> {
        let url = format!(
            "{}/positions?walletAddress={}&showTpslRequests=true",
            PERPS_API_BASE, wallet_address
        );

        let response = self.client.get(&url).send().await?;
        response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to fetch positions: {}", e))
    }

    pub async fn fetch_positions_pnl_and_format(
        &self,
        wallet_address: &str,
    ) -> Result<PositionPNLs> {
        let positions_response = self.fetch_positions(wallet_address).await?;
        let mut total_pnl_usd = 0.0;
        let mut total_pnl_percent = 0.0;
        let mut position_pnls = Vec::new();

        for position in positions_response.data_list {
            let pnl_usd = position.pnl_after_fees_usd.parse::<f64>().map_err(|e| {
                anyhow!(
                    "Failed to parse pnl_after_fees_usd '{}': {}",
                    position.pnl_after_fees_usd,
                    e
                )
            })?;
            let pnl_percent = position
                .pnl_change_pct_after_fees
                .parse::<f64>()
                .map_err(|e| {
                    anyhow!(
                        "Failed to parse pnl_change_pct_after_fees '{}': {}",
                        position.pnl_change_pct_after_fees,
                        e
                    )
                })?;

            total_pnl_usd += pnl_usd;
            total_pnl_percent += pnl_percent;
            position_pnls.push(PositionPNL {
                position_pubkey: position.position_pubkey.clone(),
                side: position.side.clone(),
                pnl_usd,
                pnl_percent,
            });
        }

        Ok(PositionPNLs {
            total_pnl_usd,
            total_pnl_percent,
            position_pnls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_positions() -> Result<()> {
        dotenvy::from_filename(".env").expect("No .env file");
        let wallet_address = std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS not set");

        let perps_fetcher = PerpsFetcher::default();
        let positions = perps_fetcher.fetch_positions(&wallet_address).await?;
        println!("Fetched {} positions:", positions.count);
        if let Some(pos) = positions.data_list.first() {
            println!(
                "First position: {} ({}), PNL: ${}, TP: {:?}, SL: {:?}",
                pos.position_pubkey,
                pos.side,
                pos.pnl_after_fees_usd,
                pos.tpsl_requests.tp,
                pos.tpsl_requests.sl
            );
            // Debug raw tpsl_requests
            println!("Raw tpsl_requests: {:?}", pos.tpsl_requests);
        } else {
            println!("No positions found.");
        }
        Ok(())
    }
}
