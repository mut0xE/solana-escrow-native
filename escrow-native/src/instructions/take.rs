use crate::{error::EscrowError, state::Escrow};
use solana_program::{
    account_info::next_account_info,
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::{
    instruction::{close_account, transfer},
    state::Account,
    ID as TOKEN_PROGRAM_ID,
};

/// Completes an escrow exchange by releasing funds to the respective parties.
///
/// Accounts expected:
/// 0. `[signer]`       taker_info:         The account of the person taking the escrow. Must be a signer.
/// 1. `[writable]`     maker_info:         The account of the person who initialized the escrow. Used to reclaim rent.
/// 2. `[]`             mint_a_info:        The mint account of token A (the token originally deposited by maker).
/// 3. `[]`             mint_b_info:        The mint account of token B (the token the maker wants to receive).
/// 4. `[writable]`     maker_ata_b_info:   The maker's SPL Token account holding token B, where they receive funds.
/// 5. `[writable]`     taker_ata_a_info:   The taker's SPL Token account holding token A, where they receive funds.
/// 6. `[writable]`     taker_ata_b_info:   The taker's SPL Token account holding token B, from which they pay.
/// 7. `[writable]`     vault_info:         The PDA token account where token A was deposited. This account will be closed.
/// 8. `[writable]`     escrow_info:        The PDA account storing the escrow state. This account will be closed.
/// 9. `[]`             token_program_info: The SPL Token Program account.
pub fn release_funds(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    escrow_id: u64,
) -> ProgramResult {
    // Create an iterator for the accounts array to process them in order.
    let account_iter = &mut accounts.iter();

    let taker_info = next_account_info(account_iter)?; // Taker's wallet account (signer)
    let maker_info = next_account_info(account_iter)?; // Maker's wallet account (receives rent back)
    let mint_a_info = next_account_info(account_iter)?; // Mint for Token A (deposited by maker)
    let mint_b_info = next_account_info(account_iter)?; // Mint for Token B (desired by maker, paid by taker)
    let maker_ata_b_info = next_account_info(account_iter)?; // Maker's SPL Token account for Token B
    let taker_ata_a_info = next_account_info(account_iter)?; // Taker's SPL Token account for Token A
    let taker_ata_b_info = next_account_info(account_iter)?; // Taker's SPL Token account for Token B
    let vault_info = next_account_info(account_iter)?; // Program's vault holding Token A
    let escrow_info = next_account_info(account_iter)?; // Escrow state account (PDA)
    let token_program_info = next_account_info(account_iter)?; // SPL Token program account

    // --- Validation Checks ---

    // Ensure the taker has signed the transaction as they are initiating the exchange.
    if !taker_info.is_signer {
        return Err(EscrowError::MissingRequiredSignature.into());
    }
    // Ensure all writable accounts are actually writable to prevent unauthorized modifications.
    if !maker_info.is_writable
        || !maker_ata_b_info.is_writable
        || !taker_ata_a_info.is_writable
        || !taker_ata_b_info.is_writable
        || !vault_info.is_writable
        || !escrow_info.is_writable
    {
        return Err(EscrowError::InvalidAccountData.into());
    }
    // Verify the SPL Token program ID to ensure correct interaction with the token program.
    if *token_program_info.key != TOKEN_PROGRAM_ID {
        return Err(EscrowError::IncorrectProgramId.into());
    }
    // Verify the escrow account is owned by the current program to ensure its authenticity.
    if escrow_info.owner != program_id {
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    msg!("Unpacking escrow account...");
    // Unpack the escrow account data to access its state.
    let escrow_acc = Escrow::unpack_the_slice_data(&escrow_info.data.borrow())
        .expect("unable to unpack the escrow account");
    // Verify the provided escrow_id matches the one stored in the escrow account.
    if escrow_acc.escrow_id != escrow_id {
        return Err(EscrowError::InvalidEscrowId.into());
    }
    // Verify that the mint A account provided matches the one recorded in the escrow.
    if escrow_acc.token_mint_a != *mint_a_info.key {
        return Err(EscrowError::InvalidMint.into());
    }
    // Verify that the maker account provided matches the maker recorded in the escrow.
    if escrow_acc.maker != *maker_info.key {
        return Err(EscrowError::InvalidAccountData.into());
    }
    msg!("Validating taker's Token B account...");
    // Unpack the taker's Token B account data.
    let taker_token_b_data = Account::unpack(&taker_ata_b_info.data.borrow())?;
    // Verify taker's Token B account is owned by the taker.
    if taker_token_b_data.owner != *taker_info.key {
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    // Verify taker's Token B account holds the correct mint.
    if taker_token_b_data.mint != *mint_b_info.key {
        return Err(EscrowError::InvalidMint.into());
    }
    // Get the amount of Token B the maker expects to receive.
    let receive_amount = escrow_acc.receive;
    // Verify taker has sufficient Token B balance to fulfill the exchange.
    if taker_token_b_data.amount < receive_amount {
        return Err(EscrowError::InsufficientFunds.into());
    }
    msg!("Validating maker's Token B account...");
    // Unpack the maker's Token B account data.
    let maker_token_b_data = Account::unpack(&maker_ata_b_info.data.borrow())?;

    // Verify maker's Token B account is owned by the maker.
    if maker_token_b_data.owner != *maker_info.key {
        return Err(EscrowError::InvalidAccountData.into());
    }
    // Verify maker's Token B account has the correct mint.
    if maker_token_b_data.mint != *mint_b_info.key {
        return Err(EscrowError::InvalidMint.into());
    }
    msg!("Validating taker's Token A account...");
    // Unpack the taker's Token A account data.
    let takers_token_a_data = Account::unpack(&taker_ata_a_info.data.borrow())?;
    // Verify taker's Token A account is owned by the taker.
    if takers_token_a_data.owner != *taker_info.key {
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    // Verify taker's Token A account has the correct mint.
    if takers_token_a_data.mint != *mint_a_info.key {
        return Err(EscrowError::InvalidMint.into());
    }
    // Verify the vault account is owned by the SPL Token program.
    if *vault_info.owner != TOKEN_PROGRAM_ID {
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    msg!("Validating vault...");
    // Unpack the vault account data.
    let vault_data = Account::unpack(&vault_info.data.borrow())?;
    // Verify vault has the correct mint (Token A).
    if vault_data.mint != *mint_a_info.key {
        return Err(EscrowError::InvalidMint.into());
    }

    // Derive the vault PDA to verify its ownership and generate signer seeds.
    let escrow_seed = escrow_id.to_le_bytes();
    let vault_seeds: &[&[u8]] = &[b"vault", maker_info.key.as_ref(), escrow_seed.as_ref()];
    let (vault_pda, vault_bump) = Pubkey::find_program_address(vault_seeds, &program_id);
    // Verify the vault token account's authority matches the derived vault PDA.
    if vault_data.owner != vault_pda {
        return Err(EscrowError::InvalidAccountOwner.into());
    }

    msg!("All validations passed. Executing exchange...");

    // --- Exchange Execution ---

    // STEP 1: Transfer Token B from taker to maker.
    // The taker pays `receive_amount` of Token B to the maker.
    msg!(
        "Transferring {} Token B from taker to maker...",
        receive_amount
    );
    let transfer_b_instruction = transfer(
        &TOKEN_PROGRAM_ID,     // Token program ID
        &taker_ata_b_info.key, // Source: Taker's Token B account
        &maker_ata_b_info.key, // Destination: Maker's Token B account
        &taker_info.key,       // Authority: Taker
        &[taker_info.key],     // Signers: Taker
        receive_amount,        // Amount to transfer
    )?;
    // Invoke the transfer instruction.
    invoke(
        &transfer_b_instruction,
        &[
            token_program_info.clone(), // Token program
            taker_ata_b_info.clone(),   // Taker's Token B account
            maker_ata_b_info.clone(),   // Maker's Token B account
            taker_info.clone(),         // Taker (signer)
        ],
    )?;
    msg!("Token B transferred successfully.");

    // STEP 2: Transfer Token A from vault to taker.
    // The amount of Token A in the vault is transferred to the taker.
    let deposit_amount = vault_data.amount; // Get the total amount of Token A held in the vault.
    msg!(
        "Transferring {} Token A from vault to taker...",
        deposit_amount
    );
    let transfer_a_instrcution = transfer(
        &TOKEN_PROGRAM_ID,     // Token program ID
        &vault_info.key,       // Source: Vault (holding Token A)
        &taker_ata_a_info.key, // Destination: Taker's Token A account
        &vault_pda,            // Authority: Vault PDA
        &[&vault_pda],         // Signers: Vault PDA (program signed)
        deposit_amount,        // Amount to transfer
    )?;
    // Define the signer seeds for the vault PDA.
    let vault_signer_seeds: &[&[&[u8]]] = &[&[
        b"vault",
        maker_info.key.as_ref(),
        escrow_seed.as_ref(),
        &[vault_bump], // Add the bump seed
    ]];

    // Invoke the transfer instruction with the vault PDA as signer.
    invoke_signed(
        &transfer_a_instrcution,
        &[
            vault_info.clone(),         // Vault account
            taker_ata_a_info.clone(),   // Taker's Token A account
            token_program_info.clone(), // Token program
        ],
        vault_signer_seeds,
    )?;
    msg!("Token A transferred successfully.");

    // STEP 3: Close the vault account to reclaim rent.
    // The rent collected for the vault account is returned to the maker.
    msg!("Closing vault account and reclaiming rent...");
    let close_vault_instrution = close_account(
        &TOKEN_PROGRAM_ID, // Token program ID
        &vault_info.key,   // Account to close: Vault
        &maker_info.key,   // Destination for rent: Maker's wallet
        &vault_pda,        // Authority: Vault PDA
        &[&vault_pda],     // Signers: Vault PDA (program signed)
    )?;
    // Invoke the close account instruction with the vault PDA as signer.
    invoke_signed(
        &close_vault_instrution,
        &[
            vault_info.clone(),         // Vault account to be closed
            maker_info.clone(),         // Maker's account (receives rent)
            token_program_info.clone(), // Token program
        ],
        vault_signer_seeds,
    )?;
    msg!("Vault closed.");

    // The escrow account is also implicitly closed and its rent returned to the maker
    // since it is writable and its data has been consumed.

    // --- Final Logging ---
    msg!("Escrow exchange completed successfully!");
    msg!("   - Taker received: {} Token A", deposit_amount);
    msg!("   - Maker received: {} Token B", receive_amount);
    Ok(())
}
