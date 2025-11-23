use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::io::Error;
#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    /// Unique identifier for this escrow account.
    pub escrow_id: u64,
    /// The public key of the account that created this escrow (the maker).
    pub maker: Pubkey,
    /// The public key of the token mint for token A, which the maker is depositing.
    pub token_mint_a: Pubkey,
    /// The public key of the token mint for token B, which the maker wants to receive.
    pub token_mint_b: Pubkey,
    /// The amount of token B the maker expects to receive.
    pub receive: u64,
    /// The bump seed used to derive the PDA for this escrow account.
    pub bump: u8,
}
impl Escrow {
    pub const ACCOUNT_LEN: usize = 8 + 32 + 32 + 32 + 8 + 1;

    /// Serializes the `Escrow` struct.
    ///
    /// It `expect`s serialization to not fail, as `Escrow` contains only types
    /// that are guaranteed to serialize successfully with Borsh.
    pub fn pack_the_slice_data(&self, data: &mut [u8]) -> Result<(), Error> {
        self.serialize(&mut &mut data[..])
    }
    pub fn unpack_the_slice_data(data: &[u8]) -> Result<Self, ProgramError> {
        BorshDeserialize::try_from_slice(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}
