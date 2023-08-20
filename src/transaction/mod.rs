pub mod tx;
pub mod utxoset;

use crate::{
    errors::Result,
    wallet::{hash_pub_key, Wallet},
};
use crypto::ed25519;
use failure::format_err;
use log::{error, info};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tx::{TXInput, TXOutput};
use utxoset::UTXOSet;

pub const SUBSIDY: i32 = 10;

/// Transaction represents a Bitcoin transaction.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub vin: Vec<TXInput>,
    pub vout: Vec<TXOutput>,
}

impl Transaction {
    /// Creates a new transaction.
    pub fn new_utxo(
        wallet: &Wallet,
        to: &str,
        amount: i32,
        utxo_set: &UTXOSet,
    ) -> Result<Transaction> {
        info!(
            "new UTXO Transaction from: {} to: {}",
            wallet.get_address(),
            to
        );

        let mut vin = Vec::new();
        let mut pub_key_hash = wallet.public_key.clone();
        hash_pub_key(&mut pub_key_hash);

        let acc_v = utxo_set.find_spendable_outputs(&pub_key_hash, amount)?;

        if acc_v.0 < amount {
            error!("Not Enough balance");
            return Err(format_err!(
                "Not Enough balance: current balance {}",
                acc_v.0
            ));
        }

        for tx in acc_v.1 {
            for out in tx.1 {
                let input = TXInput {
                    txid: tx.0.clone(),
                    vout: out,
                    signature: Vec::new(),
                    pub_key: wallet.public_key.clone(),
                };
                vin.push(input);
            }
        }

        let mut vout = vec![TXOutput::new(amount, to.to_string())?];
        if acc_v.0 > amount {
            vout.push(TXOutput::new(acc_v.0 - amount, wallet.get_address())?)
        }

        let mut tx = Transaction {
            id: String::new(),
            vin,
            vout,
        };
        tx.id = tx.hash()?;
        utxo_set
            .blockchain
            .sign_transacton(&mut tx, &wallet.secret_key)?;

        Ok(tx)
    }

    /// Creates a new coinbase transaction.
    pub fn new_coinbase(to: String, mut data: String) -> Result<Transaction> {
        info!("new coinbase Transaction to: {}", to);

        let mut key: [u8; 32] = [0; 32];
        if data.is_empty() {
            let mut rand = OsRng;
            rand.fill_bytes(&mut key);
            data = format!("Reward to '{}'", to);
        }

        let mut pub_key = Vec::from(data.as_bytes());
        pub_key.append(&mut Vec::from(key));

        let mut tx = Transaction {
            id: String::new(),
            vin: vec![TXInput {
                txid: String::new(),
                vout: -1,
                signature: Vec::new(),
                pub_key: Vec::from(data.as_bytes()),
            }],
            vout: vec![TXOutput::new(SUBSIDY, to)?],
        };
        tx.id = tx.hash()?;

        Ok(tx)
    }

    /// Checks whether the transaction is coinbase.
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.is_empty() && self.vin[0].vout == -1
    }

    /// Signs each input of a transaction.
    pub fn sign(
        &mut self,
        private_key: &[u8],
        prev_txs: HashMap<String, Transaction>,
    ) -> Result<()> {
        if self.is_coinbase() {
            return Ok(());
        }

        for vin in &self.vin {
            if prev_txs.get(&vin.txid).unwrap().id.is_empty() {
                return Err(format_err!("ERROR: Previous transaction is not correct"));
            }
        }

        let mut tx_copy = self.trim_copy();

        for in_id in 0..tx_copy.vin.len() {
            let prev_tx = prev_txs.get(&tx_copy.vin[in_id].txid).unwrap();
            tx_copy.vin[in_id].signature.clear();
            tx_copy.vin[in_id].pub_key = prev_tx.vout[tx_copy.vin[in_id].vout as usize]
                .pub_key_hash
                .clone();
            tx_copy.id = tx_copy.hash()?;
            tx_copy.vin[in_id].pub_key = Vec::new();
            let signature = ed25519::signature(tx_copy.id.as_bytes(), private_key);
            self.vin[in_id].signature = signature.to_vec();
        }

        Ok(())
    }

    /// Get hash of the Transaction.
    pub fn hash(&self) -> Result<String> {
        let mut copy = self.clone();
        copy.id = String::new();
        let data = bincode::serialize(self)?;
        let hasher = Sha256::new_with_prefix(&data[..]);

        Ok(format!("{:X}", hasher.finalize()))
    }

    /// Creates a trimmed copy of the Transaction to be used in signing.
    fn trim_copy(&self) -> Transaction {
        let vin = self
            .vin
            .iter()
            .map(|v| TXInput {
                txid: v.txid.clone(),
                vout: v.vout,
                signature: Vec::new(),
                pub_key: Vec::new(),
            })
            .collect();

        let vout = self
            .vout
            .iter()
            .map(|v| TXOutput {
                value: v.value,
                pub_key_hash: v.pub_key_hash.clone(),
            })
            .collect();

        Transaction {
            id: self.id.clone(),
            vin,
            vout,
        }
    }
}
