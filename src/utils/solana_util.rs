use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey, signature::{read_keypair_file, Keypair}, signer::Signer, system_instruction, transaction::Transaction
};
use serde_json;
use std::str::FromStr;

//MARK: ADDED NEWLY
pub async fn transfer_sol(to_pubkey_str: String, amount_sol: f64) {
    if(to_pubkey_str == 'f'.to_string()){
        return
    }
    let from_keypair_path = "../../../wome.json";

    // Replace with your RPC URL
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let client = RpcClient::new(rpc_url);

    // Read the keypair from a JSON file
    let from_keypair = read_keypair_file(from_keypair_path)
        .expect("Failed to read keypair from file");

    // Convert SOL to lamports
    let lamports = solana_sdk::native_token::sol_to_lamports(amount_sol);

    // Convert string to Pubkey
    let to_pubkey = Pubkey::from_str(&to_pubkey_str)
        .expect("Failed to create pubkey from string");

    // Create transfer instruction
    let transfer_instruction = system_instruction::transfer(
        &from_keypair.try_pubkey(),
        &to_pubkey, 
        lamports
    );

    // Create the transaction
    let mut transaction = Transaction::new_with_payer(
        &[transfer_instruction], 
        Some(&from_keypair.try_pubkey())
    );

    // Fetch recent blockhash
    let recent_blockhash = client.get_recent_blockhash()
        .expect("Failed to get recent blockhash")
        .0;

    // Sign the transaction
    transaction.try_sign(&[&from_keypair], recent_blockhash)
        .expect("Failed to sign transaction");

    // Send the transaction
    let signature = client.send_and_confirm_transaction(&transaction)
        .expect("Failed to send transaction");

    println!("Transaction sent with signature: {}", signature);
}