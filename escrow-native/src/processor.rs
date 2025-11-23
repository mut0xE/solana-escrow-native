use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instructions::{
    instruction::EscrowInstruction, make::initialize_escrow, refund::cancel_escrow,
    take::release_funds,
};
pub struct Process;
impl Process {
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
        let instruction =
            EscrowInstruction::unpack(data).map_err(|_| ProgramError::InvalidAccountData)?;
        match instruction {
            EscrowInstruction::InitializeEscrow {
                escrow_id,
                deposit_amount,
                receive_amount,
            } => initialize_escrow(
                program_id,
                accounts,
                escrow_id,
                deposit_amount,
                receive_amount,
            ),
            EscrowInstruction::ReleaseFunds { escrow_id } => {
                release_funds(program_id, accounts, escrow_id)
            }
            EscrowInstruction::CancelEscrow { escrow_id } => {
                cancel_escrow(program_id, accounts, escrow_id)
            }
        }
    }
}
