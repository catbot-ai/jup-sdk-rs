use super::fetcher::{Fetcher, RetrySettings};
use anyhow::{anyhow, Result};
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
    // Use the generic Fetcher
    fetcher: Fetcher,
}

impl PerpsFetcher {
    /// Creates a new PerpsFetcher with default retry settings.
    pub fn new() -> Self {
        Self {
            fetcher: Fetcher::new(), // Or Fetcher::default()
        }
    }

    /// Creates a new PerpsFetcher with custom retry settings.
    pub fn with_settings(settings: RetrySettings) -> Self {
        Self {
            fetcher: Fetcher::with_settings(settings),
        }
    }

    /// Fetches positions from the Jupiter Perps API with retry logic.
    pub async fn fetch_positions(&self, wallet_address: &str) -> Result<PositionsResponse> {
        let url = format!(
            "{}/positions?walletAddress={}&showTpslRequests=true",
            PERPS_API_BASE, wallet_address
        );

        // Use the fetcher's fetch_with_retry method
        self.fetcher
            .fetch_with_retry::<PositionsResponse>(&url)
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to fetch positions for wallet {}: {}",
                    wallet_address,
                    e
                )
            })
    }

    /// Fetches positions, calculates aggregate PNL, and formats the result.
    pub async fn fetch_positions_pnl_and_format(
        &self,
        wallet_address: &str,
    ) -> Result<PositionPNLs> {
        // This now uses the fetch_positions method which includes retries
        let positions_response = self.fetch_positions(wallet_address).await?;

        let mut total_pnl_usd = 0.0;
        let mut total_value = 0.0; // Needed to calculate weighted average PNL percentage
        let mut position_pnls = Vec::new();

        for position in positions_response.data_list {
            let pnl_usd = position.pnl_after_fees_usd.parse::<f64>().map_err(|e| {
                anyhow!(
                    "Failed to parse pnl_after_fees_usd '{}' for position {}: {}",
                    position.pnl_after_fees_usd,
                    position.position_pubkey,
                    e
                )
            })?;
            let pnl_percent = position
                .pnl_change_pct_after_fees
                .parse::<f64>()
                .map_err(|e| {
                    anyhow!(
                        "Failed to parse pnl_change_pct_after_fees '{}' for position {}: {}",
                        position.pnl_change_pct_after_fees,
                        position.position_pubkey,
                        e
                    )
                })?;
            // Parse value for weighted percentage calculation
            let value_usd = position.value.parse::<f64>().map_err(|e| {
                anyhow!(
                    "Failed to parse value '{}' for position {}: {}",
                    position.value,
                    position.position_pubkey,
                    e
                )
            })?;

            total_pnl_usd += pnl_usd;
            // Accumulate value for weighted average calculation
            if value_usd > 0.0 {
                // Avoid division by zero or issues with negative value if possible
                total_value += value_usd;
            }

            position_pnls.push(PositionPNL {
                position_pubkey: position.position_pubkey.clone(),
                side: position.side.clone(),
                pnl_usd,
                pnl_percent, // Keep individual percent if needed
            });
        }

        // Calculate a weighted average PNL percentage if total value is positive
        let total_pnl_percent_avg = if total_value > 0.0 {
            // Calculate weighted average: sum(pnl_usd_i) / sum(value_usd_i) * 100
            // Or approximate by (total_pnl_usd / (total_value - total_pnl_usd)) * 100 if 'value' is collateral+pnl
            // Simpler: total_pnl_usd / total_value * 100 if 'value' is current total value including PNL
            // The direct sum `total_pnl_percent` is usually not meaningful. Let's calculate a weighted average.
            (total_pnl_usd / total_value) * 100.0 // Assuming 'value' is the total current value
        } else {
            0.0 // Avoid division by zero
        };

        Ok(PositionPNLs {
            total_pnl_usd,
            // Use the calculated weighted average percentage instead of summing percentages
            total_pnl_percent: total_pnl_percent_avg,
            position_pnls,
        })
    }

    /// Fetches positions and converts them into a simplified `PerpsPosition` format.
    pub async fn fetch_perps_positions(&self, wallet_address: &str) -> Result<Vec<PerpsPosition>> {
        // This now uses the fetch_positions method which includes retries
        let positions_response = self.fetch_positions(wallet_address).await?;
        Ok(positions_response
            .data_list
            .into_iter()
            .map(PerpsPosition::from) // Note: Consider handling potential errors from `from` if it were changed to return Result
            .collect())
    }
}

