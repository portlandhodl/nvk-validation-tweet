mod rpc_client;
mod wallet;
use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;
use bdk_wallet::LocalOutput;
use rpc_client::BitcoinRpcClient;
use wallet::wallet::BitcoinWallet;
use bdk_wallet::bitcoin::{Network, Amount, Address};
use std::str::FromStr;
use std::path::PathBuf;


const BITCOIN_HOST: &str = "http://192.168.3.89:18443";
const BITCOIN_USER: &str = "test";
const BITCOIN_PASS: &str = "test";
const REGTEST_BURN_ADDRESS: &str = "bcrt1qs758ursh4q9z627kt3pp5yysm78ddny6txaqgw";
const UTXOS_PER_GROUP: u8 = 100;
const NUMBER_PER_BLOCK_TXNS: u64 = 10000;
const XPUB2_CONSOLIDATION_TXNS: u64 = 20;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    // Initialize RPC client with the defined constants
    let rpc_client = BitcoinRpcClient::new(BITCOIN_HOST, BITCOIN_USER, BITCOIN_PASS)?;
    let rpc_url = BITCOIN_HOST.to_string();
    let auth = bdk_bitcoind_rpc::bitcoincore_rpc::Auth::UserPass(
        BITCOIN_USER.to_string(),
        BITCOIN_PASS.to_string(),
    );

    println!("Creating XPUB1 Wallet.");
    // XPUB 1 = Initialize wallet (Using the BDK defaults becuase our coins cant be rugged on regtest)
    let xpub1_wallet_path = PathBuf::from("./wallet_data/regtest_xpub1.db");
    let xpub1_descriptor = "wpkh(tprv8gRuBrXL4NQDSur4QQFRBDs4187CCQAnANo3bu6u4sHQgXbypRHpZPPKG2nBppvQ56sdWEqLLXaxmbrpt5Gcc2jLwbZ4S8Jx2D326888XqQ/84'/1'/0'/0/*)";
    let xpub1_change_descriptor = "wpkh(tprv8gRuBrXL4NQDSur4QQFRBDs4187CCQAnANo3bu6u4sHQgXbypRHpZPPKG2nBppvQ56sdWEqLLXaxmbrpt5Gcc2jLwbZ4S8Jx2D326888XqQ/84'/1'/0'/1/*)";    
    let mut xpub1_wallet = BitcoinWallet::new(xpub1_wallet_path, xpub1_descriptor, xpub1_change_descriptor, Network::Regtest)?;

    println!("Creating XPUB2 Wallet.");
    // XPUB 2 = Consolidates the inputs of the wallet into 100 utxos
    let xpub2_wallet_path = PathBuf::from("./wallet_data/regtest_xpub2.db");
    let xpub2_descriptor = "wpkh(tprv8gxoMPs1ky6faLrqSF4k1ARFJgd6pUjg5b4YrG5YmK1zPidubwypKvmBfUM4PhtgfMwfKPodVpPgTM7syKW7pRHfZ5U5DF83aYqzjLze4Wg/84'/1'/0'/0/*)";
    let xpub2_change_descriptor = "wpkh(tprv8gxoMPs1ky6faLrqSF4k1ARFJgd6pUjg5b4YrG5YmK1zPidubwypKvmBfUM4PhtgfMwfKPodVpPgTM7syKW7pRHfZ5U5DF83aYqzjLze4Wg/84'/1'/0'/1/*)";    
    let mut xpub2_wallet = BitcoinWallet::new(xpub2_wallet_path, xpub2_descriptor, xpub2_change_descriptor, Network::Regtest)?;
    
    // Generating Addresses
    let burn_address = Address::from_str(REGTEST_BURN_ADDRESS)?;
    let xpub_2_receive_address = xpub2_wallet.wallet.reveal_next_address(bdk_wallet::KeychainKind::External).address;
    let xpub_1_receive_address = xpub1_wallet.wallet.reveal_next_address(bdk_wallet::KeychainKind::External).address;

    // First lets send one blocks of rewards to our XPUB 1 wallet, then burn 100 more blocks of coinbase rewards.
    let _res = rpc_client.get_client().generate_to_address(1, &xpub_1_receive_address);
    let _res = rpc_client.get_client().generate_to_address(100, &burn_address.clone().assume_checked());

    // Create and mine 10,000 TXNs - Sync the XPUB 1 wallet
    for _ in 0..NUMBER_PER_BLOCK_TXNS {
        xpub1_wallet.sync_with_node(rpc_url.clone(), auth.clone(), 0)?;
        let mut tx_builder = xpub1_wallet.wallet.build_tx();
        tx_builder.add_recipient(xpub_2_receive_address.script_pubkey(), Amount::from_sat(777));
    
        let mut psbt = tx_builder
            .finish()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        let finalized = xpub1_wallet.wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        if !finalized {
            println!("Warning: Transaction not finalized after signing");
            continue;
        }
        
        let tx = psbt
            .extract_tx()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        match rpc_client.submit_transaction(&tx) {
            Ok(txid) => {
                println!("Transaction sent successfully: {}", txid);
            },
            Err(e) => {
                println!("Failed to send transaction: {}", e);
            }
        }

        let res = rpc_client.get_client().generate_to_address(1, &burn_address.clone().assume_checked());
        println!("Mined our TXN into the chain ... {res:?}");
    }
    
    // Print the stats of our XPUB2 wallet
    println!("Now syncing XPUB2 wallet with our regtest chain.");
    xpub2_wallet.sync_with_node(rpc_url.clone(), auth.clone(), 0)?;
    println!("XPUB2 Wallet contains currently {:?} UTXOs", xpub2_wallet.wallet.list_unspent().count());
    
    if xpub2_wallet.wallet.list_unspent().count() < NUMBER_PER_BLOCK_TXNS as usize {
        println!("You don't have enought UTXOS to consolidate 😊");
        return Ok(());
    }    

    println!("Syncing wallets for finality...");
    xpub1_wallet.sync_with_node(rpc_url.clone(), auth.clone(), 0)?;
    xpub2_wallet.sync_with_node(rpc_url.clone(), auth.clone(), 0)?;

    // Create the groups of UTXOs by UTXOS_PER_GROUP up to NUMBER_PER_BLOCK_TXNS
    let mut utxo_groups: Vec<Vec<LocalOutput>> = vec![];
    let mut utxo_group: Vec<LocalOutput> = vec![];
    let mut counter: u64 = 0;
    
    // Create NUMBER_PER_BLOCK_TXNS / UTXOS_PER_GROUP groups (100 groups of 100 UTXOs each)
    for utxo in xpub2_wallet.wallet.list_unspent() {
        utxo_group.push(utxo);
        counter += 1;
        if counter % UTXOS_PER_GROUP as u64 == 0 {
            utxo_groups.push(utxo_group);
            utxo_group = vec![];
            if utxo_groups.len() >= (NUMBER_PER_BLOCK_TXNS / UTXOS_PER_GROUP as u64) as usize {
                break;
            }
        }
        
        if counter >= NUMBER_PER_BLOCK_TXNS {
            break;
        }
    }
    
    println!("Created {} UTXO groups with approximately {} UTXOs per group", 
             utxo_groups.len(), 
             if !utxo_groups.is_empty() { utxo_groups[0].len() } else { 0 });
             
    // Process each group of UTXOs into a transaction
    println!("Building and sending transactions for each UTXO group...");
    let mut transactions_sent = 0;
    for utxo_group in utxo_groups {
        let receive_address = xpub2_wallet.wallet.reveal_next_address(bdk_wallet::KeychainKind::External).address;
        let mut tx_builder = xpub2_wallet.wallet.build_tx();
        tx_builder.add_recipient(receive_address.script_pubkey(), Amount::from_sat(777));
        println!("Adding UTXOs to transaction builder.");
        // Collect all outpoints from the UTXO group
        let outpoints: Vec<_> = utxo_group.iter().map(|utxo| utxo.outpoint).collect();
        
        // Add all UTXOs at once
        tx_builder.add_utxos(&outpoints)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        let mut psbt = tx_builder
            .finish()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        println!("Signing Transaction");
        let finalized = xpub2_wallet.wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        if !finalized {
            println!("Warning: Transaction not finalized after signing");
            continue;
        }
        
        let tx = psbt
            .extract_tx()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        match rpc_client.submit_transaction(&tx) {
            Ok(txid) => {
                println!("Transaction sent successfully: {}", txid);
                transactions_sent += 1;
            },
            Err(e) => {
                println!("Failed to send transaction: {}", e);
            }
        }

        let res = rpc_client.get_client().generate_to_address(1, &receive_address);
        println!("Mined our TXN into the chain ... {res:?}");
    }
    println!("Successfully sent {} transactions", transactions_sent);
    
    // Sync the wallet to see the new transactions
    println!("Syncing wallet to see new transactions...");
    xpub2_wallet.sync_with_node(rpc_url.clone(), auth.clone(), 0)?;

    // Now we need to send to 20 address with wallet number 2 mining a block each time
    for _ in 0..XPUB2_CONSOLIDATION_TXNS {
        xpub2_wallet.sync_with_node(rpc_url.clone(), auth.clone(), 0)?;
        
        let new_receive_address = xpub1_wallet.wallet.reveal_next_address(bdk_wallet::KeychainKind::External).address;
        let mut tx_builder = xpub1_wallet.wallet.build_tx();
        tx_builder.add_recipient(new_receive_address.script_pubkey(), Amount::from_sat(400));
    
        let mut psbt = tx_builder
            .finish()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        let finalized = xpub1_wallet.wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        if !finalized {
            println!("Warning: Transaction not finalized after signing");
            continue;
        }
        
        let tx = psbt
            .extract_tx()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
        match rpc_client.submit_transaction(&tx) {
            Ok(txid) => {
                println!("Transaction sent successfully: {}", txid);
                println!("Sent to address: {}", new_receive_address);
            },
            Err(e) => {
                println!("Failed to send transaction: {}", e);
            }
        }

        let res = rpc_client.get_client().generate_to_address(1, &burn_address.clone().assume_checked());
        println!("Mined our TXN into the chain ... {res:?}");
    }
    
    // Print out the final balances and various chain stats
    println!("WALLET STATS\r\n\tXPUB1: {:?}\r\n\tXPUB2: {:?}", xpub1_wallet.get_balance(), xpub2_wallet.get_balance());

    Ok(())
}
