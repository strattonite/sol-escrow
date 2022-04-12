use crate::state::{get_seed, OfferData};
use solana_program::program_error::ProgramError;
use std::convert::TryInto;

// index seed format: [0..32 = initialiser main pubkey, 32 = contract_type, 33..41 = contract_no (u64)]
pub enum InstructionType {
    OFFER {
        offer_data: OfferData,
        index_seed: [u8; 28],
    },
    ACCEPT {
        index_seed: [u8; 28],
    },
    CANCEL {
        index_seed: [u8; 28],
    },
}

pub fn decode_instruction(instruction_data: &[u8]) -> Result<InstructionType, ProgramError> {
    match instruction_data[0] {
        0 => Ok(InstructionType::OFFER {
            offer_data: OfferData::from_bytes(
                instruction_data[1..OfferData::LEN + 1].try_into().unwrap(),
            ),
            index_seed: get_seed(&instruction_data[OfferData::LEN + 1..]),
        }),
        1 => Ok(InstructionType::ACCEPT {
            index_seed: get_seed(&instruction_data[1..]),
        }),
        2 => Ok(InstructionType::CANCEL {
            index_seed: get_seed(&instruction_data[1..]),
        }),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
