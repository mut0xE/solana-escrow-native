use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;

/// Represents the various instructions that can be sent to the escrow program.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum EscrowInstruction {
    /// Initializes a new escrow account.
    ///
    /// The `escrow_id` uniquely identifies the escrow.
    /// The `amount` specifies the value of the escrow.
    /// Accounts expected:
    InitializeEscrow {
        escrow_id: u64,
        deposit_amount: u64,
        receive_amount: u64,
    },
    /// Releases funds from an existing escrow account.
    ///
    /// The `escrow_id` identifies the escrow to release funds from.
    /// Accounts expected:
    ReleaseFunds { escrow_id: u64 },
    /// Cancels an existing escrow account, returning funds to the initializer.
    ///
    /// The `escrow_id` parameter identifies the escrow to cancel.
    /// Accounts expected:
    CancelEscrow { escrow_id: u64 },
}
impl EscrowInstruction {
    /// Deserializes an `Escrow` struct from a byte slice.
    ///
    /// This function uses `borsh::try_from_slice` to attempt deserialization.
    /// If deserialization fails, it returns a `ProgramError::InvalidAccountData`.
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}
