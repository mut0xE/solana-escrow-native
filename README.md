# Simple Escrow Program

This repository contains a simple escrow program built on the Solana blockchain. It demonstrates a basic escrow functionality where a maker can create an escrow, a taker can complete the exchange, and the maker can cancel the escrow if the exchange doesn't occur.

## Project Structure

The project is organized into two main components:

-   `escrow-native`: This directory contains the Solana program (smart contract) written in Rust. It defines the on-chain logic for managing escrows.
-   `escrow-cli`: This directory contains a command-line interface (CLI) tool built in Rust for interacting with the `escrow-native` program.

## `escrow-native` Program Overview

The `escrow-native` program is the core smart contract responsible for handling the escrow logic. It allows participants to:

-   **Initialize an Escrow (`InitializeEscrow`)**: A maker can initiate an escrow by specifying the amount of tokens they wish to deposit and the amount of SOL they expect to receive. The maker's tokens are then locked into a program-controlled escrow account.
-   **Release Funds (`ReleaseFunds`)**: A taker can execute the escrow by sending the agreed-upon SOL amount to the maker. In return, the tokens held in escrow are transferred to the taker. This effectively completes the exchange.
-   **Cancel Escrow (`CancelEscrow`)**: If the taker does not release funds, the maker can cancel the escrow. This action returns the locked tokens from the escrow account back to the maker.

### Workflow Example

1.  **Maker Initializes**: A maker calls the `InitializeEscrow` instruction, providing a unique `escrow_id`, the `deposit_amount` of tokens, and the `receive_amount` of SOL. The specified tokens are moved from the maker's token account into the escrow's dedicated token account.
2.  **Taker Takes**: A taker calls the `ReleaseFunds` instruction, using the `escrow_id`. They transfer the `receive_amount` of SOL to the maker's SOL account. The program then transfers the `deposit_amount` of tokens from the escrow's token account to the taker's token account.
3.  **(Alternative) Maker Refunds**: If the taker does not take the escrow (i.e., `ReleaseFunds` is not called), the maker can call `CancelEscrow` using the `escrow_id`. The tokens locked in the escrow's token account are returned to the maker's original token account.

## `escrow-cli` Overview

The `escrow-cli` is a command-line utility designed to simplify interaction with the `escrow-native` program. It provides a convenient way to:

-   Deploy the `escrow-native` program to a Solana cluster.
-   Initialize new escrows.
-   Execute fund releases (taking an escrow).
-   Cancel existing escrows.
-   Query the state of escrow accounts.

## Getting Started

To get started with this project, you will need the Solana Tool Suite installed.

Detailed instructions on how to build, deploy, and interact with the program using the `escrow-cli` can be found in the `escrow-cli/README.md` file.

## Contributing

Feel free to open issues or submit pull requests if you have any suggestions or improvements.