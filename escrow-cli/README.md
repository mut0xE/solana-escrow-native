Simple Escrow CLI — README

This README guides you through using a minimal command-line tool for trustless, atomic swaps of two SPL tokens on Solana using a custom escrow program. Learn the full flow, token roles, and how Maker and Taker interact to perform a safe exchange.

📌 What Is This Escrow?

This escrow program enables:

*   **Maker** to lock Token A into a Program Derived Address (PDA) vault.
*   **Taker** to send Token B to the Maker.
*   The **Program** to automatically transfer Token A from the vault to the Taker.

This ensures a fair, atomic swap: no partial transfers, no cheating.

✔ Both sides receive tokens
✔ Fully on-chain


📁 Wallet Setup

You'll need two separate keypair files to simulate Maker and Taker roles:

1.  **Create Wallets:**
    ```bash
    solana-keygen new -o ./maker.json  # Wallet for the Maker
    solana-keygen new -o ./taker.json  # Wallet for the Taker
    ```

2.  **Airdrop SOL:** Fund both wallets with some SOL for transaction fees (on Devnet):
    ```bash
    solana airdrop 2 ./maker.json
    solana airdrop 2 ./taker.json
    solana balance ./maker.json # Verify balance
    solana balance ./taker.json # Verify balance
    ```

🔧 Program ID and Environment Variables

To simplify command execution, we'll use a `.env` file to manage the program ID and token mint addresses.

1.  **Create `.env` file:**
    Create a file named `.env` in the project root with the following content:
    ```
    ESCROW_PROGRAM_ID=Cv89UWHyKLcByH2HByxWeKdRKBcqqB8sibdgC2KUzjYK
    ```

    This `ESCROW_PROGRAM_ID` is the public key of the deployed escrow program.

🧱 Token Setup (Important)

For this demonstration, one entity will act as the **Mint Authority** to create and distribute both tokens. We'll assume the `maker.json` wallet acts as this Mint Authority.

