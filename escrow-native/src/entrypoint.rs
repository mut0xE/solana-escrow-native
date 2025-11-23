use solana_program::{
    account_info::AccountInfo,
    entrypoint::{entrypoint, ProgramResult},
    pubkey::Pubkey,
};

use crate::processor::Process;

entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    Process::process(program_id, accounts, data)?;
    Ok(())
}
