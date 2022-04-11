use arrayref::{array_refs, mut_array_refs};
use sha2::{Digest, Sha256};
use solana_program::{program_error::ProgramError, program_pack::*, pubkey::Pubkey};
use std::convert::TryInto;

#[derive(Debug, PartialEq)]
pub struct OfferData {
    pub token_type: Pubkey,
    pub token_qty: u64,
    pub strike_type: Pubkey,
    pub strike_qty: u64,
}

#[derive(Debug, PartialEq)]
pub struct EscrowPDA {
    pub offer_data: OfferData,
    pub seller_main: Pubkey,
    pub seller_temp: Pubkey,
    pub seller_receive: Pubkey,
}

impl OfferData {
    pub fn from_bytes(src: &[u8; 80]) -> Self {
        let (token_type, token_qty, strike_type, strike_qty) = array_refs![src, 32, 8, 32, 8];
        let token_type = Pubkey::new_from_array(*token_type);
        let token_qty = u64::from_le_bytes(*token_qty);
        let strike_type = Pubkey::new_from_array(*strike_type);
        let strike_qty = u64::from_le_bytes(*strike_qty);

        OfferData {
            token_type,
            token_qty,
            strike_type,
            strike_qty,
        }
    }

    pub fn to_bytes(&self) -> [u8; 80] {
        let mut dst = [0; 80];
        let (token_type, token_qty, strike_type, strike_qty) =
            mut_array_refs![&mut dst, 32, 8, 32, 8];
        *token_type = self.token_type.to_bytes();
        *token_qty = self.token_qty.to_le_bytes();
        *strike_type = self.strike_type.to_bytes();
        *strike_qty = self.strike_qty.to_le_bytes();
        dst
    }

    pub fn get_seed(&self) -> [u8; 32] {
        get_seed(&self.to_bytes())
    }
}

impl Sealed for EscrowPDA {}

impl Pack for EscrowPDA {
    const LEN: usize = 176;

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src: &[u8; 176] = src.try_into().unwrap();
        let (seller_main, seller_temp, seller_receive, offer_data) =
            array_refs![src, 32, 32, 32, 80];
        let seller_main = Pubkey::new_from_array(*seller_main);
        let seller_temp = Pubkey::new_from_array(*seller_temp);
        let seller_receive = Pubkey::new_from_array(*seller_receive);
        let offer_data = OfferData::from_bytes(offer_data);

        Ok(EscrowPDA {
            offer_data,
            seller_main,
            seller_temp,
            seller_receive,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst: &mut [u8; 176] = dst.try_into().unwrap();
        let (seller_main, seller_temp, seller_receive, offer_data) =
            mut_array_refs![dst, 32, 32, 32, 80];

        seller_main.copy_from_slice(&self.seller_main.to_bytes());
        seller_temp.copy_from_slice(&self.seller_temp.to_bytes());
        seller_receive.copy_from_slice(&self.seller_receive.to_bytes());
        offer_data.copy_from_slice(&self.offer_data.to_bytes());
    }
}

pub fn get_seed(bytes: &[u8; 80]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let seed: [u8; 32] = hasher.finalize().try_into().unwrap();
    seed
}
