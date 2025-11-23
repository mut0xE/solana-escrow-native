use std::str::FromStr;

use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    system_instruction::create_account,
    system_program, sysvar,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
const PROGRAM_ID: &str = "YOUR_PROGRAM_ID";
const MAKER: &str = "YOUR_KEYPAIR";
const TAKER: &str = "YOUR_KEYPAIR";
const TOKEN_MINT_A: &str = "TOKEN_MINT_A_KEYPAIR";
const TOKEN_MINT_B: &str = "TOKEN_MINT_B_KEYPAIR";

use spl_token::{
    instruction::{initialize_mint, mint_to},
    state::Mint,
    ID as TOKEN_PROGRAM_ID,
};

// Holds all setup data needed for an escrow test
pub struct EscrowTestSetup {
    pub svm: LiteSVM,
    pub program_id: Pubkey,
    pub maker: Keypair,
    pub taker: Keypair,
    pub maker_token_acc_a: Pubkey,
    pub maker_token_acc_b: Pubkey,
    pub taker_token_acc_a: Pubkey,
    pub taker_token_acc_b: Pubkey,
    pub mint_a_pubkey: Pubkey,
    pub mint_b_pubkey: Pubkey,
    pub vault_pda: Pubkey,
    pub escrow_pda: Pubkey,
    pub vault_bump: u8,
    pub escrow_bump: u8,
    pub token_a_decimals: u8,
    pub token_b_decimals: u8,
}
/// Holds escrow transaction parameters
pub struct EscrowParams {
    pub escrow_id: u64,
    pub deposit_amount: u64,
    pub receive_amount: u64,
}
//helper fns
/// Creates and initializes a token mint in LiteSVM
pub fn create_token_mint(
    svm: &mut LiteSVM,
    mint: &Keypair,
    decimals: u8,
    auth_payer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("started create token mint");
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);

    let mint_instruction = create_account(
        &auth_payer.pubkey(),
        &mint.pubkey(),
        rent,
        Mint::LEN as u64,
        &TOKEN_PROGRAM_ID,
    );
    let initialize_mint_instruction = initialize_mint(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        &auth_payer.pubkey(),
        Some(&auth_payer.pubkey()),
        decimals,
    )?;
    let tx = Transaction::new_signed_with_payer(
        &[mint_instruction, initialize_mint_instruction],
        Some(&auth_payer.pubkey()),
        &[&auth_payer, &mint],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx)
        .map_err(|e| format!("Failed to ...: {:?}", e))?;
    println!(
        "Created Mint:\n  Mint: {}\n  Authority: {}\n  Decimals: {}\n",
        mint.pubkey(),
        auth_payer.pubkey(),
        decimals
    );

    Ok(())
}
/// Creates and initializes a token ata account
pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    println!("started creating token account...");

    // Derive the associated token account address for fee_payer
    let associated_token_account = get_associated_token_address(
        &payer.pubkey(), // owner
        &mint,           // mint
    );
    // Instruction to create associated token account
    let create_ata_instruction = create_associated_token_account(
        &payer.pubkey(),   // funding address
        &owner,            // wallet address (owner)
        &mint,             // mint address
        &TOKEN_PROGRAM_ID, // program id
    );
    // Create transaction for associated token account creation
    let transaction = Transaction::new_signed_with_payer(
        &[create_ata_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(transaction)
        .map_err(|e| format!("Failed to ...: {:?}", e))?;

    println!(
        "Created ATA:\n  Owner: {}\n  Mint:  {}\n  ATA:   {}\n",
        owner, mint, associated_token_account
    );

    Ok(associated_token_account)
}
/// Mints tokens to a token account
pub fn mint_tokens_to(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    amount: u64,
    ata: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("started mint to");

    // Create mint_to instruction to mint tokens to the associated token account
    let mint_to_instruction = mint_to(
        &TOKEN_PROGRAM_ID,
        &mint,              // mint
        &ata,               // destination
        &payer.pubkey(),    // authority
        &[&payer.pubkey()], // signer
        amount,             // amount
    )?;
    // Create transaction for minting tokens
    let transaction = Transaction::new_signed_with_payer(
        &[mint_to_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(transaction)
        .map_err(|e| format!("Failed to ...: {:?}", e))?;

    Ok(())
}
/// Setup complete token environment (mint + token account + tokens)
pub fn setup_token_with_account(
    svm: &mut LiteSVM,
    mint_keypair: &Keypair,
    owner: &Keypair,
    initial_amount: u64,
) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
    println!("started setup token with account");
    // Create mint
    create_token_mint(svm, mint_keypair, 9, &owner)?;

    // Create token account
    let ata = create_token_account(svm, &owner, &mint_keypair.pubkey(), &owner.pubkey())?;

    // Mint tokens if requested
    if initial_amount > 0 {
        mint_tokens_to(svm, &owner, &mint_keypair.pubkey(), initial_amount, &ata)?;
    }
    println!("done setup token with account");

    Ok((mint_keypair.pubkey(), ata))
}
/// Derive and verify PDAs
pub fn derive_pdas(
    maker_key: &Pubkey,
    escrow_id: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8, Pubkey, u8) {
    let vault_seeds = &[b"vault", maker_key.as_ref(), &escrow_id.to_le_bytes()];
    let (vault_pda, vault_bump) = Pubkey::find_program_address(vault_seeds, program_id);

    let escrow_seeds = &[
        b"escrow_vault",
        maker_key.as_ref(),
        &escrow_id.to_le_bytes(),
    ];
    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(escrow_seeds, program_id);

    println!("PDAs derived:");
    println!("  Vault PDA: {} (bump: {})", vault_pda, vault_bump);
    println!("  Escrow PDA: {} (bump: {})", escrow_pda, escrow_bump);

    (vault_pda, vault_bump, escrow_pda, escrow_bump)
}

/// Initialize LiteSVM and deploy program
pub fn setup_svm_and_program() -> Result<(LiteSVM, Pubkey), Box<dyn std::error::Error>> {
    // STEP 1: Setup LiteSVM
    let mut svm = LiteSVM::new().with_sysvars();

    // Deploy your program to the test environment
    let program_id = Pubkey::from_str(PROGRAM_ID).expect("Failed to read Program ID");
    // Include bytes at compile time
    let program_bytes = include_bytes!("../../target/deploy/escrow_native.so");
    // Deploy from bytes
    svm.add_program(program_id, program_bytes);
    // Verify deployment
    assert!(
        svm.get_account(&program_id).is_some(),
        "Program account not created"
    );
    assert!(
        svm.get_account(&program_id).unwrap().executable,
        "Program not executable"
    );
    println!("Program verified at {}", program_id);
    Ok((svm, program_id))
}
/// Load keypairs from files
pub fn load_keypairs(
    maker_path: &str,
    mint_a_path: &str,
    mint_b_path: &str,
    taker_path: &str,
) -> Result<(Keypair, Keypair, Keypair, Keypair), Box<dyn std::error::Error>> {
    let maker = read_keypair_file(maker_path)?;
    let mint_a = read_keypair_file(mint_a_path)?;
    let mint_b = read_keypair_file(mint_b_path)?;
    let taker = read_keypair_file(taker_path)?;

    println!("Keypairs loaded:");
    println!("  Maker: {}", maker.pubkey());
    println!("  Mint A: {}", mint_a.pubkey());
    println!("  Mint B: {}", mint_b.pubkey());
    println!("  Taker: {}", taker.pubkey());

    Ok((maker, mint_a, mint_b, taker))
}
/// Perform complete test setup
pub fn setup_escrow_test(escrow_id: u64) -> Result<EscrowTestSetup, Box<dyn std::error::Error>> {
    // Setup SVM and program
    let (mut svm, program_id) = setup_svm_and_program()?;

    // Load keypairs
    let (maker, token_mint_a, token_mint_b, taker) =
        load_keypairs(MAKER, TOKEN_MINT_A, TOKEN_MINT_B, TAKER)?;

    // Airdrop SOL
    svm.airdrop(&maker.pubkey(), 5_000_000_000)
        .expect("Failed to airdrop SOL");
    println!("Airdropped 5 SOL to maker");
    svm.airdrop(&taker.pubkey(), 5_000_000_000)
        .expect("Failed to airdrop SOL");
    println!("Airdropped 5 SOL to maker");

    // Setup tokens
    let initial_amount = 1_000_000_000;
    let (mint_a_pubkey, maker_token_acc_a) =
        setup_token_with_account(&mut svm, &token_mint_a, &maker, initial_amount)?;
    // Verify maker has tokens in account A
    let maker_acc_a_info = svm
        .get_account(&maker_token_acc_a)
        .expect("Maker token account A not found");
    // Deserialize token account data
    let token_acc_data = spl_token::state::Account::unpack(&maker_acc_a_info.data)
        .expect("Failed to unpack token account");
    // Get decimals for Token A
    let mint_a_account = svm
        .get_account(&mint_a_pubkey)
        .expect("failed to get mint A");
    let mint_a_data =
        spl_token::state::Mint::unpack(&mint_a_account.data).expect("failed to unpack mint a");
    let token_a_decimals = mint_a_data.decimals;
    println!(
        "Maker Token Account A:\n  Mint: {}\n  ATA:  {}\n  Amount: {} ({})\n",
        mint_a_pubkey,
        maker_token_acc_a,
        token_acc_data.amount,
        to_ui_amount(token_acc_data.amount, token_a_decimals)
    );
    let (mint_b_pubkey, maker_token_acc_b) =
        setup_token_with_account(&mut svm, &token_mint_b, &maker, 0)?;
    // Verify maker has tokens in account A
    let maker_acc_b_info = svm
        .get_account(&maker_token_acc_b)
        .expect("Maker token account B not found");
    // Deserialize token account data
    let token_acc_data = spl_token::state::Account::unpack(&maker_acc_b_info.data)
        .expect("Failed to unpack token account");
    // Get decimals for Token B
    let mint_b_account = svm
        .get_account(&mint_b_pubkey)
        .expect("failed to get mint B");
    let mint_b_data =
        spl_token::state::Mint::unpack(&mint_b_account.data).expect("failed to unpack mint b");
    let token_b_decimals = mint_b_data.decimals;
    println!(
        "Maker Token Account B:\n  Mint: {}\n  ATA:  {}\n  Amount: {} ({})\n",
        mint_b_pubkey,
        maker_token_acc_b,
        token_acc_data.amount,
        to_ui_amount(token_acc_data.amount, token_b_decimals)
    );

    // STEP 3: Create taker's Token B account (using SAME mint_b_pubkey, just different owner)
    let taker_token_acc_b = create_token_account(
        &mut svm,
        &taker,
        &mint_b_pubkey, // Use the SAME Token B mint as maker
        &taker.pubkey(),
    )?;

    // Mint tokens to taker's Token B account
    mint_tokens_to(
        &mut svm,
        &maker,
        &mint_b_pubkey,
        initial_amount,
        &taker_token_acc_b,
    )?;

    // Verify taker has tokens in account B
    let taker_acc_b_info = svm
        .get_account(&taker_token_acc_b)
        .expect("Taker token account B not found");
    // Deserialize token account data
    let token_acc_data = spl_token::state::Account::unpack(&taker_acc_b_info.data)
        .expect("Failed to unpack token account");
    println!(
        "Taker Token Account B:\n  Mint: {}\n  ATA:  {}\n  Amount: {} ({})\n",
        mint_b_pubkey,
        taker_token_acc_b,
        token_acc_data.amount,
        to_ui_amount(token_acc_data.amount, token_b_decimals)
    );

    // STEP 4: Create taker's Token A account (using SAME mint_a_pubkey, just different owner)
    let taker_token_acc_a = create_token_account(
        &mut svm,
        &taker,
        &mint_a_pubkey, // Use the SAME Token A mint as maker
        &taker.pubkey(),
    )?;

    // Verify taker has tokens in account A
    let taker_acc_a_info = svm
        .get_account(&taker_token_acc_a)
        .expect("Taker token account A not found");
    // Deserialize token account data
    let token_acc_data = spl_token::state::Account::unpack(&taker_acc_a_info.data)
        .expect("Failed to unpack token account");

    println!(
        "Taker Token Account A:\n  Mint: {}\n  ATA:  {}\n  Amount: {} ({})\n",
        mint_a_pubkey,
        taker_token_acc_a,
        token_acc_data.amount,
        to_ui_amount(token_acc_data.amount, token_a_decimals)
    );

    // Derive PDAs
    let (vault_pda, vault_bump, escrow_pda, escrow_bump) =
        derive_pdas(&maker.pubkey(), escrow_id, &program_id);

    Ok(EscrowTestSetup {
        svm,
        program_id,
        maker,
        taker,
        maker_token_acc_a,
        maker_token_acc_b,
        taker_token_acc_a,
        taker_token_acc_b,
        mint_a_pubkey,
        mint_b_pubkey,
        vault_pda,
        escrow_pda,
        vault_bump,
        escrow_bump,
        token_a_decimals,
        token_b_decimals,
    })
}
/// Send and verify transaction
pub fn send_transaction(
    svm: &mut LiteSVM,
    instruction: Instruction,
    payer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .map_err(|e| format!("Transaction failed: {:?}", e))?;

    println!("Transaction sent successfully");
    Ok(())
}
fn to_ui_amount(amount: u64, decimals: u8) -> f64 {
    amount as f64 / 10f64.powi(decimals as i32)
}

pub fn create_initialize_escrow_instruction(
    set_up: &EscrowTestSetup,
    params: &EscrowParams,
) -> Instruction {
    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&params.escrow_id.to_le_bytes());
    instruction_data.extend_from_slice(&params.deposit_amount.to_le_bytes());
    instruction_data.extend_from_slice(&params.receive_amount.to_le_bytes());

    let instruction = Instruction {
        program_id: set_up.program_id, // The program to call
        accounts: vec![
            // Accounts the program needs
            AccountMeta::new(set_up.maker.pubkey(), true),
            AccountMeta::new_readonly(set_up.mint_a_pubkey, false),
            AccountMeta::new_readonly(set_up.mint_b_pubkey, false),
            AccountMeta::new(set_up.maker_token_acc_a, false),
            AccountMeta::new(set_up.vault_pda, false),
            AccountMeta::new(set_up.escrow_pda, false),
            AccountMeta::new(set_up.maker_token_acc_b, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: instruction_data, // Instruction data
    };
    instruction
}

pub fn create_refund_escrow_instruction(
    set_up: &mut EscrowTestSetup,
    params: &EscrowParams,
) -> Instruction {
    let mut instruction_data = vec![2u8];
    instruction_data.extend_from_slice(&params.escrow_id.to_le_bytes());
    Instruction {
        program_id: set_up.program_id,
        accounts: vec![
            AccountMeta::new(set_up.maker.pubkey(), true),
            AccountMeta::new_readonly(set_up.mint_a_pubkey, false),
            AccountMeta::new(set_up.maker_token_acc_a, false),
            AccountMeta::new(set_up.escrow_pda, false),
            AccountMeta::new(set_up.vault_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    }
}

pub fn create_release_funds_instruction(
    set_up: &mut EscrowTestSetup,
    params: &EscrowParams,
) -> Instruction {
    let mut instruction_data = vec![1u8];
    instruction_data.extend_from_slice(&params.escrow_id.to_le_bytes());
    Instruction {
        program_id: set_up.program_id,
        accounts: vec![
            AccountMeta::new(set_up.taker.pubkey(), true),
            AccountMeta::new(set_up.maker.pubkey(), false),
            AccountMeta::new(set_up.mint_a_pubkey, false),
            AccountMeta::new(set_up.mint_b_pubkey, false),
            AccountMeta::new(set_up.maker_token_acc_b, false),
            AccountMeta::new(set_up.taker_token_acc_a, false),
            AccountMeta::new(set_up.taker_token_acc_b, false),
            AccountMeta::new(set_up.vault_pda, false),
            AccountMeta::new(set_up.escrow_pda, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ],
        data: instruction_data,
    }
}
