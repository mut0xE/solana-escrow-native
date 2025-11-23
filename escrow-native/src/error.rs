use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EscrowError {
    #[error("Account not owned by program")]
    InvalidAccountOwner,
    #[error("PDA Derivation Mismatched")]
    PDADerivationMismatch,
    #[error("creator must be a signer")]
    MissingRequiredSignature,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("IncorrectProgramId")]
    IncorrectProgramId,
    #[error("Invalid amount: must be greater than zero")]
    InvalidAmount,
    #[error("InvalidAccountData")]
    InvalidAccountData,
    #[error("InvalidAuthority")]
    InvalidAuthority,
    #[error("InvalidMint")]
    InvalidMint,
    #[error("Invaild Escrow Id")]
    InvalidEscrowId,
    #[error("Arithmetic Overflow")]
    ArithmeticOverflow,
}
impl From<EscrowError> for ProgramError {
    fn from(error: EscrowError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
