use escrow_native::state::Escrow;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    system_program, sysvar,
    transaction::Transaction,
};
use std::str::FromStr;
const PROGRAM_ID: &str = "YOUR_PROGRAM_ID";
const MAKER: &str = "YOUR_KEYPAIR";
const TOKEN_MINT_A: &str = "TOKEN_MINT_A_KEYPAIR";
const TOKEN_MINT_B: &str = "TOKEN_MINT_B_KEYPAIR";

use spl_token::{state::Account, ID as TOKEN_PROGRAM_ID};
#[tokio::main]
async fn main() {
    // Replace with your actual program ID from deployment
    let program_id = Pubkey::from_str(PROGRAM_ID).expect("Invalid program ID");

    // Connect to local cluster
    let rpc_url = String::from("https://api.devnet.solana.com");
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Generate a new keypair for paying fees
    let payer = read_keypair_file(MAKER).expect("failed to read auth keypair");
    let token_mint_a =
        read_keypair_file(TOKEN_MINT_A).expect("failed to read token mint A keypair");
    let token_mint_b =
        read_keypair_file(TOKEN_MINT_B).expect("failed to read token mint B keypair");

    println!(" Loaded keypairs");
    println!("   Maker: {}", payer.pubkey());
    println!("   Mint A: {}", token_mint_a.pubkey());
    println!("   Mint B: {}", token_mint_b.pubkey());
    let maker_key = payer.pubkey();
    let escrow_id: u64 = 10;
    // STEP 6: Derive PDAs for Vault & Escrow
    let seeds = &[b"vault", maker_key.as_ref(), &escrow_id.to_le_bytes()];
    // Vault PDA - holds the deposited tokens
    let (vault_pda, _bump) = Pubkey::find_program_address(seeds, &program_id);
    // Escrow PDA - stores escrow state data
    let (escrow_pda, _escrow_bump) = Pubkey::find_program_address(
        &[
            b"escrow_vault",
            maker_key.as_ref(),
            &escrow_id.to_le_bytes(),
        ],
        &program_id,
    );

    println!(" Derived PDAs");
    println!("   Vault PDA: {}", vault_pda);
    println!("   Escrow PDA: {}", escrow_pda);
    let deposit_amount: u64 = 1_000_000_000; // 0.1 tokens
    let receive_amount: u64 = 2_000_000_000; // 0.2 tokens
    println!("  Escrow details:");
    println!(
        "   Deposit: {} Token A",
        deposit_amount as f64 / 1_000_000_000.0
    );
    println!(
        "   Receive: {} Token B",
        receive_amount as f64 / 1_000_000_000.0
    );
    println!("   ID: {}\n", escrow_id);
    let maker_token_acc_a =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &token_mint_a.pubkey(),
            &TOKEN_PROGRAM_ID,
        );
    let maker_token_acc_b =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &token_mint_b.pubkey(),
            &TOKEN_PROGRAM_ID,
        );
    println!(
        "maker token account A:{},maker token account B:{}",
        maker_token_acc_a, maker_token_acc_b,
    );
    // Check if maker_token_acc_b exists
    println!("\n Checking if Maker Token Account B exists...");
    match client.get_account(&maker_token_acc_b) {
        Ok(account) => {
            // Account exists!
            println!(" Maker Token Account B already exists");
            println!("   Owner: {}", account.owner);
            println!("   Lamports: {}", account.lamports);
        }
        Err(_) => {
            // Account doesn't exist, create it
            println!("Maker Token Account B doesn't exist");
            println!("Creating Maker Token Account B...");

            let create_ata_b_ix =
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer.pubkey(),        // payer
                    &payer.pubkey(),        // wallet owner
                    &token_mint_b.pubkey(), // mint
                    &TOKEN_PROGRAM_ID,      // token program
                );

            let mut tx = Transaction::new_with_payer(&[create_ata_b_ix], Some(&payer.pubkey()));

            let blockhash = client
                .get_latest_blockhash()
                .expect("Failed to get blockhash");
            tx.sign(&[&payer], blockhash);

            match client.send_and_confirm_transaction(&tx) {
                Ok(signature) => {
                    println!(" Successfully created Maker Token Account B");
                    println!("   Transaction: {}", signature);
                }
                Err(e) => {
                    eprintln!("❌ Failed to create Token Account B: {}", e);
                    return;
                }
            }
        }
    }

    let initialize_instruction = create_initialize_escrow_instruction(
        &payer,
        escrow_id,
        deposit_amount,
        receive_amount,
        program_id,
        token_mint_a.pubkey(),
        token_mint_b.pubkey(),
        vault_pda,
        escrow_pda,
        maker_token_acc_a,
        maker_token_acc_b,
    );

    let mut transaction =
        Transaction::new_with_payer(&[initialize_instruction], Some(&payer.pubkey()));

    let blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");
    transaction.sign(&[&payer], blockhash);

    match client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            println!("Escrow initialized!");
            println!("Transaction: {}", signature);
            println!("Payer address: {}", payer.pubkey());
        }
        Err(err) => {
            eprintln!("Failed to initialize counter: {}", err);
            return;
        }
    }
    let vault_account = client
        .get_account(&vault_pda)
        .expect("failed to get vault account");
    let vault_data =
        Account::unpack(&vault_account.data).expect("failed to get vault account data");
    println!("vault account:{:#?}", vault_data);
    let escrow_account = client
        .get_account(&escrow_pda)
        .expect("failed to get escrow account");
    let escrow_data = Escrow::unpack_the_slice_data(&escrow_account.data)
        .expect("failed to deserialize escrow account");
    println!("escrow account:{:#?}", escrow_data);
}

fn create_initialize_escrow_instruction(
    maker: &Keypair,
    escrow_id: u64,
    deposit_amount: u64,
    receive_amount: u64,
    program_id: Pubkey,
    token_mint_a: Pubkey,
    token_mint_b: Pubkey,
    vault_pda: Pubkey,
    escrow_pda: Pubkey,
    maker_token_acc_a: Pubkey,
    maker_token_acc_b: Pubkey,
) -> Instruction {
    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&escrow_id.to_le_bytes());
    instruction_data.extend_from_slice(&deposit_amount.to_le_bytes());
    instruction_data.extend_from_slice(&receive_amount.to_le_bytes());

    let instruction = Instruction {
        program_id: program_id, // The program to call
        accounts: vec![
            // Accounts the program needs
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(token_mint_a, false),
            AccountMeta::new_readonly(token_mint_b, false),
            AccountMeta::new(maker_token_acc_a, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(maker_token_acc_b, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: instruction_data, // Instruction data
    };
    instruction
}
