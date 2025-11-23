# Simple Escrow Program

This repository contains a simple escrow program built on the Solana blockchain. It demonstrates a basic escrow functionality where a maker can create an escrow, a taker can take the escrow, and the maker can refund it if not taken.

## Project Structure

The project is organized into two main components:

- `escrow-native`: This directory contains the Solana program (smart contract) written in Rust.
- `escrow-cli`: This directory contains a command-line interface (CLI) tool for interacting with the `escrow-native` program.

## `escrow-native` Program Details

The `escrow-native` program defines the core logic for the escrow system.

### Instructions

The `escrow-native` program supports the following instructions, corresponding to the `EscrowInstruction` enum:

1.  **`InitializeEscrow`**
    *   **Description**: Creates a new escrow account, locking the maker's tokens. The maker specifies the `deposit_amount` of tokens they are putting into escrow and the `receive_amount` of SOL they expect to receive from the taker.
    *   **Accounts**: Maker's token account, escrow account (PDA), escrow's token account (PDA), mint account of the token, System program, Token program, Rent sysvar.
    *   **Data**: `escrow_id` (unique identifier), `deposit_amount` (tokens the maker provides), `receive_amount` (SOL the maker wants).

2.  **`ReleaseFunds` (Take Escrow)**
    *   **Description**: Allows the taker to complete the escrow. The taker sends the `receive_amount` of SOL to the maker, and in return, the maker's tokens from the escrow are transferred to the taker.
    *   **Accounts**: Taker's SOL account, taker's token account, maker's SOL account, escrow account, escrow's token account, PDA account (escrow authority), System program, Token program.
    *   **Data**: `escrow_id` (unique identifier).

3.  **`CancelEscrow` (Refund Escrow)**
    *   **Description**: The maker can cancel an active escrow if it has not been `ReleaseFunds` (taken) by the taker. The locked tokens are returned to the maker.
    *   **Accounts**: Maker's token account, escrow account, escrow's token account, PDA account (escrow authority), Token program.
    *   **Data**: `escrow_id` (unique identifier).

### Workflow Example

1.  **Maker Initiates**: The maker calls `InitializeEscrow`, creating a new escrow. Their specified `deposit_amount` of tokens is locked in an escrow-specific token account.
2.  **Taker Takes**: The taker calls `ReleaseFunds`, sending the agreed `receive_amount` of SOL to the maker. The tokens from the escrow are then transferred to the taker.
3.  **(Alternative) Maker Refunds**: If the taker does not `ReleaseFunds`, the maker can call `CancelEscrow` to retrieve their tokens from the escrow.

## Getting Started

Detailed instructions on how to build, deploy, and interact with the program using the `escrow-cli` will be provided in the `escrow-cli/README.md` file.

## Contributing

Feel free to open issues or submit pull requests if you have any suggestions or improvements.
