use bdk_bitcoind_rpc::bitcoincore_rpc::{Auth, Client, RpcApi};
use bdk_wallet::bitcoin::Transaction;
use bdk_wallet::bitcoin::consensus::Encodable;
use serde_json::{json, Value};

pub struct BitcoinRpcClient {
    client: Client,
}

impl BitcoinRpcClient {
    pub fn new(rpc_url: &str, username: &str, password: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let auth = Auth::UserPass(
            username.to_string(),
            password.to_string(),
        );

        let client = Client::new(rpc_url, auth)?;

        Ok(Self { client })
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    
    pub fn submit_transaction(&self, transaction: &Transaction) -> Result<String, Box<dyn std::error::Error>> {
        let txid = self.client.send_raw_transaction(transaction)?;
        Ok(txid.to_string())
    }

    #[allow(dead_code)]
    pub fn submit_package(&self, transactions: &[Transaction]) -> Result<Value, Box<dyn std::error::Error>> {
        // Convert transactions to hex strings
        let tx_hexes: Vec<String> = transactions
            .iter()
            .map(|tx| {
                let mut writer = Vec::new();
                tx.consensus_encode(&mut writer)?;
                Ok(hex::encode(&writer))
            })
            .collect::<Result<_, Box<dyn std::error::Error>>>()?;

        // Call submitpackage RPC
        let params = json!(tx_hexes);
        let result = self.client.call::<Value>("submitpackage", &[params])?;
        Ok(result)
    }
}
