use std::str::FromStr;

use crate::{error::EscrowCliError, helper::derive_pdas};
use colored::*;
use escrow_native::state::Escrow;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_token::{
    amount_to_ui_amount,
    state::{Account, Mint},
};
pub async fn view_escrow(
    network: &str,
    program_id: &str,
    maker_str: &str,
    escrow_id: u64,
    client: &RpcClient,
) -> Result<(), EscrowCliError> {
    // println!("\n{}", "========== VIEW ESCROW ==========".bold().blue());
    let program_id = Pubkey::from_str(program_id)
        .map_err(|e| EscrowCliError::InvalidProgramId(e.to_string()))?;
    let maker = Pubkey::from_str(maker_str)
        .map_err(|e| EscrowCliError::InvalidPubkey(format!("Invalid maker address: {}", e)))?;
    // Derive PDAs
    let (vault_pda, escrow_pda) = derive_pdas(&program_id, &maker, escrow_id);
    // Fetch and parse escrow state account
    let escrow_account = client.get_account(&escrow_pda).await.map_err(|e| {
        EscrowCliError::CustomError(format!("failed to get the escrow account:{}", e))
    })?;
    let escrow_data = Escrow::unpack_the_slice_data(&escrow_account.data).map_err(|e| {
        EscrowCliError::CustomError(format!("Failed to deserialize escrow data: {:?}", e))
    })?;

    let mint_a_acc = client
        .get_account(&escrow_data.token_mint_a)
        .await
        .map_err(|e| {
            EscrowCliError::TokenAccountNotFound(format!("Failed to get Mint A Account:{}", e))
        })?;
    let mint_a_data = Mint::unpack(&mint_a_acc.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to get Mint A Account Data:{}", e))
    })?;
    // Token B info (what maker wants)
    let mint_b_acc = client
        .get_account(&escrow_data.token_mint_b)
        .await
        .map_err(|e| {
            EscrowCliError::TokenAccountNotFound(format!("Failed to get Mint B Account:{}", e))
        })?;
    let mint_b_data = Mint::unpack(&mint_b_acc.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to get Mint B Account Data:{}", e))
    })?;

    // Fetch vault account to see deposited amount
    let vault_account = client.get_account(&vault_pda).await.map_err(|e| {
        EscrowCliError::CustomError(format!("Failed to get the vault account:{}", e))
    })?;
    let vault_data = Account::unpack(&vault_account.data).map_err(|e| {
        EscrowCliError::TokenAccountNotFound(format!("Failed to parse vault: {}", e))
    })?;
    // let token_a_amount = amount_to_ui_amount(vault_data.amount, mint_a_data.decimals);

    // println!("{}", "\nEscrow State:".bold().green());
    // println!("Maker: {}", escrow_data.maker.to_string().bright_white());
    // println!("Escrow ID: {}", escrow_data.escrow_id);
    // println!(
    //     "\n{}",
    //     "========== VAULT DETAILS ==========".bold().bright_green()
    // );
    // println!("{}", "Vault account found:".bold().red());
    // println!("   Mint: {}", vault_data.mint);
    // println!("   Owner (Vault PDA): {}", vault_data.owner);
    // println!(
    //     "   Locked Balance: {} ({}) tokens",
    //     token_a_amount, vault_data.amount
    // );
    // println!("   State: {:?}", vault_data.state);
    // println!(
    //     "\n{}",
    //     "========== EXCHANGE SUMMARY =========="
    //         .bold()
    //         .bright_green()
    // );
    // println!(
    //     "   Offering: {} ({}) Token A",
    //     token_a_amount, vault_data.amount
    // );
    // println!(
    //     "   Requesting: {} ({}) Token B",
    //     amount_to_ui_amount(escrow_data.receive, mint_b_data.decimals),
    //     escrow_data.receive
    // );

    // println!("\nEscrow is ACTIVE and ready for exchange!");
    // println!("\nBlockchain Explorer:");
    // println!(
    //     "Escrow: {}",
    //     format!(
    //         "https://explorer.solana.com/address/{}?cluster={}",
    //         escrow_pda, network
    //     )
    //     .underline()
    //     .cyan()
    // );
    // println!(
    //     "Vault: {}",
    //     format!(
    //         "https://explorer.solana.com/address/{}?cluster={}",
    //         vault_pda, network
    //     )
    //     .underline()
    //     .cyan()
    // );
    let offered_amount = amount_to_ui_amount(vault_data.amount, mint_a_data.decimals);
    let requested_amount = amount_to_ui_amount(escrow_data.receive, mint_b_data.decimals);
    let separator =
        "────────────────────────────────────────────────────────────".truecolor(90, 90, 90);

    // Header box (exact width: 60 chars)
    println!(
        "\n{}",
        "╔════════════════════════════════════════════════════════════╗"
            .blue()
            .bold()
    );
    println!(
        "{}",
        "║                     VIEW ESCROW DETAILS                    ║"
            .blue()
            .bold()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════════════════════════╝"
            .blue()
            .bold()
    );

    // ESCROW METADATA
    println!("\n{}", "Escrow".white().bold());
    println!(
        "  {:<14} {}",
        "ID:".dimmed(),
        escrow_data.escrow_id.to_string().yellow().bold()
    );
    println!(
        "  {:<14} {}",
        "Maker:".dimmed(),
        escrow_data.maker.to_string().cyan()
    );
    println!(
        "  {:<14} {}",
        "Address:".dimmed(),
        escrow_pda.to_string().bright_blue()
    );
    println!("{}", separator);

    // EXCHANGE TERMS
    println!("\n{}", "Exchange Terms".white().bold());

    // OFFERING SIDE
    println!("\n{} {}", "OFFERING".green().bold(), "(Locked in Vault)");
    println!(
        "    {:<12} {} Tokens",
        "Amount:".dimmed(),
        offered_amount.to_string().bold()
    );
    println!(
        "    {:<12} {}",
        "Mint A:".dimmed(),
        escrow_data.token_mint_a
    );

    // REQUESTING SIDE
    println!("\n{} {}", "REQUESTING".magenta().bold(), "(To Withdraw)");
    println!(
        "    {:<12} {} Tokens",
        "Amount:".dimmed(),
        requested_amount.to_string().bold()
    );
    println!(
        "    {:<12} {}",
        "Mint B:".dimmed(),
        escrow_data.token_mint_b
    );
    println!("{}", separator);

    // VAULT STATUS
    println!("\n{}", "Vault Status".white().bold());
    println!("  {:<14} {}", "PDA:".dimmed(), vault_pda);

    let vault_state = format!("{:?}", vault_data.state).to_uppercase();

    if vault_data.amount > 0 {
        println!(
            "  {:<14} {}",
            "Status:".dimmed(),
            vault_state.green().bold()
        );
        println!(
            "  {:<14} {} (Raw units: {})",
            "Balance:".dimmed(),
            offered_amount,
            vault_data.amount
        );
    } else {
        println!(
            "  {:<14} {}",
            "Status:".dimmed(),
            "EMPTY / INACTIVE".red().bold()
        );
    }

    println!("{}", separator);

    let base_url = "https://explorer.solana.com/address";
    let escrow_link = format!("{}/{}?cluster={}", base_url, escrow_pda, network);
    let vault_link = format!("{}/{}?cluster={}", base_url, vault_pda, network);

    println!(
        "  {:<10} {}",
        "Escrow:".dimmed(),
        escrow_link.underline().blue()
    );
    println!(
        "  {:<10} {}",
        "Vault:".dimmed(),
        vault_link.underline().blue()
    );
    println!();

    Ok(())
}
