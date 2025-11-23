use std::str::FromStr;

use crate::{
    error::EscrowCliError,
    helper::{
        check_token_account, create_initialize_escrow_instruction, derive_pdas,
        ensure_token_account,
    },
};
use colored::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use spl_token::amount_to_ui_amount;

pub async fn initialize_escrow(
    network: &str,
    program_id_str: &str,
    maker: &Keypair,
    mint_a_str: &str,
    mint_b_str: &str,
    deposit_amount: u64,
    receive_amount: u64,
    escrow_id: u64,
    client: &RpcClient,
) -> Result<(), EscrowCliError> {
    println!("\n{}", "═══════════════════════════════════".bold().blue());
    println!("{}", "      INITIALIZE ESCROW".bold().blue());
    println!("{}", "═══════════════════════════════════".bold().blue());
    let program_id = Pubkey::from_str(program_id_str)
        .map_err(|e| EscrowCliError::InvalidProgramId(e.to_string()))?;

    let mint_a =
        Pubkey::from_str(mint_a_str).map_err(|e| EscrowCliError::InvalidPubkey(e.to_string()))?;

    let mint_b =
        Pubkey::from_str(mint_b_str).map_err(|e| EscrowCliError::InvalidPubkey(e.to_string()))?;

    // Get or create token accounts
    let maker_token_a_acc =
        ensure_token_account(&client, maker, &maker.pubkey(), &mint_a, "Token A")
            .await
            .map_err(|e| {
                EscrowCliError::TokenAccountCreation(format!(
                    "Failed to setup Token A account: {}",
                    e
                ))
            })?;
    let maker_token_b_acc =
        ensure_token_account(&client, maker, &maker.pubkey(), &mint_b, "Token B")
            .await
            .map_err(|e| {
                EscrowCliError::TokenAccountCreation(format!(
                    "Failed to setup Token A account: {}",
                    e
                ))
            })?;
    // Check Token A balance
    check_token_account(client, &maker_token_a_acc, deposit_amount).await?;
    // Derive PDAs
    let (vault_pda, escrow_pda) = derive_pdas(&program_id, &maker.pubkey(), escrow_id);
    println!("\n{}", "Derive PDAs".bold().cyan());
    println!("  Vault PDA: {}", vault_pda.to_string().magenta());
    println!("  Escrow PDA: {}", escrow_pda.to_string().bright_magenta());

    println!("\n{}", "Send Transaction".bold().cyan());

    // Build initialize instruction
    let init_instruction = create_initialize_escrow_instruction(
        &maker,
        escrow_id,
        deposit_amount,
        receive_amount,
        &program_id,
        &mint_a,
        &mint_b,
        &vault_pda,
        &escrow_pda,
        &maker_token_a_acc,
        &maker_token_b_acc,
    );

    let recent_blockhash = client
        .get_latest_blockhash()
        .await
        .map_err(|e| EscrowCliError::RpcError(format!("failed to get latest blockhash:{}", e)))?;
    let init_tx = Transaction::new_signed_with_payer(
        &[init_instruction],
        Some(&maker.pubkey()),
        &[&maker],
        recent_blockhash,
    );

    let signature = client
        .send_and_confirm_transaction(&init_tx)
        .await
        .map_err(|e| {
            EscrowCliError::TransactionFailed(format!("failed to send transaction:{}", e))
        })?;
    println!(
        "\n{}",
        "╔════════════════════════════════════════════════════╗"
            .green()
            .bold()
    );
    println!(
        "{}",
        "║     ✓ ESCROW INITIALIZED SUCCESSFULLY!              ║"
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

    println!("\n{}", "Escrow Information".bold().white());
    println!(
        "  Escrow ID: {}",
        escrow_id.to_string().bright_cyan().bold()
    );
    println!("  Vault PDA: {}", vault_pda.to_string().magenta());
    println!("  Escrow PDA: {}", escrow_pda.to_string().bright_magenta());

    println!("\n{}", "Vault Details".bold().white());
    let vault_account = client
        .get_account(&vault_pda)
        .await
        .map_err(|e| EscrowCliError::TokenAccountNotFound(format!("vault not found:{}", e)))?;
    let vault_data = spl_token::state::Account::unpack(&vault_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("vault account not found:{}", e))
    })?;
    let mint_a_account = client
        .get_account(&mint_a)
        .await
        .map_err(|e| EscrowCliError::TokenAccountNotFound(format!("Mint A not found: {}", e)))?;

    let mint_a_data = spl_token::state::Mint::unpack(&mint_a_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to parse Mint A: {}", e))
    })?;
    let offered_amount = amount_to_ui_amount(vault_data.amount, mint_a_data.decimals);

    println!("  Mint: {}", vault_data.mint.to_string().green());
    println!(
        "  Balance: {} tokens",
        offered_amount.to_string().green().bold()
    );
    println!("  Owner: {}", vault_data.owner.to_string().magenta());
    println!("  State: {}", format!("{:?}", vault_data.state).blue());

    println!("\n{}", "Summary".bold().white());
    println!("  Your tokens are locked in the escrow vault and waiting for a taker!");
    println!("  Maker address:");
    println!(
        "    {}",
        maker.pubkey().to_string().bright_cyan().underline()
    );

    Ok(())
}
