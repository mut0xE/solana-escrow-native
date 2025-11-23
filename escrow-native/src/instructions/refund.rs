use crate::{error::EscrowError, state::Escrow};
use solana_program::{
    account_info::next_account_info, account_info::AccountInfo, entrypoint::ProgramResult, msg,
    program::invoke_signed, program_pack::Pack, pubkey::Pubkey,
    system_program::ID as SYSTEM_PROGRAM,
};
use spl_token::{
    instruction::{close_account, transfer},
    state::Account,
    ID as TOKEN_PROGRAM,
};

/// Cancels an existing escrow, refunding the tokens to the maker and closing the accounts.
///
/// This instruction is callable only by the original maker of the escrow.
/// It performs several validation checks.
///
/// Accounts expected:
///
/// 1. `[signer]` `maker_info`: The account of the person who initiated the escrow (maker).
/// 2. `[]` `mint_a_info`: The mint account of Token A.
/// 3. `[writable]` `maker_token_acc_a_info`: The maker's token account for Token A, where funds will be refunded.
/// 4. `[writable]` `escrow_info`: The escrow state account, which will be closed.
/// 5. `[writable]` `vault_info`: The token vault account holding Token A, which will be closed.
/// 6. `[]` `system_program_info`: The Solana System Program account.
/// 7. `[]` `token_program_info`: The SPL Token Program account.
///
/// Parameters:
/// - `program_id`: The public key of the current program.
/// - `accounts`: A slice of `AccountInfo` objects for all required accounts.
/// - `escrow_id`: A unique identifier for this specific escrow.
pub fn cancel_escrow(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    escrow_id: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    // Get references to the individual accounts from the iterator.
    let maker_info = next_account_info(account_iter)?; // Maker's account
    let mint_a_info = next_account_info(account_iter)?; // Mint account for Token A
    let maker_token_acc_a_info = next_account_info(account_iter)?; // Maker's Token A account
    let escrow_info = next_account_info(account_iter)?; // Escrow state account
    let vault_info = next_account_info(account_iter)?; // Token vault account
    let system_program_info = next_account_info(account_iter)?; // System Program
    let token_program_info = next_account_info(account_iter)?; // SPL Token Program

    // --- Validation Checks ---
    msg!("Starting escrow cancellation validation...");

    // 1. Ensure the maker has signed the transaction.
    if !maker_info.is_signer {
        return Err(EscrowError::InvalidAccountData.into());
    }
    // 2. Ensure all writable accounts are actually writable.
    if !maker_token_acc_a_info.is_writable || !escrow_info.is_writable || !vault_info.is_writable {
        return Err(EscrowError::InvalidAccountData.into());
    }

    // 3. Verify the SPL Token Program ID.
    // Ensures that the correct token program is being used.
    if *token_program_info.key != TOKEN_PROGRAM {
        return Err(EscrowError::IncorrectProgramId.into());
    }
    // 4. Verify the escrow account is owned by this program.
    // Essential for ensuring program control over its state.
    if *escrow_info.owner != *program_id {
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    // Verify the System Program ID.
    if *system_program_info.key != SYSTEM_PROGRAM {
        return Err(EscrowError::IncorrectProgramId.into());
    }

    msg!("Unpacking escrow account...");
    // Unpack and validate the escrow state from its data.
    let escrow_account = Escrow::unpack_the_slice_data(&escrow_info.data.borrow())?;

    // 5. Verify the provided escrow ID matches the one stored in the account.
    if escrow_account.escrow_id != escrow_id {
        return Err(EscrowError::InvalidEscrowId.into());
    }
    // 6. Verify that only the original maker can refund the escrow.
    // Prevents unauthorized refunds by others.
    if escrow_account.maker != *maker_info.key {
        return Err(EscrowError::InvalidAccountOwner.into());
    }
    // 7. Verify the mint account associated with Token A in the escrow matches the provided maker's token account.
    if escrow_account.token_mint_a != *mint_a_info.key {
        return Err(EscrowError::InvalidMint.into());
    }
    msg!("Validating maker's Token A account...");
    // Unpack the maker's Token A account data to access its properties.
    let maker_token_a_data = Account::unpack(&maker_token_acc_a_info.data.borrow())?;

    // 8. Verify maker's Token A account is owned by the maker.
    // if maker_token_acc_a_info.owner != maker_info.key {
    //     return Err(EscrowError::InvalidAccountOwner.into());
    // }
    // 9. Verify maker's Token A account has the correct mint.
    if maker_token_a_data.mint != *mint_a_info.key {
        return Err(EscrowError::InvalidMint.into());
    }
    // 10. Verify the vault account is owned by the token program.
    if *vault_info.owner != TOKEN_PROGRAM {
        return Err(EscrowError::IncorrectProgramId.into());
    }
    msg!("Validating vault...");
    // Unpack the vault account data.
    let vault_data = Account::unpack(&vault_info.data.borrow())?;

    // 11. Verify vault has the correct mint (Token A).
    if vault_data.mint != *mint_a_info.key {
        return Err(EscrowError::InvalidMint.into());
    }

    // 12. Verify the vault account and escrow account are derived correctly.
    // Recalculate the PDA for the vault and escrow based on the seeds.
    let escrow_seed = escrow_id.to_le_bytes();
    let seeds = &[b"vault", maker_info.key.as_ref(), escrow_seed.as_ref()];
    let (vault_pda, vault_bump) = Pubkey::find_program_address(seeds, program_id);
    let seeds = &[
        b"escrow_vault",
        maker_info.key.as_ref(),
        escrow_seed.as_ref(),
    ];
    let (escrow_pda, _escrow_bump) = Pubkey::find_program_address(seeds, program_id);

    // Ensure the provided escrow_info key matches the derived PDA.
    if *escrow_info.key != escrow_pda {
        return Err(EscrowError::PDADerivationMismatch.into());
    }
    // Ensure the provided vault_info key matches the derived PDA.
    if *vault_info.key != vault_pda {
        return Err(EscrowError::PDADerivationMismatch.into());
    }

    // Check if the vault actually contains tokens.
    if vault_data.amount == 0 {
        msg!("Error: Vault is empty! No tokens to refund.");
        return Err(EscrowError::InsufficientFunds.into());
    }
    msg!("All validations passed. Executing refund...");

    // --- Refund Execution ---
    // Get the amount of tokens to refund from the vault.
    let refund_amount = vault_data.amount;
    msg!("Refunding {} Token A to maker...", refund_amount);

    // STEP 1: Transfer the tokens from the vault back to the maker's Token A account.
    let refund_instruction = transfer(
        &TOKEN_PROGRAM,              // The token program ID
        &vault_info.key,             // SOURCE account (the vault)
        &maker_token_acc_a_info.key, // DESTINATION account (maker's Token A account)
        &vault_pda,                  // AUTHORITY (the vault PDA, which owns the vault account)
        &[&vault_pda],               // SIGNERS (the vault PDA needs to sign this)
        refund_amount,               // The amount of tokens to transfer
    )?;

    // Define the signer seeds for the vault PDA. These seeds are used to sign the transaction.
    let vault_signer_seeds: &[&[&[u8]]] = &[&[
        b"vault",
        maker_info.key.as_ref(),
        escrow_seed.as_ref(),
        &[vault_bump],
    ]];

    // Invoke the transfer instruction, signed by the vault PDA.
    invoke_signed(
        &refund_instruction,
        &[
            token_program_info.clone(),     // The token program
            maker_token_acc_a_info.clone(), // Maker's destination account
            vault_info.clone(),             // The vault account (source)
        ],
        vault_signer_seeds, // PDA signer seeds
    )?;
    msg!("Refund transferred successfully.");

    // STEP 2: Close the vault token account to reclaim its rent.
    // The remaining lamports in the vault will be sent to the maker.
    msg!("Closing vault account and reclaiming rent...");
    let close_acc_instruction = close_account(
        &TOKEN_PROGRAM,  // The token program ID
        &vault_info.key, // The account to close (vault)
        &maker_info.key, // The recipient of the rent lamports (maker)
        &vault_pda,      // The authority that can close the account (vault PDA)
        &[&vault_pda],   // SIGNERS (vault PDA needs to sign)
    )?;
    // Invoke the close account instruction, signed by the vault PDA.
    invoke_signed(
        &close_acc_instruction,
        &[
            token_program_info.clone(), // The token program
            vault_info.clone(),         // The vault account to close
            maker_info.clone(),         // The maker (recipient of rent)
        ],
        vault_signer_seeds, // PDA signer seeds
    )?;
    msg!("Vault closed.");

    // STEP 3: Close the escrow state account to reclaim rent.
    // This is done by transferring all lamports from the escrow account to the maker,
    // then marking the account as closed by zeroing its data.
    msg!("Closing escrow state account and reclaiming rent...");

    // Transfer all lamports from the escrow account back to the maker.
    let mut maker_lamports = maker_info.lamports.borrow_mut();
    let escrow_lamports = escrow_info.lamports();
    **maker_lamports = maker_lamports
        .checked_add(escrow_lamports) // Add escrow's lamports to maker's
        .ok_or(EscrowError::ArithmeticOverflow)?; // Handle potential overflow

    // Set the escrow account's lamports to 0, effectively closing it and making it rent-exempt.
    **escrow_info.lamports.borrow_mut() = 0;
    // Zero out the data of the escrow account to clear its state.
    escrow_info.data.borrow_mut().fill(0);

    msg!("Escrow account closed.");

    Ok(()) // Return success.
}
