use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::BufReader;

use crate::prices::TokenSymbol;

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Token {
    pub address: String,
    pub symbol: TokenSymbol,
    pub name: String,
    pub decimals: u8,
}

#[derive(Debug, Default, Clone)]
pub struct TokenRegistry {
    pub tokens: Vec<Token>,
    pub pairs: Vec<[Token; 2]>,
    pub stable_tokens: Vec<Token>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        let file_path = "./tokens/default.json";
        let stable_file_path = "./tokens/stable.json";
        let pairs_file_path = "./tokens/pairs.json";
        let tokens = Self::load_tokens(file_path).expect("Missing default.json");
        let stable_tokens = Self::load_tokens(stable_file_path).expect("Missing stable.json");
        let pairs = Self::load_pairs(tokens.clone(), pairs_file_path).expect("Missing pairs.json");

        TokenRegistry {
            tokens,
            pairs,
            stable_tokens,
        }
    }

    fn load_tokens(file_path: &str) -> anyhow::Result<Vec<Token>> {
        let file = File::open(file_path).context("Failed to open file")?;
        let reader = BufReader::new(file);
        let tokens = serde_json::from_reader(reader)?;

        Ok(tokens)
    }

    fn load_pairs(tokens: Vec<Token>, file_path: &str) -> anyhow::Result<Vec<[Token; 2]>> {
        let file = File::open(file_path).context("Failed to open file")?;
        let reader = BufReader::new(file);
        let pair_addresses: Vec<[String; 2]> = serde_json::from_reader(reader)?;
        let pairs = pair_addresses
            .into_iter()
            .map(|pair_addresses| {
                let token_a = tokens
                    .iter()
                    .find(|token| token.address == pair_addresses[0])
                    .expect("Token not found");
                let token_b = tokens
                    .iter()
                    .find(|token| token.address == pair_addresses[1])
                    .expect("Token not found");

                [token_a.clone(), token_b.clone()]
            })
            .collect::<Vec<_>>();

        Ok(pairs)
    }

    pub fn get_by_address(&self, address: &str) -> Option<&Token> {
        self.tokens
            .iter()
            .find(|token| token.address == address)
            .or(self
                .stable_tokens
                .iter()
                .find(|token| token.address == address))
    }

    pub fn get_by_symbol(&self, symbol: &TokenSymbol) -> Option<&Token> {
        self.tokens
            .iter()
            .find(|token| token.symbol == *symbol)
            .or(self
                .stable_tokens
                .iter()
                .find(|token| token.symbol == *symbol))
    }

    pub fn get_by_pair_address(&self, address: &str) -> anyhow::Result<Vec<Token>> {
        if !address.contains("_") {
            bail!("Not pair address")
        }

        let pairs = address.split("_").collect::<Vec<_>>();
        let tokens = vec![
            self.get_by_address(pairs[0])
                .unwrap_or_else(|| panic!("Not exist: {}", pairs[0]))
                .clone(),
            self.get_by_address(pairs[1])
                .unwrap_or_else(|| panic!("Not exist: {}", pairs[1]))
                .clone(),
        ];

        Ok(tokens)
    }

    pub fn get_tokens_from_pair_address(&self, address: &str) -> anyhow::Result<Vec<Token>> {
        let tokens = if address.starts_with("SOL_PERPS") {
            // TODO: support more token?
            vec![Token {
                address: "So11111111111111111111111111111111111111112_PERPS".to_owned(),
                symbol: TokenSymbol::SOL_PERPS,
                name: address.to_owned(),
                decimals: 9u8,
            }]
        } else if address.contains("_") {
            self.get_by_pair_address(address).expect("Invalid address")
        } else if let Some(token) = self.get_by_address(address) {
            vec![token.clone()]
        } else {
            vec![]
        };

        Ok(tokens)
    }
}

pub fn get_pair_or_token_address_from_tokens(tokens: &[Token]) -> anyhow::Result<String> {
    let address = if tokens.len() == 1 {
        tokens[0].address.clone()
    } else {
        format!("{}_{}", tokens[0].address, tokens[1].address)
    };

    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_registry_load_and_parse() {
        // Make test return Result
        let token_registry = TokenRegistry::new();
        let sol_token = token_registry
            .get_by_address("So11111111111111111111111111111111111111112")
            .unwrap();
        let jlp_token = token_registry.get_by_symbol(&TokenSymbol::JLP).unwrap();

        assert_eq!(sol_token.symbol, TokenSymbol::SOL);
        assert_eq!(jlp_token.symbol, TokenSymbol::JLP);
    }

    #[test]
    fn test_tokens() {
        let registry = TokenRegistry::new();
        assert!(!registry.tokens.is_empty());
        assert!(!registry.stable_tokens.is_empty());
    }
}
