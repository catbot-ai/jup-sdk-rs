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

#[derive(Serialize, Deserialize, Debug)]
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
    Side::from_str(&s).map_err(|_| serde::de::Error::custom(format!("Invalid side value: {}", s)))
}

#[derive(Serialize, Deserialize, Debug, Default)] // Added Default for cases where tp or sl are null
#[serde(rename_all = "camelCase")]
pub struct TpslRequests {
    pub tp: Option<TpslRequest>, // Option to handle null values
    pub sl: Option<TpslRequest>, // Option to handle null values
}

#[derive(Serialize, Deserialize, Debug, Default)] // Added Default for TpslRequest as well, though might not be strictly necessary
#[serde(rename_all = "camelCase")]
pub struct TpslRequest {
    // Define struct for tp/sl if they are not always null and have a structure
    // Add fields for tp and sl requests if they are not always null and have a specific structure
    // Based on the example, they are null, so for now, an empty struct or just Option<TpslRequest> is sufficient
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
            "{}/positions?walletAddress={}",
            PERPS_API_BASE, wallet_address
        );
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            let positions_response: PositionsResponse = response.json().await?;
            Ok(positions_response)
        } else {
            Err(anyhow!(
                "Failed to fetch positions. Status: {}",
                response.status()
            ))
        }
    }

    pub async fn fetch_positions_pnl_and_format(
        &self,
        wallet_address: &str,
    ) -> Result<PositionPNLs> {
        let positions_response = self.fetch_positions(wallet_address).await?;
        let mut total_pnl_usd = 0.0;
        let mut total_pnl_percent = 0.0;
        let mut position_pnls: Vec<PositionPNL> = Vec::new();

        for position in positions_response.data_list {
            let pnl_usd = position.pnl_after_fees_usd.parse::<f64>().map_err(|_| {
                anyhow!(
                    "Failed to parse pnl_after_fees_usd to f64: {}",
                    position.pnl_after_fees_usd
                )
            })?;

            let pnl_percent = position
                .pnl_change_pct_after_fees
                .parse::<f64>()
                .map_err(|_| {
                    anyhow!(
                        "Failed to parse pnl_change_pct_after_fees to f64: {}",
                        position.pnl_change_pct_after_fees
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
        let perps_fetcher = PerpsFetcher::default();
        let wallet_address = "...";

        println!("Fetching positions for wallet: {}", wallet_address);
        let positions_result = perps_fetcher.fetch_positions(wallet_address).await;

        match positions_result {
            Ok(positions) => {
                println!("Successfully fetched positions:");
                println!("{:#?}", positions); // Using pretty print for better readability
                if let Some(first_position) = positions.data_list.first() {
                    println!("\nDetails of the first position:");
                    println!("  Position Pubkey: {}", first_position.position_pubkey);
                    println!("  Side: {}", first_position.side);
                    println!("  Size: {}", first_position.size);
                    println!(
                        "  Pnl After Fees Usd: {}",
                        first_position.pnl_after_fees_usd
                    );
                    println!("  Collateral: {}", first_position.collateral);
                    println!("  Leverage: {}", first_position.leverage);
                    println!("  Entry Price: {}", first_position.entry_price);
                    println!("  Liquidation Price: {}", first_position.liquidation_price);
                    println!("  Total Fees Usd: {}", first_position.total_fees_usd);
                    println!("  Open Fees Usd: {}", first_position.open_fees_usd);
                    println!("  Close Fees Usd: {}", first_position.close_fees_usd);
                    println!("  Borrow Fees Usd: {}", first_position.borrow_fees_usd);
                    println!("  Created Time: {}", first_position.created_time);
                    println!("  Updated Time: {}", first_position.updated_time);

                    if let Some(tp_request) = &first_position.tpsl_requests.tp {
                        println!("  Take Profit Request: {:?}", tp_request);
                    } else {
                        println!("  Take Profit Request: None");
                    }

                    if let Some(sl_request) = &first_position.tpsl_requests.sl {
                        println!("  Stop Loss Request: {:?}", sl_request);
                    } else {
                        println!("  Stop Loss Request: None");
                    }
                } else {
                    println!("No positions found in the response.");
                }
            }
            Err(e) => {
                eprintln!("Error fetching positions: {:?}", e);
            }
        }

        Ok(())
    }
}
