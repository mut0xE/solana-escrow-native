use clap::{arg, Parser, Subcommand};
use colored::Colorize;
use dotenv::dotenv;
use escrow_cli::{
    commands::{
        cancel::cancel_escrow, exchange::exchange_funds, initialize::initialize_escrow,
        view::view_escrow,
    },
    error::EscrowCliError,
    helper::{check_sol_balance, connect_to_network, get_wallet},
};
use solana_sdk::signer::Signer;

#[derive(Parser)]
#[command(name = "escrow-cli")]
#[command(about="A CLI TOOL FOR ESCROW PROGRAM",long_about=None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
    // Network to use
    #[arg(short, long, default_value = "devnet", global = true)]
    network: String,
}
#[derive(Subcommand)]
enum Commands {
    /// Initialize a new escrow (offer tokens for exchange)
    Initialize {
        /// Path to your wallet keypair (e.g., ~/.config/solana/id.json)
        #[arg(short = 'w', long)]
        wallet: String,
        /// Token A mint address (what you're offering)
        #[arg(short = 'a', long)]
        mint_a: String,
        /// Token B mint address (what you want in return)
        #[arg(short = 'b', long)]
        mint_b: String,
        /// Amount of Token A to deposit (in smallest units)
        #[arg(short = 'd', long)]
        deposit: u64,
        /// Amount of Token B you want (in smallest units)
        #[arg(short = 'r', long)]
        receive: u64,
        /// Unique escrow ID (choose any number)
        #[arg(short = 'i', long)]
        escrow_id: u64,
    },
    /// Take escrow
    Exchange {
        #[arg(short, long)]
        wallet: String,

        #[arg(short, long)]
        escrow_id: u64,

        #[arg(short, long)]
        maker: String,
    },
    /// Cancel escrow
    Cancel {
        #[arg(short, long)]
        wallet: String,
        /// Token A mint address (what you're offering)
        #[arg(short = 'a', long)]
        mint_a: String,
        #[arg(short, long)]
        escrow_id: u64,
    },

    /// View escrow details
    View {
        #[arg(short, long)]
        escrow_id: u64,

        #[arg(short, long)]
        maker: String,
    },
}
#[tokio::main]
async fn main() -> Result<(), EscrowCliError> {
    let cli = Cli::parse();
    dotenv().ok();
    let program_id = std::env::var("ESCROW_PROGRAM_ID").map_err(|e| {
        EscrowCliError::CustomError(format!(
            "Failed to read ESCROW_PROGRAM_ID from environment: {}",
            e
        ))
    })?;
    // Connect to network
    let client = match connect_to_network(&cli.network) {
        Ok(client) => {
            println!(
                "{} {} {}",
                "Connected to".bold().green(),
                cli.network.to_uppercase().bold().blue(),
                "successfully!".bold().green()
            );
            client
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                "Failed to connect".bold().red(),
                e.to_string().red()
            );
            return Err(EscrowCliError::NetworkConnection(
                "Failed to connect".to_string(),
            ));
        }
    };
    match cli.commands {
        Commands::Initialize {
            wallet,
            mint_a,
            mint_b,
            deposit,
            receive,
            escrow_id,
        } => {
            // println!("INITIALIZE ESCROW");
            // println!("   Wallet path: {}", wallet);
            // println!("   Token A mint: {}", mint_a);
            // println!("   Token B mint: {}", mint_b);
            // println!("   Deposit amount: {}", deposit);
            // println!("   Receive amount: {}", receive);
            // println!("   Escrow ID: {}", escrow_id);

            let maker = get_wallet(&wallet).map_err(|e| {
                EscrowCliError::WalletLoad(format!("failed to get payer keypair:{}", e))
            })?;
            let sol_balance = check_sol_balance(&client, &maker.pubkey())
                .await
                .map_err(|e| EscrowCliError::RpcError(e.to_string()))?;
            if sol_balance < 5_000_000 {
                eprintln!("You need at least 0.005 SOL for transaction fees");
                return Err(EscrowCliError::InsufficientSol {
                    needed: 5_000_000,
                    actual: sol_balance,
                })?;
            }
            initialize_escrow(
                &cli.network,
                &program_id,
                &maker,
                &mint_a,
                &mint_b,
                deposit,
                receive,
                escrow_id,
                &client,
            )
            .await
            .map_err(|e| {
                EscrowCliError::CustomError(format!("Failed to initialize escrow: {}", e))
            })?;
        }
        Commands::Exchange {
            wallet,
            escrow_id,
            maker,
        } => {
            let taker = get_wallet(&wallet).map_err(|e| {
                EscrowCliError::WalletLoad(format!("failed to get tayer keypair:{}", e))
            })?;
            let maker = get_wallet(&maker).map_err(|e| {
                EscrowCliError::WalletLoad(format!("failed to get maker keypair:{}", e))
            })?;
            let sol_balance = check_sol_balance(&client, &taker.pubkey())
                .await
                .map_err(|e| EscrowCliError::RpcError(e.to_string()))?;
            if sol_balance < 5_000_000 {
                eprintln!("You need at least 0.005 SOL for transaction fees");
                return Err(EscrowCliError::InsufficientSol {
                    needed: 5_000_000,
                    actual: sol_balance,
                })?;
            };
            exchange_funds(
                &cli.network,
                &program_id,
                &taker,
                &maker,
                escrow_id,
                &client,
            )
            .await
            .map_err(|e| {
                EscrowCliError::CustomError(format!("Failed to exchange the funds:{}", e))
            })?
        }
        Commands::Cancel {
            wallet,
            escrow_id,
            mint_a,
        } => {
            let maker = get_wallet(&wallet).map_err(|e| {
                EscrowCliError::WalletLoad(format!("failed to get payer keypair:{}", e))
            })?;
            let sol_balance = check_sol_balance(&client, &maker.pubkey())
                .await
                .map_err(|e| EscrowCliError::RpcError(e.to_string()))?;
            if sol_balance < 5_000_000 {
                eprintln!("You need at least 0.005 SOL for transaction fees");
                return Err(EscrowCliError::InsufficientSol {
                    needed: 5_000_000,
                    actual: sol_balance,
                })?;
            };
            cancel_escrow(
                &cli.network,
                &maker,
                escrow_id,
                &program_id,
                &mint_a,
                &client,
            )
            .await
            .map_err(|e| {
                EscrowCliError::CustomError(format!("Failed to cancel the escrow:{}", e))
            })?;
        }
        Commands::View { escrow_id, maker } => {
            view_escrow(&cli.network, &program_id, &maker, escrow_id, &client)
                .await
                .map_err(|e| {
                    EscrowCliError::CustomError(format!("Failed to view the escrow:{}", e))
                })?;
        }
    }
    Ok(())
}
