use crate::{database::errors::ParsingError, eth_rpc::errors::EthCallError};
use axum::http::Uri;
use crypto_bigint::U256;
use hex::{encode, decode};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{future::Future, str::FromStr, sync::OnceLock};

pub static ETHEREUM_ENDPOINT: OnceLock<Provider> = OnceLock::new();

pub struct Endpoints;

impl Endpoints {
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        ETHEREUM_ENDPOINT.get_or_init(|| Provider {
            url: Uri::from_str(&dotenvy::var("ETHEREUM_ENDPOINT").unwrap()).unwrap(),
        });

        Ok(())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Chains {
    Ethereum,
}

pub trait EthCall {
    type Inner;

    fn call(
        &self,
        provider: &Uri,
    ) -> impl Future<Output = Result<Self::Inner, EthCallError>> + Send;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Methods<T>
where
    T: EthCall,
{
    GetTxByHash(T),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionByHash {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt(pub GetTransactionByHash);

impl Receipt {
    pub fn new(hash: String) -> Self {
        let hash = GetTransactionByHash::new(hash);
        Receipt(hash)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResultWrapper<T> {
    result: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawGetTransactionReceiptResponse {
    transaction_hash: String,
    transaction_index: String,
    block_hash: String,
    block_number: String,
    from: String,
    to: String,
    cumulative_gas_used: String,
    effective_gas_price: String,
    gas_used: String,
    contract_address: Option<String>,
    logs: Vec<String>,
    logs_bloom: String,
    #[serde(rename = "type")]
    tx_type: String,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawGetTransactionByHashResponse {
    access_list: serde_json::Value,
    block_hash: String,
    block_number: String,
    chain_id: String,
    from: String,
    gas: String,
    gas_price: String,
    hash: String,
    input: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    nonce: String,
    to: String,
    transaction_index: String,
    #[serde(rename = "type")]
    tx_type: String,
    value: String,
    v: String,
    r: String,
    s: String,
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub to: Address,
    pub amount: U256,
}

impl FromStr for Transfer {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fn_selector = &s[0..10];
        if fn_selector != "0xa9059cbb" {
            return Err(Box::new(ParsingError(fn_selector.to_string(), "Transfer")));
        }
        let bytes = decode(&s[2..])?;
        let to = &bytes[16..36];
        let amount = U256::from_be_slice(&bytes[36..]);
        let addy = format!("{}{}", "0x", encode(to));
        Ok(Transfer {
            to: Address::from_str(&addy)?,
            amount,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Address(String);

impl Address {
    pub fn try_address(s: &str) -> Result<(), ParsingError> {
        if !s.starts_with("0x"){
            return Err(ParsingError(s.to_owned(), "Address"));

        }

        if s.len() != 42usize {
            return Err(ParsingError(s.to_owned(), "Address"));
        }

        Ok(())
    }
}

impl FromStr for Address {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("0x") {
            return Err(ParsingError(s.to_owned(), "Address"));
        }

        if s.len() != 42usize {
            return Err(ParsingError(s.to_owned(), "Address"));
        } 

        Ok(Address(s.to_string()))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// #[derive(Debug, Clone, Serialize)]
// pub struct TransactionData {
//     block_number: u64,
//     chain_id: u16,
//     from: String,
//     to: String,
//     value: u64,
//     // this is important -- for parsing the calldata sent to a smart
//     // contract if we are being paid in tokens
//     input: String,
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub url: axum::http::Uri,
}

impl GetTransactionByHash {
    pub fn new(hash: String) -> GetTransactionByHash {
        Self { data: hash }
    }
}

impl Provider {
    pub fn new(url: Uri) -> Provider {
        Self { url }
    }

    pub async fn get_transaction_by_hash(
        &self,
        args: GetTransactionByHash,
    ) -> Result<RawGetTransactionByHashResponse, EthCallError> {
        let res = args.call(&self.url).await?.result.result;
        Ok(res)
    }

    pub async fn get_transaction_receipt(
        &self,
        args: Receipt,
    ) -> Result<RawGetTransactionReceiptResponse, EthCallError> {
        let res = args.call(&self.url).await?.result.result;
        Ok(res)
    }
}

#[cfg(test)]
pub mod tests {
   use crate::eth_rpc::types::{Address, Transfer};
   use std::str::FromStr;
   use crypto_bigint::U256;

    #[test]
    fn parse_transfer_calldata() -> Result<(), Box<dyn std::error::Error>>{
        let calldata = "0xa9059cbb0000000000000000000000003f5047bdb647dc39c88625e17bdbffee905a9f4400000000000000000000000000000000000000000000011c9a62d04ed0c80000"; 
        let expected_address = Address::from_str(&"0x3F5047BDb647Dc39C88625E17BDBffee905A9F44".to_lowercase())?; 
        let expected_amount = U256::from_u128(5250000000000000000000u128);
        let res = Transfer::from_str(calldata)?;
        assert_eq!(res.to, expected_address);
        assert_eq!(res.amount, expected_amount);
        Ok(())
    }
    

}
