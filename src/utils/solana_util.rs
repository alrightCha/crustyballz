use serde_json;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use std::str::FromStr;

//MARK: ADDED NEWLY
fn transfer_sol(recipient_address: &str, amount_sol: f64) -> Result<(), Box<dyn Error>> {
    let keypair_path = "../../../wome.json";
    // Connect to Solana cluster
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let client = RpcClient::new(rpc_url);

    // Load keypair from the given path
    let keypair = read_keypair_file(keypair_path)?;

    // Convert SOL to lamports (the smallest unit of SOL)
    let lamports = (amount_sol * 1_000_000_000f64) as u64;

    // Create a transfer transaction
    let recipient_pubkey = recipient_address.parse()?;
    let transfer_instruction = solana_sdk::system_instruction::transfer(
        &keypair.pubkey(), &recipient_pubkey, lamports
    );
    let recent_blockhash = client.get_recent_blockhash()?.0;
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&keypair.pubkey()),
        &[&keypair],
        recent_blockhash
    );

    // Send the transaction
    client.send_and_confirm_transaction(&transaction)?;

    Ok(())
}
