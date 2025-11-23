use escrow_native::state::Escrow;
use litesvm::LiteSVM;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signer::Signer};

mod common;
use common::*;
use spl_token::{
    amount_to_ui_amount,
    state::{Account, Mint},
};
/// Verify vault account
pub fn verify_vault(
    svm: &LiteSVM,
    vault_pda: &Pubkey,
    expected_amount: u64,
    expected_mint: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let vault_account = svm
        .get_account(vault_pda)
        .ok_or("Vault account not found")?;

    let vault_data = Account::unpack(&vault_account.data)?;

    assert_eq!(vault_data.amount, expected_amount, "Vault amount mismatch");
    assert_eq!(vault_data.mint, *expected_mint, "Vault mint mismatch");

    println!("Vault verified:");
    println!("  Amount: {}", vault_data.amount);
    println!("  Mint: {}", vault_data.mint);

    Ok(())
}
/// Verify escrow state
pub fn verify_escrow(
    svm: &LiteSVM,
    escrow_pda: &Pubkey,
    expected_maker: &Pubkey,
    expected_mint_a: &Pubkey,
    expected_mint_b: &Pubkey,
    expected_escrow_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let escrow_account = svm
        .get_account(escrow_pda)
        .ok_or("Escrow account not found")?;

    let escrow_data = Escrow::unpack_the_slice_data(&escrow_account.data)?;

    assert_eq!(
        escrow_data.escrow_id, expected_escrow_id,
        "Escrow ID mismatch"
    );
    assert_eq!(escrow_data.maker, *expected_maker, "Maker mismatch");
    assert_eq!(
        escrow_data.token_mint_a, *expected_mint_a,
        "Mint A mismatch"
    );
    assert_eq!(
        escrow_data.token_mint_b, *expected_mint_b,
        "Mint B mismatch"
    );

    println!(" Escrow verified:");
    println!("  ID: {}", escrow_data.escrow_id);
    println!("  Maker: {}", escrow_data.maker);
    println!("  Mint A: {}", escrow_data.token_mint_a);
    println!("  Mint B: {}", escrow_data.token_mint_b);

    Ok(())
}
// TESTS

