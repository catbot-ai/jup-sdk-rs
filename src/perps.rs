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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TpslRequests {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tp: Option<TpslRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sl: Option<TpslRequest>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct PerpsPosition {
    pub side: Side,                // Position side: Long or Short
    pub market_mint: String,       // So11111111111111111111111111111111111111112
    pub collateral_mint: String,   // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
    pub entry_price: f64,          // Entry price of the position
    pub leverage: f64,             // Leverage used for the position
    pub liquidation_price: f64,    // Liquidation price of the position
    pub pnl_after_fees_usd: f64,   // Profit/loss after fees in USD
    pub value: f64,                // Current position value in USD
    pub target_price: Option<f64>, // Current target price in USD
    pub stop_loss: Option<f64>,    // Current stop loss in USD
}

impl From<PositionData> for PerpsPosition {
    fn from(position: PositionData) -> Self {
        let entry_price = position.entry_price.parse().unwrap_or(0.0);
        let leverage = position.leverage.parse().unwrap_or(1.1); // 1x leverage as fallback
        let liquidation_price = position.liquidation_price.parse().unwrap_or(0.0);
        let pnl_after_fees_usd = position.pnl_after_fees_usd.parse().unwrap_or(0.0);
        let value = position.value.parse().unwrap_or(0.0);
        let target_price = position
            .tpsl_requests
            .tp
            .unwrap_or_default()
            .trigger_price_usd
            .parse()
            .ok();
        let stop_loss = position
            .tpsl_requests
            .sl
            .unwrap_or_default()
            .trigger_price_usd
            .parse()
            .ok();

        PerpsPosition {
            side: position.side,
            market_mint: position.market_mint,
            collateral_mint: position.collateral_mint,
            entry_price,
            leverage,
            liquidation_price,
            pnl_after_fees_usd,
            value,
            target_price,
            stop_loss,
        }
    }
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

    // New method to fetch and convert to PerpsPosition
    pub async fn fetch_perps_positions(&self, wallet_address: &str) -> Result<Vec<PerpsPosition>> {
        let positions_response = self.fetch_positions(wallet_address).await?;
        Ok(positions_response
            .data_list
            .into_iter()
            .map(PerpsPosition::from)
            .collect())
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
        println!("Fetched {:#?} positions:", positions);
        if let Some(pos) = positions.data_list.first() {
            println!(
                "First position: {} {}x, PNL: {}, TP: {:?}, SL: {:?}",
                pos.side,
                pos.leverage,
                pos.pnl_after_fees_usd,
                pos.tpsl_requests.tp,
                pos.tpsl_requests.sl
            );
            println!("Raw tpsl_requests: {:?}", pos.tpsl_requests);
        } else {
            println!("No positions found.");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_perps_positions() -> Result<()> {
        dotenvy::from_filename(".env").expect("No .env file");
        let wallet_address = std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS not set");

        let perps_fetcher = PerpsFetcher::default();
        let perps_positions = perps_fetcher.fetch_perps_positions(&wallet_address).await?;
        println!("Fetched {} perps positions:", perps_positions.len());

        if let Some(pos) = perps_positions.first() {
            println!("{:#?}", perps_positions);
            println!(
                "First position: {} {}x, PNL: {}, TP: {:?}, SL: {:?}",
                pos.side, pos.leverage, pos.entry_price, pos.pnl_after_fees_usd, pos.value
            );
        } else {
            println!("No perps positions found.");
        }
        Ok(())
    }
}
