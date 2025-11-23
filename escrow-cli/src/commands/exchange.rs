use std::str::FromStr;

use escrow_native::state::Escrow;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use spl_token::{amount_to_ui_amount, state::Account};

use crate::{
    error::EscrowCliError,
    helper::{check_token_account, create_exchange_instruction, derive_pdas, ensure_token_account},
};
use colored::*;
pub async fn exchange_funds(
    network: &str,
    program_id_str: &str,
    taker: &Keypair,
    maker: &Keypair,
    escrow_id: u64,
    client: &RpcClient,
) -> Result<(), EscrowCliError> {
    println!("\n{}", "═══════════════════════════════════".bold().green());
    println!("{}", "        EXCHANGE ESCROW (TAKE)".bold().green());
    println!("{}", "═══════════════════════════════════".bold().green());
    let program_id = Pubkey::from_str(program_id_str)
        .map_err(|e| EscrowCliError::InvalidProgramId(e.to_string()))?;

    // Derive PDAs
    let (vault_pda, escrow_pda) = derive_pdas(&program_id, &maker.pubkey(), escrow_id);
    println!("  Vault PDA: {}", vault_pda);
    println!("  Escrow PDA: {}", escrow_pda);
    println!("\n{}", "Step 2: Fetch Escrow Details".bold().cyan());
    let escrow_account = client
        .get_account(&escrow_pda)
        .await
        .map_err(|e| EscrowCliError::CustomError(format!("Escrow not found: {}", e)))?;
    let escrow_data = Escrow::unpack_the_slice_data(&escrow_account.data).map_err(|e| {
        EscrowCliError::CustomError(format!("Failed to deserialize escrow data: {:?}", e))
    })?;

    println!("  Escrow found!");
    println!("    Maker: {}", escrow_data.maker);
    println!("    Token A Mint: {}", escrow_data.token_mint_a);
    println!("    Token B Mint: {}", escrow_data.token_mint_b);
    println!("\n{}", "Step 3: Get Token Decimals".bold().cyan());
    let mint_a_account = client
        .get_account(&escrow_data.token_mint_a)
        .await
        .map_err(|e| EscrowCliError::TokenAccountNotFound(format!("Mint A not found: {}", e)))?;

    let mint_a_data = spl_token::state::Mint::unpack(&mint_a_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to parse Mint A: {}", e))
    })?;
    let mint_b_account = client
        .get_account(&escrow_data.token_mint_b)
        .await
        .map_err(|e| EscrowCliError::TokenAccountNotFound(format!("Mint B not found: {}", e)))?;

    let mint_b_data = spl_token::state::Mint::unpack(&mint_b_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to parse Mint B: {}", e))
    })?;
    println!("\n{}", "Step 4: Fetch Vault Details".bold().cyan());
    let vault_account = client
        .get_account(&vault_pda)
        .await
        .map_err(|e| EscrowCliError::TokenAccountNotFound(format!("Vault not found: {}", e)))?;
    let vault_data = Account::unpack(&vault_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to parse vault: {}", e))
    })?;

    let offered_amount = amount_to_ui_amount(vault_data.amount, mint_a_data.decimals);
    let requested_amount = amount_to_ui_amount(escrow_data.receive, mint_b_data.decimals);
    println!("  Token A decimals: {}", mint_a_data.decimals);
    println!("  Token B decimals: {}", mint_b_data.decimals);

    // Display exchange terms
    println!("\n{}", "Step 5: Exchange Terms".bold().cyan());
    println!(
        "  You will RECEIVE: {} Token A",
        offered_amount.to_string().yellow().bold()
    );
    println!(
        "  You will SEND: {} Token B",
        requested_amount.to_string().yellow().bold()
    );
    // Check/create taker's token accounts
    println!("\n{}", "Step 6: Setup Taker Token Accounts".bold().cyan());
    // Taker needs Token A account (to receive)
    let taker_token_a_acc = ensure_token_account(
        &client,
        &taker,
        &taker.pubkey(),
        &escrow_data.token_mint_a,
        "Token A",
    )
    .await
    .map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Taker A Account Not Found:{}", e))
    })?;
    println!("  Token A account: {}", taker_token_a_acc);

    // Taker needs Token B account (to send)
    let taker_token_b_acc = ensure_token_account(
        client,
        taker,
        &taker.pubkey(),
        &escrow_data.token_mint_b,
        "Token B (send)",
    )
    .await
    .map_err(|e| {
        EscrowCliError::TokenAccountCreation(format!("Failed to setup Token B account: {}", e))
    })?;
    println!("  Token B account: {}", taker_token_b_acc);
    // Verify taker has enough Token B
    let taker_token_b_data = check_token_account(&client, &taker_token_b_acc, escrow_data.receive)
        .await
        .map_err(|e| {
            EscrowCliError::CustomError(format!("Failed to get taker token b balance:{}", e))
        })?;
    println!(
        "  Balance: {} Token B (needed: {})",
        amount_to_ui_amount(taker_token_b_data, mint_b_data.decimals),
        requested_amount
    );
    println!("  Sufficient balance confirmed!");
    let maker_token_b_acc = ensure_token_account(
        &client,
        maker,
        &maker.pubkey(),
        &escrow_data.token_mint_b,
        "Token B",
    )
    .await
    .map_err(|e| {
        EscrowCliError::TokenAccountCreation(format!("Failed to setup Token A account: {}", e))
    })?;

    println!("\n{}", "Step 7: Verify Vault Has Tokens".bold().cyan());
    if vault_data.amount == 0 {
        return Err(EscrowCliError::CustomError(
            "Vault is empty - escrow has already been taken or cancelled".to_string(),
        ));
    }
    println!("  Vault contains {} tokens", vault_data.amount);
    // Build exchange instruction
    println!("\n{}", "Step 8: Build Exchange Instruction".bold().cyan());
    let exchange_instruction = create_exchange_instruction(
        &program_id,
        &maker.pubkey(),
        &taker,
        &vault_pda,
        &escrow_pda,
        &taker_token_a_acc,
        &taker_token_b_acc,
        &maker_token_b_acc,
        &escrow_data.token_mint_a,
        &escrow_data.token_mint_b,
        escrow_id,
    );
    println!("\n{}", "Step 9: Send Transaction".bold().cyan());
    let recent_blockhash = client
        .get_latest_blockhash()
        .await
        .map_err(|e| EscrowCliError::RpcError(format!("Failed to get blockhash: {}", e)))?;
    let exchange_tx = Transaction::new_signed_with_payer(
        &[exchange_instruction],
        Some(&taker.pubkey()),
        &[taker],
        recent_blockhash,
    );
    println!("  Sending transaction...");
    let signature = client
        .send_and_confirm_transaction(&exchange_tx)
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
        "║        ✓ EXCHANGE COMPLETED SUCCESSFULLY!         ║"
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

    println!("\n{}", "Exchange Summary".bold().white());
    println!(
        "  You received: {} Token A",
        offered_amount.to_string().green()
    );
    println!(
        "  You sent: {} Token B",
        requested_amount.to_string().yellow()
    );

    println!("\n{}", "Account Details".bold().white());
    println!("  Your Token A: {}", taker_token_a_acc);
    println!("  Your Token B: {}", taker_token_b_acc);
    println!("  Maker Token B: {}", maker_token_b_acc);
    println!("  Vault: {}", vault_pda);
    Ok(())
}