// Implement Default for PerpsFetcher for convenience
impl Default for PerpsFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // Only run tests if native feature is enabled
    #![cfg(all(test, feature = "native"))]

    use super::*;
    use std::time::Duration;

    fn setup() {
        // Run `export RUST_LOG=warn` or similar before running tests to see logs
        // Requires the `env_logger` and `log` features to be enabled for the test build
        #[cfg(feature = "env_logger")]
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_fetch_positions_with_retry() -> Result<()> {
        setup();
        // dotenvy requires the 'native' feature
        dotenvy::from_filename(".env").ok(); // Use .ok() to not panic if .env is missing
        let wallet_address =
            std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS not set in .env");

        // Use default settings which include retries
        let perps_fetcher = PerpsFetcher::default();
        println!("Fetching positions for {} with retry...", wallet_address);

        let positions = perps_fetcher.fetch_positions(&wallet_address).await?;
        println!(
            "Fetched {} positions for wallet {}",
            positions.count, wallet_address
        );
        // Add assertions or more detailed checks if needed
        assert!(positions.count >= 0); // Basic check

        if let Some(pos) = positions.data_list.first() {
            println!("First position details: {:#?}", pos);
            println!(
                "First position summary: {} {}x, PNL: {}, TP: {:?}, SL: {:?}",
                pos.side,
                pos.leverage,
                pos.pnl_after_fees_usd,
                pos.tpsl_requests.tp.as_ref().map(|r| &r.trigger_price_usd),
                pos.tpsl_requests.sl.as_ref().map(|r| &r.trigger_price_usd)
            );
        } else {
            println!("No positions found for this wallet.");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_perps_positions_conversion() -> Result<()> {
        setup();
        // dotenvy requires the 'native' feature
        dotenvy::from_filename(".env").ok();
        let wallet_address =
            std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS not set in .env");

        let perps_fetcher = PerpsFetcher::default();
        println!(
            "Fetching and converting PerpsPositions for {}...",
            wallet_address
        );

        let perps_positions = perps_fetcher.fetch_perps_positions(&wallet_address).await?;
        println!("Fetched {} PerpsPosition(s):", perps_positions.len());

        if let Some(pos) = perps_positions.first() {
            println!("First PerpsPosition: {:#?}", pos);
            assert!(pos.leverage >= 1.0); // Leverage should be >= 1
                                          // Add more assertions based on expected data structure
        } else {
            println!("No perps positions found to convert.");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_positions_pnl_format() -> Result<()> {
        setup();
        // dotenvy requires the 'native' feature
        dotenvy::from_filename(".env").ok();
        let wallet_address =
            std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS not set in .env");

        let perps_fetcher = PerpsFetcher::default();
        println!("Fetching PNL summary for {}...", wallet_address);

        let pnl_summary = perps_fetcher
            .fetch_positions_pnl_and_format(&wallet_address)
            .await?;
        println!("PNL Summary: {:#?}", pnl_summary);

        // Basic assertion: Check if total PNL is a valid number
        assert!(pnl_summary.total_pnl_usd.is_finite());
        assert!(pnl_summary.total_pnl_percent.is_finite());
        assert_eq!(
            pnl_summary.position_pnls.len(),
            perps_fetcher
                .fetch_positions(&wallet_address)
                .await?
                .data_list
                .len()
        );

        Ok(())
    }

    // Optional: Test with custom settings (e.g., shorter timeout to force failure/retry)
    #[tokio::test]
    async fn test_fetch_with_custom_settings_timeout() {
        setup();
        // dotenvy requires the 'native' feature
        dotenvy::from_filename(".env").ok();
        let wallet_address =
            std::env::var("WALLET_ADDRESS").expect("WALLET_ADDRESS not set in .env");

        // Configure extremely short timeout to likely trigger retries or failure
        let settings = RetrySettings::default()
            .with_request_timeout(Duration::from_millis(10)) // Very short timeout
            .with_max_retries(2); // Limit retries

        let perps_fetcher = PerpsFetcher::with_settings(settings);
        println!(
            "Fetching positions for {} with short timeout...",
            wallet_address
        );

        let result = perps_fetcher.fetch_positions(&wallet_address).await;

        // Expecting an error due to timeout
        assert!(result.is_err());
        if let Err(e) = result {
            println!("Received expected error: {}", e);
            // Check for timeout message OR the retry limit message
            assert!(
                e.to_string().contains("timed out") || e.to_string().contains("after 2 retries")
            );
        }
    }
}
