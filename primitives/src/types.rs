use near_primitives::borsh::{BorshDeserialize, BorshSerialize};
use std::fmt::Display;

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ChainId {
    Mainnet,
    Testnet,
}

impl Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainId::Mainnet => write!(f, "mainnet"),
            ChainId::Testnet => write!(f, "testnet"),
        }
    }
}

impl TryFrom<String> for ChainId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "mainnet" => Ok(ChainId::Mainnet),
            "testnet" => Ok(ChainId::Testnet),
            _ => Err(format!("Invalid chain id: {}", value)),
        }
    }
}
