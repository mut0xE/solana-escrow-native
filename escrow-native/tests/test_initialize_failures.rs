mod common;
use common::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    signature::Keypair,
    signer::Signer,
    system_program, sysvar,
};
use spl_token::{state::Account, ID as TOKEN_PROGRAM_ID};
// TEST 1: Insufficient Funds
/// Test when maker doesn't have enough Token A to deposit
#[test]
fn test_initialize_escrow_insufficient_funds() {
    println!("\n========== TEST: Insufficient Funds ==========\n");
    let escrow_params = EscrowParams {
        escrow_id: 1,
        deposit_amount: 2_000_000_000_000,
        receive_amount: 1_000_000,
    };
    let mut set_up = setup_escrow_test(escrow_params.escrow_id).expect("Setup failed");
    let maker_token_acc_a = set_up
        .svm
        .get_account(&set_up.maker_token_acc_a)
        .expect("Maker token account A not found");
    let maker_data =
        Account::unpack(&maker_token_acc_a.data).expect("Failed to unpack maker token account");

    println!("\nMaker has: {} tokens", maker_data.amount);

    println!("Trying to deposit: {} tokens", escrow_params.deposit_amount);
    let instruction = create_initialize_escrow_instruction(&set_up, &escrow_params);
    let result = send_transaction(&mut set_up.svm, instruction, &set_up.maker);
    println!("");
    assert!(
        result.is_err(),
        "Expected insufficient funds error, but transaction succeeded"
    );
}
// TEST 2: Invalid Mint
// Test when wrong mint is provided for Token A
#[test]
fn test_initialize_escrow_wrong_mint_a() {
    println!("\n========== TEST: Wrong Mint A ==========\n");

    let params = EscrowParams {
        escrow_id: 102,
        deposit_amount: 100_000_000,
        receive_amount: 200_000_000,
    };

    let wrong_mint_a = Keypair::new();
    let mut setup = setup_escrow_test(params.escrow_id).expect("failed to setup escrow");
    create_token_mint(&mut setup.svm, &wrong_mint_a, 9, &setup.maker)
        .expect("Failed to create wrong mint");

    println!("Using wrong mint: {}", wrong_mint_a.pubkey());
    println!("Expected mint: {}", setup.mint_a_pubkey);

    // Build instruction with wrong mint
    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&params.escrow_id.to_le_bytes());
    instruction_data.extend_from_slice(&params.deposit_amount.to_le_bytes());
    instruction_data.extend_from_slice(&params.receive_amount.to_le_bytes());

    let wrong_instruction = Instruction {
        program_id: setup.program_id,
        accounts: vec![
            AccountMeta::new(setup.maker.pubkey(), true),
            AccountMeta::new_readonly(wrong_mint_a.pubkey(), false), // WRONG MINT!
            AccountMeta::new_readonly(setup.mint_b_pubkey, false),
            AccountMeta::new(setup.maker_token_acc_a, false),
            AccountMeta::new(setup.vault_pda, false),
            AccountMeta::new(setup.escrow_pda, false),
            AccountMeta::new(setup.maker_token_acc_b, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: instruction_data,
    };

    let result = send_transaction(&mut setup.svm, wrong_instruction, &setup.maker);
    assert!(
        result.is_err(),
        "Transaction should fail when wrong mint is provided"
    );
}
// TEST 3: Zero Deposit Amount
// Test when deposit amount is zero
#[test]
fn test_initialize_escrow_zero_deposit() {
    let params = EscrowParams {
        escrow_id: 103,
        deposit_amount: 0, // Invalid: zero deposit
        receive_amount: 200_000_000,
    };
    let mut set_up = setup_escrow_test(params.escrow_id).expect("failed to setup escrow");
    let init_instruction = create_initialize_escrow_instruction(&mut set_up, &params);
    let result = send_transaction(&mut set_up.svm, init_instruction, &set_up.maker);
    assert!(
        result.is_err(),
        "Transaction should fail when deposit amount is zero"
    )
}
// TEST 4: Zero Receive Amount
// Test when receive amount is zero
#[test]
fn test_initialize_escrow_zero_receive() {
    let params = EscrowParams {
        escrow_id: 103,
        deposit_amount: 100_000_000, // Invalid: zero deposit
        receive_amount: 0,
    };
    let mut set_up = setup_escrow_test(params.escrow_id).expect("failed to setup escrow");
    let init_instruction = create_initialize_escrow_instruction(&mut set_up, &params);
    let result = send_transaction(&mut set_up.svm, init_instruction, &set_up.maker);
    assert!(
        result.is_err(),
        "Transaction should fail when receive amount is zero"
    )
}
// TEST 5: Invalid Token Account Owner
// Test when token account is not owned by maker

// TEST 6: Not Signed by Maker
// Test when transaction is not signed by maker
#[test]
fn test_initialize_escrow_not_signed_by_maker() {
    let params = EscrowParams {
        escrow_id: 106,
        deposit_amount: 100_000_000,
        receive_amount: 200_000_000,
    };

    let mut setup = setup_escrow_test(params.escrow_id).expect("Setup should succeed");

    // Create a different signer (not the maker)
    let wrong_signer = Keypair::new();
    setup
        .svm
        .airdrop(&wrong_signer.pubkey(), 5_000_000_000)
        .expect("Airdrop should succeed");

    let mut instruction_data = vec![0u8];
    instruction_data.extend_from_slice(&params.escrow_id.to_le_bytes());
    instruction_data.extend_from_slice(&params.deposit_amount.to_le_bytes());
    instruction_data.extend_from_slice(&params.receive_amount.to_le_bytes());

    let instruction = Instruction {
        program_id: setup.program_id,
        accounts: vec![
            // maker — NOT a signer here
            AccountMeta::new(setup.maker.pubkey(), false),
            AccountMeta::new_readonly(setup.mint_a_pubkey, false),
            AccountMeta::new_readonly(setup.mint_b_pubkey, false),
            AccountMeta::new(setup.maker_token_acc_a, false),
            AccountMeta::new(setup.vault_pda, false),
            AccountMeta::new(setup.escrow_pda, false),
            AccountMeta::new(setup.maker_token_acc_b, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: instruction_data,
    };
    let result = send_transaction(&mut setup.svm, instruction, &wrong_signer);

    // Assert transaction failed
    assert!(
        result.is_err(),
        "Transaction should fail when not signed by maker"
    );
}

// TEST 7: Mismatched Token Account Mint
// Test when maker's token account mint doesn't match expected mint

// TEST 8: Duplicate Escrow ID
/// Test when trying to create escrow with ID that already exists

#[test]
fn test_initialize_escrow_duplicate_id() {
    let params = EscrowParams {
        escrow_id: 108,
        deposit_amount: 100_000_000,
        receive_amount: 200_000_000,
    };

    let mut setup = setup_escrow_test(params.escrow_id).expect("Setup should succeed");

    // First escrow creation (should succeed)
    let instruction1 = create_initialize_escrow_instruction(&setup, &params);
    let result1 = send_transaction(&mut setup.svm, instruction1, &setup.maker);
    assert!(result1.is_ok(), "First escrow creation should succeed");

    // Try to create escrow with same ID (should fail)
    let instruction2 = create_initialize_escrow_instruction(&setup, &params);
    let result2 = send_transaction(&mut setup.svm, instruction2, &setup.maker);

    assert!(
        result2.is_err(),
        "Transaction should fail when escrow ID already exists"
    );
}
