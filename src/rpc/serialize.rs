//! # Serialize JSON RPC parameters

use super::{ClientMethod, Error, MethodParams};
use super::{ToHex, align_bytes, to_arr, to_u64, trim_hex};
use super::core::{Address, PrivateKey, Transaction};
use jsonrpc_core::{Params, Value as JValue};
use rustc_serialize::hex::FromHex;
use serde::{Serialize, Serializer};
use serde_json::{self, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Deserialize, Debug)]
pub struct RPCAccount {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Debug)]
pub struct RPCTransaction {
    pub from: String,
    pub to: String,
    pub gas: String,
    #[serde(rename="gasPrice")]
    pub gas_price: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub data: String,
    pub nonce: String,
}

impl RPCTransaction {
    pub fn try_into(self) -> Result<Transaction, Error> {
        let gp_str = trim_hex(self.gas_price.as_str());
        let v_str = trim_hex(self.value.as_str());

        let gas_limit = trim_hex(self.gas.as_str()).from_hex()?;
        let gas_price = gp_str.from_hex()?;
        let value = v_str.from_hex()?;
        let nonce = trim_hex(self.nonce.as_str()).from_hex()?;

        Ok(Transaction {
            nonce: to_u64(&nonce),
            gas_price: to_arr(&align_bytes(&gas_price, 32)),
            gas_limit: to_u64(&gas_limit),
            to: self.to.as_str().parse::<Address>().ok(),
            value: to_arr(&align_bytes(&value, 32)),
            data: trim_hex(self.data.as_str()).from_hex()?,
        })
    }
}

lazy_static! {
    static ref REQ_ID: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(1));
}

fn empty_data() -> Option<String> {
    None
}

#[derive(Clone, Debug, Serialize)]
struct JsonData<'a> {
    jsonrpc: &'static str,
    method: &'static str,
    params: &'a Params,
    id: usize,
}

impl Transaction {
    /// Sign transaction and return as raw data
    pub fn to_raw_params(&self, pk: PrivateKey, chain: u8) -> Params {
        self.to_signed_raw(pk, chain)
            .map(|v| format!("0x{}", v.to_hex()))
            .map(|s| Params::Array(vec![JValue::String(s)]))
            .expect("Expect to sign a transaction")
    }
}


impl<'a> Serialize for MethodParams<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match self.0 {
            ClientMethod::Version => serialize("web3_clientVersion", self.1, s),
            ClientMethod::NetVersion => serialize("net_version", self.1, s),
            ClientMethod::EthSyncing => serialize("eth_syncing", self.1, s),
            ClientMethod::EthBlockNumber => serialize("eth_blockNumber", self.1, s),
            ClientMethod::EthGasPrice => serialize("eth_gasPrice", self.1, s),
            ClientMethod::EthAccounts => serialize("eth_accounts", self.1, s),
            ClientMethod::EthGetBalance => serialize("eth_getBalance", self.1, s),
            ClientMethod::EthGetTxCount => serialize("eth_getTransactionCount", self.1, s),
            ClientMethod::EthGetTxByHash => serialize("eth_getTransactionByHash", self.1, s),
            ClientMethod::EthSendRawTransaction => serialize("eth_sendRawTransaction", self.1, s),
            ClientMethod::EthCall => serialize("eth_call", self.1, s),
            ClientMethod::EthTraceCall => serialize("eth_traceCall", self.1, s),
        }
    }
}

fn serialize<S>(method: &'static str, params: &Params, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
{
    to_json_data(method, params).serialize(serializer)
}

fn to_json_data<'a>(method: &'static str, params: &'a Params) -> JsonData<'a> {
    let id = REQ_ID.fetch_add(1, Ordering::SeqCst);

    JsonData {
        jsonrpc: "2.0",
        method: method,
        params: params,
        id: id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpc_core::Params;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn should_increase_request_ids() {
        assert_eq!(to_json_data("", &Params::None).id, 1);
        assert_eq!(to_json_data("", &Params::None).id, 2);
        assert_eq!(to_json_data("", &Params::None).id, 3);
    }
}
