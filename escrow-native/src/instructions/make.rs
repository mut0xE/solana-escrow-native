use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    sysvar::Sysvar,
};
use spl_token::{
    instruction::{initialize_account2, transfer},
    state::Account, // The public key of the SPL Token program
    ID as TOKEN_PROGRAM_ID,
};

use crate::{error::EscrowError, state::Escrow}; // Custom error and escrow state structure

/// Initializes a new escrow transaction.
///
/// This function sets up a new escrow by:
/// 1. Validating all provided accounts and input parameters.
/// 2. Deriving Program Derived Addresses (PDAs) for the deposit vault and the escrow state.
/// 3. Creating and initializing a new SPL Token account (the "vault") controlled by a PDA.
/// 4. Transferring the maker's (depositor's) tokens into this vault.
/// 5. Creating and populating the escrow state account with transaction details.
///
/// Accounts expected:
/// 0. `[signer]`       maker_info:        The account of the person initializing the escrow. Must be a signer.
/// 1. `[]`             mint_a_info:       The mint account of token A (the token being deposited).
/// 2. `[]`             mint_b_info:       The mint account of token B (the token the maker wants to receive).
/// 3. `[writable]`     maker_token_acc_a_info: The maker's SPL Token account holding token A.
/// 4. `[writable]`     deposit_vault_info: The PDA token account where token A will be deposited.
/// 5. `[writable]`     escrow_info:       The PDA account to store the escrow state.
/// 6. `[]`             token_to_receive_account: The maker's SPL Token account for token B, where they expect to receive tokens.
/// 7. `[]`             system_program_info: The Solana System Program account.
/// 8. `[]`             token_program_info: The SPL Token Program account.
/// 9. `[]`             rent_sysvar_info:  The Rent Sysvar account.
pub fn initialize_escrow(
    program_id: &Pubkey,      // The public key of this escrow program.
    accounts: &[AccountInfo], // All accounts required for the transaction.
    escrow_id: u64,           // A unique identifier for this specific escrow.
    deposit_amount: u64,      // The amount of token A the maker is depositing.
    receive_amount: u64,      // The amount of token B the maker expects to receive.
) -> ProgramResult {
    // Iterate through the accounts to parse them.
    let account_iter = &mut accounts.iter();

    // 0. The account of the maker (depositor) who initiates the escrow.
    let maker_info = next_account_info(account_iter)?;
    // 1. The mint account for Token A, which the maker is depositing.
    let mint_a_info = next_account_info(account_iter)?;
    // 2. The mint account for Token B, which the maker expects to receive.
    let mint_b_info = next_account_info(account_iter)?;
    // 3. The maker's token account holding Token A. Tokens will be transferred from here.
    let maker_token_acc_a_info = next_account_info(account_iter)?;
    // 4. The PDA-controlled token account that will hold the deposited Token A.
    let deposit_vault_info = next_account_info(account_iter)?;
    // 5. The PDA-controlled account that will store the state of this escrow.
    let escrow_info = next_account_info(account_iter)?;
    // 6. The maker's token account for Token B, where they will eventually receive tokens.
    let token_to_receive_account = next_account_info(account_iter)?;
    // 7. The Solana System Program, required for creating new accounts.
    let system_program_info = next_account_info(account_iter)?;
    // 8. The SPL Token Program, required for token operations.
    let token_program_info = next_account_info(account_iter)?;
    // 9. The Rent Sysvar, required for calculating rent exemption.
    let rent_sysvar_info = next_account_info(account_iter)?;

    // --- Validation Checks ---

    // 1. Ensure the maker has signed the transaction.
    if !maker_info.is_signer {
        msg!("Error: Maker must be a signer.");
        return Err(EscrowError::MissingRequiredSignature.into());
    }
    // 2. Ensure critical accounts are writable to allow for state changes and token transfers.
    if !maker_token_acc_a_info.is_writable
        || !deposit_vault_info.is_writable
        || !escrow_info.is_writable
    {
        msg!("Error: Maker's Token A account, Deposit Vault, and Escrow State account must be writable.");
        return Err(EscrowError::InvalidAccountData.into());
    }
    // 3. Verify that the provided token program account matches the known SPL Token Program ID.
    if token_program_info.key != &TOKEN_PROGRAM_ID {
        msg!(
            "Error: Token program ID mismatch. Expected {}, got {}.",
            TOKEN_PROGRAM_ID,
            token_program_info.key
        );
        return Err(EscrowError::IncorrectProgramId.into());
    }
    // 4. Verify that the maker's token_to_receive_account (Token B account) is owned by the SPL Token Program.
    if *token_to_receive_account.owner != TOKEN_PROGRAM_ID {
        msg!("Error: Maker's Token B account must be owned by the SPL Token Program.");
        return Err(EscrowError::IncorrectProgramId.into());
    }
    // 5. Ensure that the deposit and receive amounts are positive.
    if deposit_amount == 0 || receive_amount == 0 {
        msg!("ERROR: Amounts must be greater than 0.");
        return Err(EscrowError::InvalidAmount.into());
    }

    msg!("Validating maker's Token A account...");
    // Unpack the data of the maker's Token A account to access its state.
    let maker_token_a_data = Account::unpack(&maker_token_acc_a_info.data.borrow())?;

    // Check: Is this Token A account actually owned by the maker?
    if maker_token_a_data.owner != *maker_info.key {
        msg!("Error: Maker's Token A account is not owned by the maker.");
        return Err(EscrowError::InvalidAccountOwner.into());
    }

    // Check: Is this Token A account for the correct mint (mint_a)?
    if maker_token_a_data.mint != *mint_a_info.key {
        msg!("Error: Maker's Token A account mint does not match expected Token A mint.");
        return Err(EscrowError::InvalidMint.into());
    }

    // Check: Does the maker have sufficient Token A balance for the deposit?
    if maker_token_a_data.amount < deposit_amount {
        msg!(
            "Error: Insufficient funds. Maker has {} Token A, needs {}.",
            maker_token_a_data.amount,
            deposit_amount
        );
        return Err(EscrowError::InsufficientFunds.into());
    }

    msg!("Maker's Token A account validated.");

    // Validate token_to_receive_account (Maker's Token B account)
    msg!("Validating maker's Token B account...");
    // Unpack the data of the maker's Token B account.
    let token_to_receive_data = Account::unpack(&token_to_receive_account.data.borrow())?;

    // Check: Is this Token B account actually owned by the maker?
    if token_to_receive_data.owner != *maker_info.key {
        msg!("Error: Maker's Token B account is not owned by the maker.");
        return Err(EscrowError::InvalidAccountOwner.into());
    }

    // Check: Is this Token B account for the correct mint (mint_b)?
    if token_to_receive_data.mint != *mint_b_info.key {
        msg!("Error: Maker's Token B account mint does not match expected Token B mint.");
        return Err(EscrowError::InvalidMint.into());
    }

    msg!("Maker's Token B account validated.");

    // Convert the escrow ID into bytes for use as a seed.
    let escrow_seed = escrow_id.to_le_bytes();

    // --- PDA Derivation and Verification ---

    // STEP 1: Derive the Program Derived Address (PDA) for the deposit vault account.
    // This PDA will be the authority over the vault.
    let vault_seeds: &[&[u8]] = &[b"vault", maker_info.key.as_ref(), escrow_seed.as_ref()];
    let (vault_pda, vault_bump) = Pubkey::find_program_address(vault_seeds, program_id);

    // Verify that the provided deposit_vault_info key matches the derived PDA.
    if vault_pda != *deposit_vault_info.key {
        msg!("Error: Deposit vault PDA derivation mismatch.");
        return Err(EscrowError::PDADerivationMismatch.into());
    }

    // STEP 2: Derive the PDA for the escrow state account.
    // This PDA will hold the structured data of the escrow.
    let escrow_seeds: &[&[u8]] = &[
        b"escrow_vault", // Different seed prefix to distinguish from the token vault
        maker_info.key.as_ref(),
        escrow_seed.as_ref(),
    ];
    let (escrow_pda, escrow_bump) = Pubkey::find_program_address(escrow_seeds, program_id);

    // Verify that the provided escrow_info key matches the derived PDA.
    if escrow_pda != *escrow_info.key {
        msg!("Error: Escrow state PDA derivation mismatch.");
        return Err(EscrowError::PDADerivationMismatch.into());
    }

    // --- Account Creation and Initialization ---

    // Get rent sysvar to calculate minimum rent required for new accounts.
    let rent = Rent::from_account_info(rent_sysvar_info)?;

    // STEP 3: Create the vault token account if it doesn't already exist.
    // The vault account is an SPL Token Account that will hold the deposited tokens.
    if deposit_vault_info.data_is_empty() {
        msg!("Creating vault token account...");

        // Calculate the space required for an SPL Token Account.
        let space = spl_token::state::Account::LEN;
        // Calculate the minimum lamports required for rent exemption for the vault.
        let rent_lamports = rent.minimum_balance(space);

        // Instruction to create a new account.
        // The `maker_info.key` pays for the rent.
        // The `deposit_vault_info.key` is the address of the new account.
        // `rent_lamports` is the amount of SOL for rent exemption.
        // `space` is the size of the account data.
        // `token_program_info.key` is the owner of the new account (SPL Token Program).
        let create_vault_instruction = create_account(
            &maker_info.key,
            &deposit_vault_info.key,
            rent_lamports,
            space as u64,
            &token_program_info.key,
        );

        // Signer seeds for the vault PDA to sign the `create_account` instruction.
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vault",
            maker_info.key.as_ref(),
            escrow_seed.as_ref(),
            &[vault_bump], // The bump seed is crucial for PDA signing.
        ]];

        // Invoke the System Program to create the vault account.
        // The `invoke_signed` function is used because the `deposit_vault_info` (which is a PDA)
        // is signing the creation of itself, specifically when its `owner` is set.
        invoke_signed(
            &create_vault_instruction,
            &[
                maker_info.clone(),          // Payer for the account creation
                deposit_vault_info.clone(),  // The account being created
                system_program_info.clone(), // The System Program itself
            ],
            signer_seeds,
        )?;
        msg!("Vault account created.");

        // STEP 4: Initialize the newly created vault account as an SPL Token account.
        // This sets its mint and assigns its authority (owner).
        msg!("Initializing vault as token account...");

        // Instruction to initialize the token account.
        // `TOKEN_PROGRAM_ID`: The program that owns the vault account.
        // `deposit_vault_info.key`: The token account to initialize.
        // `mint_a_info.key`: The mint for this token account (Token A).
        // `vault_pda`: The authority for this token account. This PDA will control tokens in the vault.
        let vault_token_instruction = initialize_account2(
            &TOKEN_PROGRAM_ID,
            &deposit_vault_info.key,
            &mint_a_info.key,
            &vault_pda, // The vault PDA will be the authority for this token account
        )?;

        // Invoke the SPL Token Program to initialize the vault.
        // The `invoke` function is used here because the vault itself is not signing this.
        // The SPL Token Program merely writes data to the account.
        invoke(
            &vault_token_instruction,
            &[
                deposit_vault_info.clone(), // The token account to initialize
                mint_a_info.clone(),        // The mint associated with this token account
                rent_sysvar_info.clone(),   // The rent sysvar is required for `initialize_account2`
            ],
        )?;
        msg!("Vault initialized.");
    }

    // STEP 5: Transfer the maker's Token A from their account into the newly created vault.
    msg!("Transferring {} tokens to vault...", deposit_amount);

    // Create the transfer instruction.
    // `TOKEN_PROGRAM_ID`: The token program.
    // `maker_token_acc_a_info.key`: The source token account (maker's Token A account).
    // `deposit_vault_info.key`: The destination token account (the vault).
    // `maker_info.key`: The authority (owner) of the source token account.
    // `&[maker_info.key]`: The signers required for this transfer (the maker).
    // `deposit_amount`: The amount of tokens to transfer.
    let transfer_instruction = transfer(
        &TOKEN_PROGRAM_ID,
        &maker_token_acc_a_info.key,
        &deposit_vault_info.key,
        &maker_info.key,
        &[maker_info.key],
        deposit_amount,
    )?;

    // Invoke the SPL Token Program to execute the transfer.
    invoke(
        &transfer_instruction,
        &[
            token_program_info.clone(),     // The SPL Token Program
            maker_token_acc_a_info.clone(), // Maker's source Token A account
            maker_info.clone(),             // Maker's account (as signer of the transfer)
            deposit_vault_info.clone(),     // Destination vault account
        ],
    )?;
    msg!("Tokens transferred to vault.");

    // STEP 6: Create the escrow state account if it doesn't already exist.
    // This account will store the details of the escrow.
    if escrow_info.owner != program_id {
        msg!("Creating escrow state account...");

        // Calculate the space required for the Escrow state struct.
        let space = Escrow::ACCOUNT_LEN;
        // Calculate the minimum lamports required for rent exemption for the escrow state.
        let rent_lamports = rent.minimum_balance(space);

        // Instruction to create a new account for the escrow state.
        // `maker_info.key` pays for the rent.
        // `escrow_info.key` is the address of the new account (escrow PDA).
        // `rent_lamports` is the amount of SOL for rent exemption.
        // `space` is the size of the account data.
        // `program_id` is the owner of the new account (this escrow program).
        let escrow_instruction = create_account(
            &maker_info.key,
            &escrow_info.key,
            rent_lamports,
            space as u64,
            program_id, // The escrow program owns this account
        );

        // Signer seeds for the escrow PDA to sign the `create_account` instruction.
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"escrow_vault",
            maker_info.key.as_ref(),
            escrow_seed.as_ref(),
            &[escrow_bump], // The bump seed for the escrow PDA.
        ]];

        // Invoke the System Program to create the escrow state account.
        // `invoke_signed` is used because the `escrow_info` (which is a PDA)
        // is effectively signing its own creation as its `owner` is set.
        invoke_signed(
            &escrow_instruction,
            &[
                maker_info.clone(),          // Payer for the account creation
                escrow_info.clone(),         // The escrow state account being created
                system_program_info.clone(), // The System Program
            ],
            signer_seeds,
        )?;
        msg!("Escrow state account created.");
    }

    // STEP 7: Store the escrow data into the newly created or existing escrow state account.
    msg!("Storing escrow data...");

    // Create an instance of the Escrow struct with the transaction details.
    let escrow_account = Escrow {
        escrow_id,
        maker: *maker_info.key,         // Public key of the maker.
        token_mint_a: *mint_a_info.key, // Mint of the deposited token.
        token_mint_b: *mint_b_info.key, // Mint of the token to receive.
        receive: receive_amount,        // receive amount of Token B.
        bump: escrow_bump,              // Bump seed for the escrow PDA.
    };

    // Pack the Escrow struct data into the escrow_info account's data buffer.
    escrow_account.pack_the_slice_data(&mut escrow_info.data.borrow_mut())?;
    msg!("Escrow data stored.");

    // --- Final Logging ---
    msg!("Escrow initialized successfully!");
    msg!("   -Escrow ID: {}", escrow_id);
    msg!("   -Deposited: {} tokens", deposit_amount);
    msg!("   -Expects: {} tokens in return", receive_amount);
    msg!(" Escrow account: {:#?}", escrow_account);

    Ok(())
}
