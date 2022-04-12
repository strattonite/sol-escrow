use crate::{
    instruction::{decode_instruction, InstructionType::*},
    processor::*,
};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let ix = decode_instruction(instruction_data)?;
    match ix {
        OFFER {
            offer_data,
            index_seed,
        } => create_offer(program_id, accounts, offer_data, index_seed),
        ACCEPT { index_seed } => accept_offer(program_id, accounts, index_seed),
        CANCEL { index_seed } => cancel_offer(program_id, accounts, index_seed),
    }
}