#[test]
fn test_initialize_escrow_success() {
    let params = EscrowParams {
        escrow_id: 10,
        deposit_amount: 1_000_000_00,
        receive_amount: 200_000_000,
    };

    let mut setup = setup_escrow_test(params.escrow_id).expect("Setup failed");

    println!("\n Escrow Parameters:");
    println!("  ID: {}", params.escrow_id);
    println!("  Deposit: {} tokens", params.deposit_amount);
    println!("  Receive: {} tokens", params.receive_amount);

    // Create and send transaction
    let instruction = create_initialize_escrow_instruction(&setup, &params);
    send_transaction(&mut setup.svm, instruction, &setup.maker).expect("Transaction failed");

    // Verify results
    println!("\n✓ Verifying results...");
    verify_vault(
        &setup.svm,
        &setup.vault_pda,
        params.deposit_amount,
        &setup.mint_a_pubkey,
    )
    .expect("Vault verification failed");

    verify_escrow(
        &setup.svm,
        &setup.escrow_pda,
        &setup.maker.pubkey(),
        &setup.mint_a_pubkey,
        &setup.mint_b_pubkey,
        params.escrow_id,
    )
    .expect("Escrow verification failed");

    println!("\n Test passed!");
}
#[test]
fn test_refund_escrow_success() {
    println!("\n========== TEST: Refund Escrow ==========\n");
    let params = EscrowParams {
        escrow_id: 10,
        deposit_amount: 900_000_000,
        receive_amount: 100_000_000,
    };
    let mut set_up = setup_escrow_test(params.escrow_id).expect("Setup failed");
    // Step 1: Initialize escrow
    println!("\nSTEP 1: Initialize Escrow");
    let initialize_escrow_instruction = create_initialize_escrow_instruction(&mut set_up, &params);
    send_transaction(
        &mut set_up.svm,
        initialize_escrow_instruction,
        &set_up.maker,
    )
    .expect("Initialize failed");
    // Verify initialized state
    verify_vault(
        &set_up.svm,
        &set_up.vault_pda,
        params.deposit_amount,
        &set_up.mint_a_pubkey,
    )
    .expect("Vault verification failed after init");
    let mint_a_acc = set_up
        .svm
        .get_account(&set_up.mint_a_pubkey)
        .expect("failed to read token mint A");

    let mint_a_data = Mint::unpack(&mint_a_acc.data).expect("failed to unpack mint a");
    let token_mint_a_decimals = mint_a_data.decimals;
    // Get maker balance before refund
    let maker_account_before = set_up
        .svm
        .get_account(&set_up.maker_token_acc_a)
        .expect("failed to get maker token A account");
    let maker_token_balance = Account::unpack(&maker_account_before.data)
        .expect("failed to unpack the account before data");
    let balance_before = amount_to_ui_amount(maker_token_balance.amount, token_mint_a_decimals);
    println!("\nMaker Token A balance BEFORE refund: {}", balance_before);
    // Get vault balance before refund
    let account_before = set_up
        .svm
        .get_account(&set_up.vault_pda)
        .expect("failed to get vault account");
    let balance_before_vault =
        Account::unpack(&account_before.data).expect("failed to unpack the account before data");
    let balance_before_vault =
        amount_to_ui_amount(balance_before_vault.amount, token_mint_a_decimals);

    println!(
        "\nVault Token A balance BEFORE refund: {}",
        balance_before_vault
    );
    // Step 2: Refund escrow
    println!("\nSTEP 2: Refund Escrow");
    let refund_instruction = create_refund_escrow_instruction(&mut set_up, &params);
    send_transaction(&mut set_up.svm, refund_instruction, &set_up.maker).expect("Refund failed");
    // Step 3: Verify refund
    println!("\nSTEP 3: Verify Refund");
    // Verify tokens returned to maker
    let maker_account_after = set_up
        .svm
        .get_account(&set_up.maker_token_acc_a)
        .expect("failed to get maker token A account");
    let maker_token_balance = Account::unpack(&maker_account_after.data)
        .expect("failed to unpack the account after data");
    let balance_after = amount_to_ui_amount(maker_token_balance.amount, token_mint_a_decimals);

    println!("\nMaker Token A balance AFTER refund: {}", balance_after);
    assert_eq!(
        balance_after,
        balance_before + amount_to_ui_amount(params.deposit_amount, token_mint_a_decimals),
        "Tokens not refunded correctly"
    );
    println!("Tokens refunded correctly!");
    // Verify vault is empty or closed
    if let Some(vault_account) = set_up.svm.get_account(&set_up.vault_pda) {
        // Account exists but data should be empty
        assert!(
            vault_account.data.is_empty(),
            "Vault account exists but is not empty"
        );
        println!("Vault account exists but is empty (closed).");
    } else {
        println!("Vault account fully closed (removed).");
    }

    // Verify escrow is closed or empty
    if let Some(escrow_account) = set_up.svm.get_account(&set_up.escrow_pda) {
        // Check if data is zeroed
        let all_zeros = escrow_account.data.iter().all(|&b| b == 0);
        assert!(all_zeros, "Escrow data should be zeroed");
        println!("Escrow data is zeroed!");
    } else {
        println!("Escrow account closed!");
    }

    println!("\nRefund Escrow Test PASSED!\n");
}
// TEST CASE FOR RELEASE_FUNDS (TAKER EXCHANGE)
#[test]
fn test_release_funds_success() {
    println!("\n========== TEST: Release Funds Success ==========\n");
    let params = EscrowParams {
        escrow_id: 10,
        deposit_amount: 200_000_000,
        receive_amount: 100_000_000,
    };

    let mut set_up = setup_escrow_test(params.escrow_id).expect("failed to set escrow setup");
    // STEP 1: Initialize escrow (maker deposits Token A)
    let initialize_escrow_instruction = create_initialize_escrow_instruction(&set_up, &params);
    send_transaction(
        &mut set_up.svm,
        initialize_escrow_instruction,
        &set_up.maker,
    )
    .expect("Initialize failed");

    verify_vault(
        &set_up.svm,
        &set_up.vault_pda,
        params.deposit_amount,
        &set_up.mint_a_pubkey,
    )
    .expect("Vault verification failed");
    let token_a_decimals = set_up.token_a_decimals;
    let token_b_decimals = set_up.token_b_decimals;
    // STEP 2: Get BEFORE balances
    println!("\nSTEP 2: Record BEFORE Balances");
    let maker_b_before = set_up
        .svm
        .get_account(&set_up.maker_token_acc_b)
        .expect("failed to get maker b account before");
    let maker_b_data_before = Account::unpack(&maker_b_before.data)
        .expect("unable to unpack maker b account data before");
    let maker_b_balance_before = maker_b_data_before.amount;
    println!(
        "Maker Token B balance BEFORE: {} ({})",
        maker_b_balance_before,
        amount_to_ui_amount(maker_b_balance_before, token_b_decimals)
    );
    let taker_a_before = set_up
        .svm
        .get_account(&set_up.taker_token_acc_a)
        .expect("failed to get taker a account before");
    let taker_a_data_before = Account::unpack(&taker_a_before.data)
        .expect("unable to unpack taker a account data before");
    let taker_a_balance_before = taker_a_data_before.amount;
    println!(
        "Taker Token A balance BEFORE: {} ({})",
        taker_a_balance_before,
        amount_to_ui_amount(taker_a_balance_before, token_a_decimals)
    );
    let taker_b_before = set_up
        .svm
        .get_account(&set_up.taker_token_acc_b)
        .expect("failed to get taker b account before");
    let taker_b_data_before = Account::unpack(&taker_b_before.data)
        .expect("unable to unpack taker b account data before");
    let taker_b_balance_before = taker_b_data_before.amount;
    println!(
        "Taker Token B balance BEFORE: {} ({})",
        taker_b_balance_before,
        amount_to_ui_amount(taker_b_balance_before, token_b_decimals)
    );
    // STEP 3: Execute release_funds (taker sends Token B, receives Token A)
    println!("\nSTEP 3: Execute Release Funds");
    let release_instruction = create_release_funds_instruction(&mut set_up, &params);
    send_transaction(&mut set_up.svm, release_instruction, &set_up.taker)
        .expect("failed to send tx");

    // STEP 4: Get AFTER balances
    println!("\nSTEP 4: Record AFTER Balances");
    let taker_a_after = set_up
        .svm
        .get_account(&set_up.taker_token_acc_a)
        .expect("failed to get taker a account after");
    let taker_a_data_after =
        Account::unpack(&taker_a_after.data).expect("unable to unpack taker a account data after");
    let taker_a_balance_after = taker_a_data_after.amount;
    println!(
        "Taker Token A balance AFTER: {} ({})",
        taker_a_balance_after,
        amount_to_ui_amount(taker_a_balance_after, token_a_decimals)
    );
    let maker_b_after = set_up
        .svm
        .get_account(&set_up.maker_token_acc_b)
        .expect("failed to get maker b account after");
    let maker_b_data_after =
        Account::unpack(&maker_b_after.data).expect("unable to unpack maker b account data after");
    let maker_b_balance_after = maker_b_data_after.amount;
    println!(
        "Maker Token B balance AFTER: {} ({})",
        maker_b_balance_after,
        amount_to_ui_amount(maker_b_balance_after, token_b_decimals)
    );
    let taker_b_after = set_up
        .svm
        .get_account(&set_up.taker_token_acc_b)
        .expect("failed to get taker b account after");
    let taker_b_data_after =
        Account::unpack(&taker_b_after.data).expect("unable to unpack taker b account data after");
    let taker_b_balance_after = taker_b_data_after.amount;
    println!(
        "Taker Token B balance AFTER: {} ({})",
        taker_b_balance_after,
        amount_to_ui_amount(taker_b_balance_after, token_b_decimals)
    );

    // STEP 5: Verify Exchange Results
    println!("\nSTEP 5: Verify Exchange Results");
    // Taker should have RECEIVED deposit_amount of Token A
    let taker_a_received = taker_a_balance_after - taker_a_balance_before;
    assert_eq!(
        taker_a_received, params.deposit_amount,
        "Taker should have received {} Token A, but received {}",
        params.deposit_amount, taker_a_received
    );
    println!(
        "Taker received {} Token A ({})",
        taker_a_received,
        amount_to_ui_amount(taker_a_received, token_a_decimals)
    );
    // Maker should have RECEIVED receive_amount of Token B
    let maker_b_received = maker_b_balance_after - maker_b_balance_before;
    assert_eq!(
        maker_b_received, params.receive_amount,
        "Maker should have received {} Token B, but received {}",
        params.receive_amount, maker_b_received
    );
    println!(
        "Maker received {} Token B ({})",
        maker_b_received,
        amount_to_ui_amount(maker_b_received, token_b_decimals)
    );

    // Taker should have SENT receive_amount of Token B
    let taker_b_sent = taker_b_balance_before - taker_b_balance_after;
    assert_eq!(
        taker_b_sent, params.receive_amount,
        "Taker should have sent {} Token B, but sent {}",
        params.receive_amount, taker_b_sent
    );
    println!(
        "Taker sent {} Token B ({})",
        taker_b_sent,
        amount_to_ui_amount(taker_b_sent, token_b_decimals)
    ); // Verify vault is closed
    if let Some(vault_account) = set_up.svm.get_account(&set_up.vault_pda) {
        assert!(vault_account.data.is_empty(), "Vault should be closed");
        println!("Vault closed successfully");
    }
    println!("\nRelease Funds Test PASSED!\n");
}
