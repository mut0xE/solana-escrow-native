use std::str::FromStr;

use crate::{
    error::EscrowCliError,
    helper::{create_cancel_instruction, derive_pdas},
};
use colored::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use spl_token::state::Account;

pub async fn cancel_escrow(
    network: &str,
    maker: &Keypair,
    escrow_id: u64,
    program_id_str: &str,
    mint_a_str: &str,
    client: &RpcClient,
) -> Result<(), EscrowCliError> {
    println!("\n{}", "═══════════════════════════════════".bold().red());
    println!("{}", "        CANCEL ESCROW".bold().red());
    println!("{}", "═══════════════════════════════════".bold().red());

    let program_id = Pubkey::from_str(program_id_str)
        .map_err(|e| EscrowCliError::InvalidProgramId(e.to_string()))?;

    let mint_a =
        Pubkey::from_str(mint_a_str).map_err(|e| EscrowCliError::InvalidPubkey(e.to_string()))?;

    println!("\n{}", "Step 1: Derive PDAs".bold().cyan());

    let (vault_pda, escrow_pda) = derive_pdas(&program_id, &maker.pubkey(), escrow_id);
    println!("Vault PDA: {}", vault_pda);
    println!("Escrow PDA: {}", escrow_pda);
    println!("\n{}", "Step 2: Verify Escrow Exists".bold().cyan());
    let escrow_account = client
        .get_account(&escrow_pda)
        .await
        .map_err(|e| EscrowCliError::CustomError(format!("Escrow not found: {}", e)))?;
    println!("Escrow account found");
    println!("    Owner: {}", escrow_account.owner);
    println!("    Lamports: {}", escrow_account.lamports);
    println!("\n{}", "Step 3: Check Vault Balance".bold().cyan());
    let vault_account = client
        .get_account(&vault_pda)
        .await
        .map_err(|e| EscrowCliError::TokenAccountNotFound(format!("Vault not found: {}", e)))?;

    let vault_data = Account::unpack(&vault_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to parse vault: {}", e))
    })?;
    if vault_data.mint != mint_a {
        return Err(EscrowCliError::CustomError(format!(
            "Vault mint mismatch! Expected: {}, Got: {}",
            mint_a, vault_data.mint
        )));
    }
    println!("Vault account found");
    println!("    Mint: {}", vault_data.mint);
    println!("    Balance: {} tokens", vault_data.amount);
    println!("    Owner: {}", vault_data.owner);
    println!("\n{}", "Step 4: Find Maker's Token Account".bold().cyan());
    let maker_token_account = spl_associated_token_account::get_associated_token_address(
        &maker.pubkey(),
        &vault_data.mint,
    );

    println!("Maker's Token Account: {}", maker_token_account);
    client
        .get_account(&maker_token_account)
        .await
        .map_err(|_e| {
            EscrowCliError::TokenAccountNotFound("Maker's token account does not exist".to_string())
        })?;
    println!("\n{}", "Step 5: Build Cancel Instruction".bold().cyan());
    let cancel_instruction = create_cancel_instruction(
        &program_id,
        &maker.pubkey(),
        &mint_a,
        &vault_pda,
        &escrow_pda,
        &maker_token_account,
        escrow_id,
    );
    println!("\n{}", "Step 6: Send Transaction".bold().cyan());
    let recent_blockhash = client.get_latest_blockhash().await.map_err(|e| {
        EscrowCliError::NetworkConnection(format!("failed to get Recent blockhash:{}", e))
    })?;
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_instruction],
        Some(&maker.pubkey()),
        &[maker],
        recent_blockhash,
    );
    println!("  Sending transaction...");
    let signature = client
        .send_and_confirm_transaction(&cancel_tx)
        .await
        .map_err(|e| EscrowCliError::TransactionFailed(format!("Transaction failed: {}", e)))?;
    println!(
        "\n{}",
        "╔════════════════════════════════════════════════════╗"
            .green()
            .bold()
    );
    println!(
        "{}",
        "║        ✓ ESCROW CANCELLED SUCCESSFULLY!           ║"
            .green()
            .bold()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════════════════╝"
            .green()
            .bold()
    );

    println!("\n{}", "Transaction Details".bold().white());
    println!("  Transaction: {}", signature.to_string().yellow().bold());
    println!(
        "  Explorer: {}",
        format!(
            "https://explorer.solana.com/tx/{}?cluster={}",
            signature, network
        )
        .underline()
        .cyan()
    );

    println!("\n{}", "Result".bold().white());
    println!("  Your tokens have been returned to your Token account");
    println!("  Escrow account closed and rent reclaimed");
    println!("  Vault account closed");

    println!("\n{}", "Account Addresses".bold().white());
    println!(
        "  Escrow PDA: {}",
        format!(
            "https://explorer.solana.com/address/{}?cluster={}",
            escrow_pda, network
        )
        .underline()
        .cyan()
    );
    println!(
        "  Vault PDA: {}",
        format!(
            "https://explorer.solana.com/address/{}?cluster={}",
            vault_pda, network
        )
        .underline()
        .cyan()
    );

    Ok(())
}
