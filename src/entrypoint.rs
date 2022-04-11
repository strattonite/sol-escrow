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
        OFFER { offer_data } => create_offer(program_id, accounts, offer_data),
        ACCEPT => accept_offer(program_id, accounts),
        CANCEL => cancel_offer(program_id, accounts),
    }
}
