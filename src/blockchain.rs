use crate::{
    block::{Block, TARGET_HEXS},
    db,
    errors::Result,
    transaction::{tx::TXOutputs, Transaction},
};
use failure::format_err;
use log::{debug, error, info};
use std::collections::HashMap;

const GENESIS_COINBASE_DATA: &str =
    "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks";

#[derive(Debug, Clone)]
pub struct Blockchain {
    current_hash: String,
    db: sled::Db,
}

/// Used to iterate over blockchain blocks.
pub struct BlockchainIterator<'a> {
    current_hash: String,
    bc: &'a Blockchain,
}

impl Blockchain {
    /// Creates a new Blockchain db.
    pub fn new() -> Result<Blockchain> {
        info!("open blockchain");

        let db = sled::open(db::DB_BLOCKS_PATH)?;
        let hash = match db.get("LAST")? {
            Some(l) => l.to_vec(),
            None => Vec::new(),
        };

        info!("Found block database");

        let last_hash = if hash.is_empty() {
            String::new()
        } else {
            String::from_utf8(hash.to_vec())?
        };

        Ok(Blockchain {
            current_hash: last_hash,
            db,
        })
    }

    /// Creates a new Blockchain db.
    pub fn create_blockchain(address: String) -> Result<Blockchain> {
        info!("creating new blockchain");

        if let Err(e) = std::fs::remove_dir_all(db::DB_BLOCKS_PATH) {
            match e.kind() {
                std::io::ErrorKind::NotFound => debug!("blocks not exists to delete"),
                _ => error!("undefined error on delete blocks: {}", e),
            };
        }

        let db = sled::open(db::DB_BLOCKS_PATH)?;

        info!("creating new block database");

        let cbtx = Transaction::new_coinbase(address, String::from(GENESIS_COINBASE_DATA))?;
        let genesis: Block = Block::new_genesis_block(cbtx);

        db.insert(genesis.get_hash(), bincode::serialize(&genesis)?)?;
        db.insert("LAST", genesis.get_hash().as_bytes())?;

        let bc = Blockchain {
            current_hash: genesis.get_hash(),
            db,
        };

        bc.db.flush()?;

        Ok(bc)
    }

    /// Adds block into the Blockchain.
    pub fn add_block(&mut self, transactions: Vec<Transaction>) -> Result<Block> {
        let lasthash = self.db.get("LAST")?.unwrap();

        let new_block = Block::new_block(
            transactions,
            String::from_utf8(lasthash.to_vec())?,
            TARGET_HEXS,
        )?;
        self.db
            .insert(new_block.get_hash(), bincode::serialize(&new_block)?)?;
        self.db.insert("LAST", new_block.get_hash().as_bytes())?;
        self.current_hash = new_block.get_hash();
        Ok(new_block)
    }

    /// Finds and returns all unspent transaction outputs
    pub fn find_utxo(&self) -> HashMap<String, TXOutputs> {
        let mut utxos: HashMap<String, TXOutputs> = HashMap::new();
        let mut spend_txos: HashMap<String, Vec<i32>> = HashMap::new();

        for block in self.iter() {
            for tx in block.get_transactions() {
                for index in 0..tx.vout.len() {
                    if let Some(ids) = spend_txos.get(&tx.id) {
                        if ids.contains(&(index as i32)) {
                            continue;
                        }
                    }

                    match utxos.get_mut(&tx.id) {
                        Some(v) => {
                            v.outputs.push(tx.vout[index].clone());
                        }
                        None => {
                            utxos.insert(
                                tx.id.clone(),
                                TXOutputs {
                                    outputs: vec![tx.vout[index].clone()],
                                },
                            );
                        }
                    }
                }

                if !tx.is_coinbase() {
                    for i in &tx.vin {
                        match spend_txos.get_mut(&i.txid) {
                            Some(v) => {
                                v.push(i.vout);
                            }
                            None => {
                                spend_txos.insert(i.txid.clone(), vec![i.vout]);
                            }
                        }
                    }
                }
            }
        }

        utxos
    }

    /// Returns Blockchain iterator.
    pub fn iter(&self) -> BlockchainIterator {
        BlockchainIterator {
            current_hash: self.current_hash.clone(),
            bc: self,
        }
    }

    /// FindTransaction finds a transaction by its ID
    pub fn find_transacton(&self, id: &str) -> Result<Transaction> {
        for b in self.iter() {
            for tx in b.get_transactions() {
                if tx.id == id {
                    return Ok(tx.clone());
                }
            }
        }

        Err(format_err!("Transaction is not found"))
    }

    fn get_prev_txs(&self, tx: &Transaction) -> Result<HashMap<String, Transaction>> {
        let mut prev_txs = HashMap::new();

        for vin in &tx.vin {
            let prev_tx = self.find_transacton(&vin.txid)?;
            prev_txs.insert(prev_tx.id.clone(), prev_tx);
        }

        Ok(prev_txs)
    }

    /// Signs inputs of a Transaction.
    pub fn sign_transacton(&self, tx: &mut Transaction, private_key: &[u8]) -> Result<()> {
        let prev_txs = self.get_prev_txs(tx)?;
        tx.sign(private_key, prev_txs)?;
        Ok(())
    }
}

impl<'a> Iterator for BlockchainIterator<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        match self.bc.db.get(&self.current_hash).ok()? {
            Some(b) => {
                if let Ok(block) = bincode::deserialize::<Block>(&b) {
                    self.current_hash = block.get_prev_hash();
                    Some(block)
                } else {
                    None
                }
            }
            None => None,
        }
    }
}
