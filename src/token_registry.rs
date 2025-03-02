use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{collections::HashMap, fmt, str::FromStr};

use crate::prices::MainTokenSymbol;

// Embedded JSON data
const TOKENS_JSON: &str = r#"
[
  {
    "address": "So11111111111111111111111111111111111111112",
    "symbol": "SOL",
    "name": "Wrapped SOL",
    "decimals": 9,
    "stable": false
  },
  {
    "address": "jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v",
    "symbol": "JupSOL",
    "name": "Jupiter Staked SOL",
    "decimals": 9,
    "stable": false
  },
  {
    "address": "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4",
    "symbol": "JLP",
    "name": "Jupiter Perps",
    "decimals": 6,
    "stable": false
  },
  {
    "address": "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
    "symbol": "JUP",
    "name": "Jupiter",
    "decimals": 6,
    "stable": false
  },
  {
    "address": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "symbol": "USDC",
    "name": "USD Coin",
    "decimals": 6,
    "stable": true
  }
]
"#;

const PAIRS_JSON: &str = r#"
[
  ["jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v", "So11111111111111111111111111111111111111112"],
  ["27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4", "So11111111111111111111111111111111111111112"]
]
"#;

// TokenSymbol now carries its string representation
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenSymbolString(String);

impl<'de> Deserialize<'de> for TokenSymbolString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(TokenSymbolString(s))
    }
}

impl fmt::Display for TokenSymbolString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for TokenSymbolString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: TokenSymbolString,
    pub name: String,
    pub decimals: u8,
    pub stable: bool,
}

#[derive(Debug, Clone)]
pub struct TokenRegistry {
    pub tokens: Vec<Token>,
    #[allow(unused)]
    pub pairs: Vec<[Token; 2]>,
    pub address_map: HashMap<String, Token>,
    pub symbol_map: HashMap<String, TokenSymbolString>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        // Parse tokens
        let tokens: Vec<Token> = serde_json::from_str(TOKENS_JSON).expect("Invalid tokens JSON");

        // Create symbol map
        let symbol_map: HashMap<String, TokenSymbolString> = tokens
            .iter()
            .map(|t| (t.symbol.0.clone(), t.symbol.clone()))
            .collect();

        // Create address map
        let address_map: HashMap<String, Token> = tokens
            .iter()
            .map(|t| (t.address.clone(), t.clone()))
            .collect();

        // Parse pairs
        let pair_addresses: Vec<[String; 2]> =
            serde_json::from_str(PAIRS_JSON).expect("Invalid pairs JSON");
        let pairs = pair_addresses
            .into_iter()
            .map(|[addr1, addr2]| {
                let token1 = address_map.get(&addr1).expect("Pair token1 not found");
                let token2 = address_map.get(&addr2).expect("Pair token2 not found");
                [token1.clone(), token2.clone()]
            })
            .collect();

        Self {
            tokens,
            pairs,
            address_map,
            symbol_map,
        }
    }

    pub fn get_by_address(&self, address: &str) -> Option<&Token> {
        self.address_map.get(address)
    }

    pub fn get_by_symbol_string(&self, symbol_string: &TokenSymbolString) -> Option<&Token> {
        self.tokens.iter().find(|t| t.symbol == *symbol_string)
    }

    pub fn get_by_symbol(&self, symbol: &MainTokenSymbol) -> Option<&Token> {
        self.tokens.iter().find(|t| t.symbol.0 == *symbol.as_ref())
    }

    pub fn get_by_pair_address(&self, address: &str) -> Option<Vec<Token>> {
        if !address.contains("_") {
            return None;
        }

        let pairs = address.split("_").collect::<Vec<_>>();
        if pairs.len() != 2 {
            return None;
        }

        Some(vec![
            self.address_map
                .get(pairs[0])
                .expect("Invalid address")
                .clone(),
            self.address_map
                .get(pairs[1])
                .expect("Invalid address")
                .clone(),
        ])
    }

    pub fn get_tokens_from_pair_address(&self, address: &str) -> Vec<Token> {
        if address.starts_with("SOL_PERPS") {
            // TODO: support more token?
            vec![Token {
                address: "So11111111111111111111111111111111111111112_PERPS".to_string(),
                symbol: TokenSymbolString("SOL_PERPS".to_string()),
                name: "SOL PERPS".to_string(),
                decimals: 9,
                stable: false,
            }]
        } else if let Some(tokens) = self.get_by_pair_address(address) {
            tokens
        } else if let Some(token) = self.get_by_address(address) {
            vec![token.clone()]
        } else {
            vec![]
        }
    }

    pub fn get_pair_or_token_address_from_tokens(&self, tokens: &[Token]) -> String {
        if tokens.len() == 1 {
            tokens[0].address.to_string()
        } else {
            format!("{}_{}", tokens[0].address, tokens[1].address)
        }
    }

    pub fn default_token() -> Token {
        get_by_symbol(&TokenSymbolString(MainTokenSymbol::SOL.to_string()))
            .unwrap()
            .clone()
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenSymbolString {
    pub fn to_str(&self) -> String {
        self.0.to_string()
    }
}

impl FromStr for TokenSymbolString {
    type Err = (); // Use a simple error type (or a custom one)

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(token_symbol) = REGISTRY.symbol_map.get(s).cloned() {
            Ok(token_symbol)
        } else {
            Err(()) // Or return a more informative error
        }
    }
}

static REGISTRY: Lazy<TokenRegistry> = Lazy::new(TokenRegistry::new);

pub fn get_by_address(address: &str) -> Option<&'static Token> {
    REGISTRY.get_by_address(address)
}

pub fn get_by_symbol(symbol: &TokenSymbolString) -> Option<&'static Token> {
    REGISTRY.get_by_symbol_string(symbol)
}

pub fn get_by_pair_address(address: &str) -> Option<Vec<Token>> {
    REGISTRY.get_by_pair_address(address)
}

pub fn get_tokens_from_pair_address(address: &str) -> Vec<Token> {
    REGISTRY.get_tokens_from_pair_address(address)
}

pub fn get_pair_or_token_address_from_tokens(tokens: &[Token]) -> String {
    REGISTRY.get_pair_or_token_address_from_tokens(tokens)
}

pub fn get_symbol_pair_from_tokens(tokens: &[Token]) -> anyhow::Result<String> {
    let symbol_pair = if tokens.len() == 1 {
        format!("{}_{}", tokens[0].symbol, "USDT")
    } else {
        format!("{}_{}", tokens[0].symbol, tokens[1].symbol)
    };

    Ok(symbol_pair)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_registry_load_and_parse() {
        let sol_token = get_by_address("So11111111111111111111111111111111111111112").unwrap();
        let jlp_token = get_by_symbol(&TokenSymbolString("JLP".to_string())).unwrap();

        assert_eq!(sol_token.symbol.to_str(), "SOL");
        assert_eq!(jlp_token.symbol.to_str(), "JLP");
    }

    #[test]
    fn test_pairs() {
        let pair = get_by_pair_address("jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v_So11111111111111111111111111111111111111112")
            .unwrap();
        assert_eq!(pair.len(), 2);
        assert_eq!(pair[0].symbol.to_str(), "JupSOL");
        assert_eq!(pair[1].symbol.to_str(), "SOL");
    }

    #[test]
    fn test_symbol_conversion() {
        assert_eq!(TokenSymbolString::from_str("SOL").unwrap().to_str(), "SOL");
        assert_eq!(
            TokenSymbolString::from_str("USDC").unwrap().to_str(),
            "USDC"
        );
    }
}
