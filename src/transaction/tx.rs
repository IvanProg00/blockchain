use crate::errors::Result;
use bitcoincash_addr::Address;
use log::debug;
use serde::{Deserialize, Serialize};

/// TXInput represents a transaction input
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXInput {
    pub txid: String,
    pub vout: i32,
    pub signature: Vec<u8>,
    pub pub_key: Vec<u8>,
}

/// TXOutput represents a transaction output
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput {
    pub value: i32,
    pub pub_key_hash: Vec<u8>,
}

// TXOutputs collects TXOutput
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutputs {
    pub outputs: Vec<TXOutput>,
}

impl TXOutput {
    pub fn new(value: i32, address: String) -> Result<Self> {
        let mut txo = TXOutput {
            value,
            pub_key_hash: Vec::new(),
        };

        txo.lock(&address)?;
        Ok(txo)
    }

    /// Checks if the output can be unlocked with the provided data.
    pub fn can_be_unlock_with(&self, unlocking_data: &[u8]) -> bool {
        self.pub_key_hash == unlocking_data
    }

    /// Lock signs the output.
    fn lock(&mut self, address: &str) -> Result<()> {
        let pub_key_hash = Address::decode(address).unwrap().body;

        debug!("lock: {}", address);

        self.pub_key_hash = pub_key_hash;
        Ok(())
    }
}
