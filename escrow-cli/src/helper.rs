use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_program, sysvar,
    transaction::Transaction,
};
use spl_token::{state::Account, ID as TOKEN_PROGRAM_ID};

use crate::error::EscrowCliError;

//Connect to Solana Network
/// Create RPC client for the specified network

pub fn connect_to_network(network: &str) -> Result<RpcClient, EscrowCliError> {
    let url = match network {
        "devnet" => "https://api.devnet.solana.com",
        "testnet" => "https://api.testnet.solana.com",
        "mainnet" => "https://api.mainnet-beta.solana.com",
        "localhost" => "http://localhost:8899",
        custom => custom,
    };
    // Create RPC client with "confirmed" commitment level
    Ok(RpcClient::new_with_commitment(
        url.to_string(),
        CommitmentConfig::confirmed(),
    ))
}
/// Load wallet keypair from file
pub fn get_wallet(path: &str) -> Result<Keypair, EscrowCliError> {
    println!("Loading wallet from: {}", path);
    read_keypair_file(path)
        .map_err(|e| EscrowCliError::WalletLoad(format!("Failed to read keypair: {}", e)))
}

/// Check SOL balance of a wallet
pub async fn check_sol_balance(client: &RpcClient, wallet: &Pubkey) -> Result<u64, EscrowCliError> {
    client
        .get_balance(&wallet)
        .await
        .map_err(|e| EscrowCliError::RpcError(format!("Failed to get balance: {}", e)))
}

/// Check if token account exists and has sufficient balance
pub async fn check_token_account(
    client: &RpcClient,
    account: &Pubkey,
    required_amount: u64,
) -> Result<u64, EscrowCliError> {
    let result = client
        .get_account(account)
        .await
        .map_err(|_| EscrowCliError::TokenAccountNotFound(account.to_string()))?;
    let token_account = Account::unpack(&result.data).map_err(|e| {
        EscrowCliError::CustomError(format!("Failed to unpack token account: {}", e))
    })?;
    if token_account.amount < required_amount {
        return Err(EscrowCliError::InsufficientTokens {
            needed: required_amount,
            actual: token_account.amount,
        });
    }
    Ok(token_account.amount)
}
/// Create associated token account if it doesn't exist
pub async fn ensure_token_account(
    client: &RpcClient,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
    token_name: &str,
) -> Result<Pubkey, EscrowCliError> {
    println!("Checking {} token account...", token_name);
    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &mint,
        &TOKEN_PROGRAM_ID,
    );
    // Check if account exists
    if client.get_account(&ata).await.is_ok() {
        println!("{} token account exists: {}", token_name, ata);
        return Ok(ata);
    }
    println!("Token account doesn't exist, creating...");

    // Create the account
    let create_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            &owner,
            &mint,
            &TOKEN_PROGRAM_ID,
        );
    let recent_blockhash = client
        .get_latest_blockhash()
        .await
        .map_err(|e| EscrowCliError::RpcError(format!("Failed to get blockhash: {}", e)))?;
    let ata_tx = Transaction::new_signed_with_payer(
        &[create_ata_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let signature = client
        .send_and_confirm_transaction(&ata_tx)
        .await
        .map_err(|e| EscrowCliError::TokenAccountCreation(format!("Transaction failed: {}", e)))?;
    println!("Created {} token account: {}", token_name, ata);
    println!("   Transaction: {}", signature);
    Ok(ata)
}
/// Derive escrow PDAs
pub fn derive_pdas(program_id: &Pubkey, maker: &Pubkey, escrow_id: u64) -> (Pubkey, Pubkey) {
    let escrow_seed = escrow_id.to_le_bytes();
    let vault_seeds: &[&[u8]] = &[b"vault", maker.as_ref(), escrow_seed.as_ref()];

    let vault_pda = Pubkey::find_program_address(vault_seeds, program_id).0;
    let escrow_pda = Pubkey::find_program_address(
        &[b"escrow_vault", maker.as_ref(), &escrow_id.to_le_bytes()],
        &program_id,
    )
    .0;
    (vault_pda, escrow_pda)
}
pub fn create_initialize_escrow_instruction(
    maker: &Keypair,
    escrow_id: u64,
    deposit_amount: u64,
    receive_amount: u64,
    program_id: &Pubkey,
    token_mint_a: &Pubkey,
    token_mint_b: &Pubkey,
    vault_pda: &Pubkey,
    escrow_pda: &Pubkey,
    maker_token_acc_a: &Pubkey,
    maker_token_acc_b: &Pubkey,
) -> Instruction {
    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&escrow_id.to_le_bytes());
    instruction_data.extend_from_slice(&deposit_amount.to_le_bytes());
    instruction_data.extend_from_slice(&receive_amount.to_le_bytes());

    let instruction = Instruction {
        program_id: *program_id, // The program to call
        accounts: vec![
            // Accounts the program needs
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(*token_mint_a, false),
            AccountMeta::new_readonly(*token_mint_b, false),
            AccountMeta::new(*maker_token_acc_a, false),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new(*maker_token_acc_b, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: instruction_data, // Instruction data
    };
    instruction
}
pub fn create_cancel_instruction(
    program_id: &Pubkey,
    maker: &Pubkey,
    mint_a: &Pubkey,
    vault_pda: &Pubkey,
    escrow_pda: &Pubkey,
    maker_token_account: &Pubkey,
    escrow_id: u64,
) -> Instruction {
    let mut instruction_data = vec![2u8];
    instruction_data.extend_from_slice(&escrow_id.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*maker, true),
            AccountMeta::new_readonly(*mint_a, false),
            AccountMeta::new(*maker_token_account, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: instruction_data,
    }
}

pub fn create_exchange_instruction(
    program_id: &Pubkey,
    maker: &Pubkey,
    taker: &Keypair,
    vault_pda: &Pubkey,
    escrow_pda: &Pubkey,
    taker_token_a_acc: &Pubkey,
    taker_token_b_acc: &Pubkey,
    maker_token_b_acc: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    escrow_id: u64,
) -> Instruction {
    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&escrow_id.to_le_bytes());
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),
            AccountMeta::new(*maker, false),
            AccountMeta::new_readonly(*mint_a, false),
            AccountMeta::new_readonly(*mint_b, false),
            AccountMeta::new(*maker_token_b_acc, false),
            AccountMeta::new(*taker_token_a_acc, false),
            AccountMeta::new(*taker_token_b_acc, false),
            AccountMeta::new(*vault_pda, false),
            AccountMeta::new(*escrow_pda, false),
            AccountMeta::new(TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    }
}
