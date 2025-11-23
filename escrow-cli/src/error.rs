use thiserror::Error;

#[derive(Debug, Error)]
pub enum EscrowCliError {
    /// Network connection failed
    #[error("Failed to connect to network: {0}")]
    NetworkConnection(String),
    /// Invalid wallet keypair
    #[error("Failed to load wallet: {0}")]
    WalletLoad(String),
    /// Insufficient SOL balance
    #[error("Insufficient SOL balance: need {needed} lamports, have {actual} lamports")]
    InsufficientSol { needed: u64, actual: u64 },
    /// Insufficient token balance
    #[error("Insufficient token balance: need {needed}, have {actual}")]
    InsufficientTokens { needed: u64, actual: u64 },
    /// Invalid public key
    #[error("Invalid public key: {0}")]
    InvalidPubkey(String),
    /// Token account doesn't exist
    #[error("Token account does not exist: {0}")]
    TokenAccountNotFound(String),
    /// Failed to create token account
    #[error("Failed to create token account: {0}")]
    TokenAccountCreation(String),
    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    /// RPC error
    #[error("RPC error: {0}")]
    RpcError(String),
    /// Invalid program ID
    #[error("Invalid program ID: {0}")]
    InvalidProgramId(String),
    /// Escrow not found
    #[error("Escrow not found for maker {maker} with ID {id}")]
    EscrowNotFound { maker: String, id: u64 },
    /// Custom Escrow Error
    #[error("Error:{0}")]
    CustomError(String),
}
// Convert from solana_client errors
impl From<solana_client::client_error::ClientError> for EscrowCliError {
    fn from(error: solana_client::client_error::ClientError) -> Self {
        EscrowCliError::RpcError(error.to_string())
    }
}
// Convert from solana_sdk errors
impl From<solana_sdk::signer::SignerError> for EscrowCliError {
    fn from(error: solana_sdk::signer::SignerError) -> Self {
        EscrowCliError::WalletLoad(error.to_string())
    }
}
