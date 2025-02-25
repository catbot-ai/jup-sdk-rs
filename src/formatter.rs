use currency_rs::{Currency, CurrencyOpts};

use crate::feeder::{PairPriceInfo, PerpValueInfo, TokenOrPairPriceInfo, TokenPriceInfo};

pub fn get_label_and_ui_price(price_info: &TokenOrPairPriceInfo) -> (String, String) {
    match price_info {
        TokenOrPairPriceInfo::Pair(PairPriceInfo {
            token_a,
            token_b,
            price_info,
        }) => {
            let label = format!("{}/{}", token_a.symbol, token_b.symbol);
            let ui_price = price_info
                .price
                .map(format_price)
                .unwrap_or("…".to_string());
            (label, ui_price)
        }
        TokenOrPairPriceInfo::Token(TokenPriceInfo { token, price_info }) => {
            let label = token.symbol.to_string();
            let ui_price = price_info
                .price
                .map(format_price_with_dollar)
                .unwrap_or("…".to_string());
            (label, ui_price)
        }
        TokenOrPairPriceInfo::Perp(PerpValueInfo {
            id: _,
            token,
            pnl_after_fees_usd,
        }) => {
            let label = format!("{}🄿", token.symbol);
            let ui_price = pnl_after_fees_usd
                .price
                .map(format_price_with_dollar_and_sign)
                .unwrap_or("…".to_string());
            (label, ui_price)
        }
    }
}

/// Formats a price result into a user-friendly string.
pub fn format_price_result(result: anyhow::Result<f64>) -> Option<String> {
    result
        .ok()
        .map(format_price)
        .or_else(|| Some("…".to_owned()))
}

/// Formats a price value into a user-friendly string.
pub fn format_price(price: f64) -> String {
    let price_string = Currency::new_string(
        price.to_string().as_str(),
        Some(CurrencyOpts::new().set_symbol("").set_precision(6)),
    )
    .unwrap()
    .to_string();

    price_string[..7.min(price_string.len())].to_string()
}

pub fn format_price_with_dollar(price: f64) -> String {
    let price_string = format_price(price);
    Currency::new_string(price_string.as_str(), None)
        .unwrap()
        .format()
}

pub fn format_price_with_dollar_and_sign(price: f64) -> String {
    let price_string = format_price_with_dollar(price);
    if price > 0f64 {
        format!("+{price_string}")
    } else {
        price_string
    }
}
