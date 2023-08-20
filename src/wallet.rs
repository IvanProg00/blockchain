use crate::{db, errors::Result};
use bitcoincash_addr::{Address, HashType, Scheme};
use crypto::ed25519;
use log::info;
use rand::{rngs::OsRng, RngCore};
use ripemd::Ripemd160;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Wallet {
    pub secret_key: Vec<u8>,
    pub public_key: Vec<u8>,
}

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallet {
    /// Creates a new Wallet.
    fn new() -> Self {
        let mut key: [u8; 32] = [0; 32];
        OsRng.fill_bytes(&mut key);
        let (secrect_key, public_key) = ed25519::keypair(&key);
        let secret_key = secrect_key.to_vec();
        let public_key = public_key.to_vec();
        Wallet {
            secret_key,
            public_key,
        }
    }

    /// Returns wallet address.
    pub fn get_address(&self) -> String {
        let mut pub_hash = self.public_key.clone();
        hash_pub_key(&mut pub_hash);
        let address = Address {
            body: pub_hash,
            scheme: Scheme::Base58,
            hash_type: HashType::Script,
            ..Default::default()
        };
        address.encode().unwrap()
    }
}

impl Wallets {
    /// Creates Wallets and fills it from a file if it exists.
    pub fn new() -> Result<Wallets> {
        let mut wlt = Wallets {
            wallets: HashMap::<String, Wallet>::new(),
        };

        let db = sled::open(db::DB_WALLETS_PATH)?;
        for item in db.into_iter() {
            let i = item?;
            let address = String::from_utf8(i.0.to_vec())?;
            let wallet = bincode::deserialize(&i.1)?;
            wlt.wallets.insert(address, wallet);
        }

        drop(db);
        Ok(wlt)
    }

    /// Creates wallet, adds it in Wallets. Returns address of the wallet
    /// created.
    pub fn create_wallet(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);

        info!("Create wallet: {}", address);

        address
    }

    /// Returns an array of addresses store in a wallet file.
    pub fn get_all_addresses(&self) -> Vec<String> {
        self.wallets.keys().cloned().collect()
    }

    /// Get wallet by address.
    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    /// Saves wallets to a file.
    pub fn save_all(&self) -> Result<()> {
        let db = sled::open(db::DB_WALLETS_PATH)?;

        for (address, wallet) in &self.wallets {
            let data = bincode::serialize(wallet)?;
            db.insert(address, data)?;
        }

        db.flush()?;
        drop(db);

        Ok(())
    }
}

/// Hashes public key.
pub fn hash_pub_key(pub_key: &mut Vec<u8>) {
    let mut hasher1 = Sha256::new();
    hasher1.update(pub_key.clone());
    pub_key.copy_from_slice(&hasher1.finalize());

    let mut hasher2 = Ripemd160::new();
    hasher2.update(pub_key.clone());
    pub_key.resize(20, 0);
    pub_key.copy_from_slice(&hasher2.finalize());
}
