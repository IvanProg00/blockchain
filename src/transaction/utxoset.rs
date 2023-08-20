use super::tx::TXOutputs;
use crate::blockchain::Blockchain;
use crate::errors::Result;
use crate::{block::Block, db};
use log::{debug, error};
use std::collections::HashMap;

pub struct UTXOSet {
    pub blockchain: Blockchain,
}

impl UTXOSet {
    /// Rebuilds the UTXO set.
    pub fn reindex(&self) -> Result<()> {
        if let Err(e) = std::fs::remove_dir_all(db::DB_UTXOS_PATH) {
            match e.kind() {
                std::io::ErrorKind::NotFound => debug!("blocks not exists to delete"),
                _ => error!("undefined error on delete blocks: {}", e),
            };
        }

        let db = sled::open(db::DB_UTXOS_PATH)?;
        let utxos = self.blockchain.find_utxo();

        for (txid, outs) in utxos {
            db.insert(txid.as_bytes(), bincode::serialize(&outs)?)?;
        }

        Ok(())
    }
    pub fn find_spendable_outputs(
        &self,
        address: &[u8],
        amount: i32,
    ) -> Result<(i32, HashMap<String, Vec<i32>>)> {
        let mut unspent_outputs: HashMap<String, Vec<i32>> = HashMap::new();
        let mut accumulated = 0;
        let db = sled::open(db::DB_UTXOS_PATH)?;

        for kv in db.iter() {
            let (k, v) = kv?;
            let txid = String::from_utf8(k.to_vec())?;
            let outs: TXOutputs = bincode::deserialize(&v)?;

            for out_idx in 0..outs.outputs.len() {
                if outs.outputs[out_idx].can_be_unlock_with(address) && accumulated < amount {
                    accumulated += outs.outputs[out_idx].value;
                    match unspent_outputs.get_mut(&txid) {
                        Some(v) => v.push(out_idx as i32),
                        None => {
                            unspent_outputs.insert(txid.clone(), vec![out_idx as i32]);
                        }
                    }
                }
            }
        }

        Ok((accumulated, unspent_outputs))
    }

    /// Finds UTXO for a public key hash
    pub fn find_utxo(&self, pub_key_hash: &[u8]) -> Result<TXOutputs> {
        let mut utxos = TXOutputs {
            outputs: Vec::new(),
        };
        let db = sled::open(db::DB_UTXOS_PATH)?;

        for kv in db.iter() {
            let (_, v) = kv?;
            let outs: TXOutputs = bincode::deserialize(&v)?;

            for out in outs.outputs {
                if out.can_be_unlock_with(pub_key_hash) {
                    utxos.outputs.push(out.clone())
                }
            }
        }

        Ok(utxos)
    }

    pub fn update(&self, block: &Block) -> Result<()> {
        let db = sled::open(db::DB_UTXOS_PATH)?;

        for tx in block.get_transactions() {
            if !tx.is_coinbase() {
                for vin in &tx.vin {
                    let mut update_outputs = TXOutputs {
                        outputs: Vec::new(),
                    };
                    let outs: TXOutputs = bincode::deserialize(&db.get(&vin.txid)?.unwrap())?;

                    for out_idx in 0..outs.outputs.len() {
                        if out_idx != vin.vout as usize {
                            update_outputs.outputs.push(outs.outputs[out_idx].clone());
                        }
                    }

                    if update_outputs.outputs.is_empty() {
                        db.remove(&vin.txid)?;
                    } else {
                        db.insert(vin.txid.as_bytes(), bincode::serialize(&update_outputs)?)?;
                    }
                }
            }

            let new_outputs = TXOutputs {
                outputs: tx.vout.to_vec(),
            };

            db.insert(tx.id.as_bytes(), bincode::serialize(&new_outputs)?)?;
        }

        Ok(())
    }

    pub fn count_transactions(&self) -> Result<i32> {
        let mut counter = 0;
        let db = sled::open(db::DB_UTXOS_PATH)?;

        for kv in db.iter() {
            kv?;
            counter += 1;
        }

        Ok(counter)
    }
}
