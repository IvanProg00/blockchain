use crate::{errors::Result, transaction::Transaction};
use chrono::Utc;
use log::info;
use merkle_cbt::merkle_tree::{Merge, CBMT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TARGET_HEXS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    timestamp: i64,
    transactions: Vec<Transaction>,
    prev_block_hash: String,
    hash: String,
    nonce: i32,
    height: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MergeTX;

impl Block {
    /// Creates new block.
    pub fn new_block(
        transactions: Vec<Transaction>,
        prev_block_hash: String,
        height: usize,
    ) -> Result<Self> {
        let timestamp = Utc::now().timestamp_millis();

        let mut block = Block {
            timestamp,
            transactions,
            hash: String::new(),
            prev_block_hash,
            nonce: 0,
            height,
        };

        block.run_proof_of_work()?;

        Ok(block)
    }

    /// Creates a new genesis block.
    pub fn new_genesis_block(coninbase: Transaction) -> Self {
        Block::new_block(vec![coninbase], String::new(), 0).unwrap()
    }

    pub fn get_transactions(&self) -> &Vec<Transaction> {
        &self.transactions
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn get_prev_hash(&self) -> String {
        self.prev_block_hash.clone()
    }

    /// Run performs a proof-of work.
    fn run_proof_of_work(&mut self) -> Result<()> {
        info!("Mining the block");

        while !self.validate()? {
            self.nonce += 1;
        }

        let data = self.prepare_hash_data()?;
        self.hash = format!("{:X}", Sha256::new().chain_update(&data[..]).finalize());

        Ok(())
    }

    /// Returns a hash of the transactions in the block.
    fn hash_transactions(&self) -> Result<Vec<u8>> {
        let transactions = self
            .transactions
            .iter()
            .map(|t| t.hash())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(|v| v.as_bytes().to_owned())
            .collect::<Vec<_>>();

        let tree = CBMT::<Vec<u8>, MergeTX>::build_merkle_tree(&transactions);

        Ok(tree.root())
    }

    fn prepare_hash_data(&self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(),
            self.hash_transactions()?,
            self.timestamp,
            TARGET_HEXS,
            self.nonce,
        );

        bincode::serialize(&content).map_err(|e| e.into())
    }

    fn validate(&self) -> Result<bool> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();

        hasher.update(&data[..]);
        let mut vec1: Vec<u8> = vec![];
        vec1.resize(TARGET_HEXS, b'0');

        Ok(format!("{:X}", hasher.finalize())[0..TARGET_HEXS] == String::from_utf8(vec1)?)
    }
}

impl Merge for MergeTX {
    type Item = Vec<u8>;

    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut data = left.clone();
        data.append(&mut right.clone());

        let hasher = Sha256::new_with_prefix(&data);
        let mut re = [0u8; 32];

        re.copy_from_slice(&hasher.finalize());
        re.to_vec()
    }
}
