use crate::state::OfferData;
use solana_program::program_error::ProgramError;
use std::convert::TryInto;

pub enum InstructionType {
    OFFER { offer_data: OfferData },
    ACCEPT,
    CANCEL,
}

pub fn decode_instruction(instruction_data: &[u8]) -> Result<InstructionType, ProgramError> {
    match instruction_data[0] {
        0 => Ok(InstructionType::OFFER {
            offer_data: OfferData::from_bytes(instruction_data[1..].try_into().unwrap()),
        }),
        1 => Ok(InstructionType::ACCEPT),
        2 => Ok(InstructionType::CANCEL),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
