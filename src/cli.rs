use crate::{
    blockchain::Blockchain,
    errors::Result,
    transaction::{utxoset::UTXOSet, Transaction},
    wallet::Wallets,
};
use bitcoincash_addr::Address;
use clap::{arg, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "blockchain",
    about = "Blockchain in Rust",
    author = "gavrilovivan2000@gmail.com"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Print all the chain blocks")]
    PrintChain,
    #[command(about = "Create a wallet")]
    CreateWallet,
    #[command(about = "List all addresses")]
    ListAddresses,
    #[command(about = "Reindex UTXO")]
    Reindex,
    #[command(about = "Get balance")]
    GetBalance {
        #[arg(help = "Address of the wallet")]
        address: String,
    },
    #[command(about = "Create new blockchain")]
    Create {
        #[arg(help = "Address of the wallet")]
        address: String,
    },
    #[command(about = "Send money to another account")]
    Send {
        #[arg(help = "Source wallet address")]
        from: String,
        #[arg(help = "Destination wallet address")]
        to: String,
        #[arg(help = "Amount of money to be sent from source wallet to recipient wallet")]
        amount: i32,
    },
}

impl Cli {
    pub fn run() -> Result<()> {
        let cli = Cli::parse();

        match cli.command {
            Commands::PrintChain => cmd_print_chain(),
            Commands::CreateWallet => {
                println!("Wallet created with address {}", cmd_create_wallet()?);
                Ok(())
            }
            Commands::ListAddresses => cmd_list_address(),
            Commands::Reindex => {
                let count = cmd_reindex()?;
                println!("Done! There are {} transactions in the UTXO set.", count);
                Ok(())
            }
            Commands::GetBalance { address } => {
                let balance = cmd_get_balance(&address)?;
                println!("Balance of '{}'; {} ", &address, balance);
                Ok(())
            }
            Commands::Create { address } => cmd_create_blockchain(&address),
            Commands::Send { from, to, amount } => cmd_send(&from, &to, amount),
        }
    }
}

fn cmd_send(from: &str, to: &str, amount: i32) -> Result<()> {
    let bc = Blockchain::new()?;
    let mut utxo_set = UTXOSet { blockchain: bc };
    let wallets = Wallets::new()?;
    let wallet = wallets.get_wallet(from).unwrap();
    let tx = Transaction::new_utxo(wallet, to, amount, &utxo_set)?;
    let new_block = utxo_set.blockchain.add_block(vec![tx])?;

    utxo_set.update(&new_block)?;

    println!("success!");
    Ok(())
}

fn cmd_create_wallet() -> Result<String> {
    let mut ws = Wallets::new()?;
    let address = ws.create_wallet();
    ws.save_all()?;

    Ok(address)
}

fn cmd_reindex() -> Result<i32> {
    let bc = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain: bc };
    utxo_set.reindex()?;
    utxo_set.count_transactions()
}

fn cmd_create_blockchain(address: &str) -> Result<()> {
    let address = String::from(address);
    let bc = Blockchain::create_blockchain(address)?;

    let utxo_set = UTXOSet { blockchain: bc };
    utxo_set.reindex()?;
    println!("create blockchain");
    Ok(())
}

fn cmd_get_balance(address: &str) -> Result<i32> {
    let pub_key_hash = Address::decode(address).unwrap().body;
    let bc = Blockchain::new()?;
    let utxo_set = UTXOSet { blockchain: bc };
    let utxos = utxo_set.find_utxo(&pub_key_hash)?;
    let mut balance = 0;

    for out in utxos.outputs {
        balance += out.value;
    }

    Ok(balance)
}

fn cmd_print_chain() -> Result<()> {
    let bc = Blockchain::new()?;

    for b in bc.iter() {
        println!("{:#?}", b);
    }

    Ok(())
}

fn cmd_list_address() -> Result<()> {
    let ws = Wallets::new()?;
    for ad in ws.get_all_addresses() {
        println!("{}", ad);
    }

    Ok(())
}
