//MARK: ADDED NEWLY
use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

pub fn transfer_sol(recipient_pubkey_str: &str, amount_sol: f64) -> Result<String> {
    let rpc_url = "https://aged-wispy-fog.solana-mainnet.quiknode.pro/bf45d61303c3f02e67820331089a0b9382250983";
    let keypair_path = "../../../wome.json";
    // Connect to the Solana network
    let rpc_client =
        RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

    // Read the keypair from the JSON file
    let sender_keypair = read_keypair_file(keypair_path)
        .map_err(|e| anyhow!("Failed to read keypair file: {}", e))?;

    // Parse the recipient's public key
    let recipient_pubkey = Pubkey::from_str(recipient_pubkey_str)
        .map_err(|e| anyhow!("Invalid recipient public key: {}", e))?;

    // Convert SOL to lamports (1 SOL = 1_000_000_000 lamports)
    let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

    // Create the transfer instruction
    let instruction =
        system_instruction::transfer(&sender_keypair.pubkey(), &recipient_pubkey, amount_lamports);

    // Get a recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

    // Create and sign the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&sender_keypair.pubkey()),
        &[&sender_keypair],
        recent_blockhash,
    );

    // Send and confirm the transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;

    Ok(signature.to_string())
}