Here's the planned token distribution for the swap:
*   **Token A (Maker's deposit):** Minted by Maker to Maker.
*   **Token B (Taker's payment):** Minted by Maker to Taker.

All token amounts will be specified in their **smallest units** (e.g., if decimals=9, `1,000,000,000` units equals 1 token).

Let's set up:

1.  **Create Token A Mint:**
    ```bash
    spl-token create-token --owner $(solana-keygen pubkey ./maker.json)
    # Save the output. For example, add to your .env file: TOKEN_A_MINT="<MINT_ADDRESS_A>"
    ```

2.  **Create Token B Mint:**
    ```bash
    spl-token create-token --owner $(solana-keygen pubkey ./maker.json)
    # Save the output. For example, add to your .env file: TOKEN_B_MINT="<MINT_ADDRESS_B>"
    ```

3.  **Create Associated Token Accounts (ATAs):**
    Both Maker and Taker need ATAs for *both* Token A and Token B to facilitate the swap.

    *   **Maker's ATAs:**
        ```bash
        spl-token create-account $TOKEN_A_MINT --owner $(solana-keygen pubkey ./maker.json)
        spl-token create-account $TOKEN_B_MINT --owner $(solana-keygen pubkey ./maker.json)
        ```

    *   **Taker's ATAs:**
        ```bash
        spl-token create-account $TOKEN_A_MINT --owner $(solana-keygen pubkey ./taker.json)
        spl-token create-account $TOKEN_B_MINT --owner $(solana-keygen pubkey ./taker.json)
        ```

4.  **Mint Initial Tokens:**
    *   **Mint Token A to Maker:** (Maker will deposit this into the escrow)
        ```bash
        spl-token mint $TOKEN_A_MINT 1000000000 $(spl-token address --owner ./maker.json --token $TOKEN_A_MINT)
        # This mints 1 Token A (assuming 9 decimals)
        ```

    *   **Mint Token B to Taker:** (Taker will use this for the exchange)
        ```bash
        spl-token mint $TOKEN_B_MINT 500000000 $(spl-token address --owner ./taker.json --token $TOKEN_B_MINT)
        # This mints 0.5 Token B (assuming 9 decimals)
        ```
    Now both parties have the necessary liquidity for the swap.

🚀 Using the Escrow CLI

First, build the project:
```bash
cargo build --release
```
All commands below assume you are running them from the project root. You might need to prefix `cargo run` with `./` or use `target/release/escrow-cli` directly depending on your shell/environment.

1️⃣ Maker Creates Escrow (Locks Token A)
The Maker initializes the escrow, depositing Token A and specifying how much of Token B they want in return.

```bash
dotenv run cargo run -- initialize \
  --program-id $ESCROW_PROGRAM_ID \
  --wallet ./maker.json \
  --mint-a $TOKEN_A_MINT \
  --mint-b $TOKEN_B_MINT \
  --deposit 1000000000 \ # Maker deposits 1 Token A (1 * 10^9, assuming 9 decimals)
  --receive 500000000 \  # Maker wants 0.5 Token B in return (0.5 * 10^9, assuming 9 decimals)
  --escrow-id 1          # A unique identifier for this escrow
```
This action locks the specified amount of Token A into a PDA controlled by the escrow program.

2️⃣ View Escrow

Anyone can inspect the details of an active escrow:

```bash
dotenv run cargo run -- view \
  --program-id $ESCROW_PROGRAM_ID \
  --escrow-id 1 \
  --maker $(solana-keygen pubkey ./maker.json)
```

3️⃣ Taker Accepts Escrow (Atomic Swap)

The Taker executes the swap. They send the required amount of Token B to the Maker, and the program automatically releases Token A from the escrow vault to the Taker.

```bash
dotenv run cargo run -- exchange \
  --program-id $ESCROW_PROGRAM_ID \
  --wallet ./taker.json \
  --maker $(solana-keygen pubkey ./maker.json) \
  --escrow-id 1
```
Upon successful exchange, the program automatically:
✔ Transfers Token B from Taker's ATA to Maker's ATA.
✔ Transfers Token A from the escrow PDA to Taker's ATA.
✔ Closes the temporary PDA accounts created for the escrow.
✔ Returns the rent collected for the PDA accounts to the Maker.

4️⃣ Cancel Escrow (Maker Only)

If the Maker decides to revoke the offer before it's accepted by a Taker, they can cancel the escrow.

```bash
dotenv run cargo run -- cancel \
  --program-id $ESCROW_PROGRAM_ID \
  --wallet ./maker.json \
  --mint-a $TOKEN_A_MINT \
  --escrow-id 1
```
This action returns the locked Token A from the escrow PDA back to the Maker's ATA and closes the escrow accounts.


⚠️ Important Considerations

*   **Mint Authority:** In this demo, one entity (the Maker) acts as the Mint Authority for both Token A and Token B for simplicity. In a real-world scenario, tokens would likely have separate mint authorities.
*   **Environment Variables (`.env`):** The guide uses a `.env` file and `dotenv-cli` to manage the program ID and token mint addresses, simplifying command execution. Ensure `dotenv-cli` is installed (`npm install -g dotenv-cli`) or manually export variables.
*   **Associated Token Accounts (ATAs):** Both the Maker and Taker must have Associated Token Accounts for *both* tokens involved in the swap. The setup steps guide you through creating these.
*   **Smallest Units:** All token amounts (e.g., `--deposit`, `--receive`, `spl-token mint`) must be specified in their smallest possible units (e.g., `1_000_000_000` for 1 token with 9 decimals).
*   **Network:** Ensure your Solana CLI is configured to `devnet` or your desired network using `solana config set --url <network>`.

🎯 Summary of the Escrow Process

This simple escrow allows for a secure, atomic exchange:
*   **Token Minting:** Maker (as Mint Authority) creates both Token A and Token B.
*   **Initial Balance:** Maker holds Token A, Taker holds Token B.
*   **Escrow Creation:** Maker initiates the escrow, locking Token A and defining the desired Token B amount.
*   **Escrow Acceptance:** Taker accepts the escrow.
*   **Atomic Transfer:** The program automatically facilitates the transfer: Token B to Maker, Token A to Taker.
*   **Account Cleanup:** Escrow-related accounts are closed, and rent is returned.
