use crate::state::{EscrowPDA, OfferData};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::*,
    pubkey::Pubkey,
    system_instruction, system_program,
    sysvar::{rent, Sysvar},
};
use spl_token::{instruction, state::Account};

pub fn create_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    offer_data: OfferData,
    index_seed: [u8; 28],
) -> Result<(), ProgramError> {
    let accounts = &mut accounts.iter();
    let seller = next_account_info(accounts)?;
    let seller_temp = next_account_info(accounts)?;
    let seller_receive = next_account_info(accounts)?;
    let escrow_pda = next_account_info(accounts)?;
    let sys_program = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    let temp_info = Account::unpack_from_slice(*seller_temp.try_borrow_data()?)?;
    let receive_info = Account::unpack_from_slice(*seller_receive.try_borrow_data()?)?;
    let seed = offer_data.get_seed();
    let (pda, bump) = Pubkey::find_program_address(&[&seed, &index_seed], program_id);
    let min_rent = rent::Rent::get()?.minimum_balance(EscrowPDA::LEN);

    msg!("asserting validity...");
    if !system_program::check_id(sys_program.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !spl_token::check_id(token_program.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !seller.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !escrow_pda.try_data_is_empty()? {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    if pda != *escrow_pda.key {
        msg!("INVALID PDA ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }
    if temp_info.mint != offer_data.token_type {
        msg!("INVALID TEMP TYPE");
        return Err(ProgramError::InvalidArgument);
    }
    if temp_info.amount != offer_data.token_qty {
        msg!("INVALID TEMP BALANCE");
        return Err(ProgramError::InvalidArgument);
    }
    if receive_info.mint != offer_data.strike_type {
        msg!("INVALID RECEIVE ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("creating escrow PDA");

    let create_ix = system_instruction::create_account(
        seller.key,
        escrow_pda.key,
        min_rent,
        EscrowPDA::LEN as u64,
        program_id,
    );
    invoke_signed(
        &create_ix,
        &[seller.clone(), escrow_pda.clone(), sys_program.clone()],
        &[&[&seed, &index_seed, &[bump]]],
    )?;

    msg!("transferring temp ownership to PDA...");
    let transfer_ix = instruction::set_authority(
        &spl_token::id(),
        seller_temp.key,
        Some(escrow_pda.key),
        instruction::AuthorityType::AccountOwner,
        seller.key,
        &[seller.key],
    )?;
    invoke(
        &transfer_ix,
        &[seller_temp.clone(), seller.clone(), token_program.clone()],
    )?;

    msg!("updating PDA data...");
    let escrow_data = EscrowPDA {
        offer_data,
        seller_main: seller.key.clone(),
        seller_temp: seller_temp.key.clone(),
        seller_receive: seller_receive.key.clone(),
    };
    escrow_data.pack_into_slice(*escrow_pda.data.borrow_mut());
    Ok(())
}

pub fn accept_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    index_seed: [u8; 28],
) -> Result<(), ProgramError> {
    let accounts = &mut accounts.iter();
    let buyer = next_account_info(accounts)?;
    let strike_acc = next_account_info(accounts)?;
    let buyer_receive = next_account_info(accounts)?;
    let escrow_pda = next_account_info(accounts)?;
    let seller = next_account_info(accounts)?;
    let seller_temp = next_account_info(accounts)?;
    let seller_receive = next_account_info(accounts)?;
    let sys_program = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    let token_info = Account::unpack_from_slice(*strike_acc.try_borrow_data()?)?;
    let receive_info = Account::unpack_from_slice(*buyer_receive.try_borrow_data()?)?;
    let escrow_data = EscrowPDA::unpack_from_slice(*escrow_pda.try_borrow_data()?)?;
    let seed = escrow_data.offer_data.get_seed();
    let (_pda, bump) = Pubkey::find_program_address(&[&seed, &index_seed], program_id);

    msg!("asserting validity...");
    if !system_program::check_id(sys_program.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !spl_token::check_id(token_program.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !buyer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if token_info.mint != escrow_data.offer_data.strike_type {
        msg!("INCORRECT TOKEN ACCOUNT TYPE");
        return Err(ProgramError::InvalidArgument);
    }
    if token_info.amount < escrow_data.offer_data.strike_qty {
        msg!("INCORRECT TOKEN ACCOUNT BALANCE");
        return Err(ProgramError::InvalidArgument);
    }
    if receive_info.mint != escrow_data.offer_data.token_type {
        msg!("INCORRECT RECEIVE ACCOUNT TYPE");
        return Err(ProgramError::InvalidArgument);
    }
    if *seller.key != escrow_data.seller_main {
        msg!("INCORRECT SELLER ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }
    if *seller_temp.key != escrow_data.seller_temp {
        msg!("INCORRECT SELLER_TEMP ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }
    if *seller_receive.key != escrow_data.seller_receive {
        msg!("INCORRECT SELLER_RECEIVE ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("transferring asset to buyer");
    let asset_ix = instruction::transfer(
        &spl_token::id(),
        seller_temp.key,
        buyer_receive.key,
        escrow_pda.key,
        &[escrow_pda.key],
        escrow_data.offer_data.token_qty,
    )?;
    invoke_signed(
        &asset_ix,
        &[
            seller_temp.clone(),
            buyer_receive.clone(),
            escrow_pda.clone(),
        ],
        &[&[&seed, &index_seed, &[bump]]],
    )?;

    msg!("transferring strike to seller");
    let strike_ix = instruction::transfer(
        &spl_token::id(),
        strike_acc.key,
        seller_receive.key,
        buyer.key,
        &[buyer.key],
        escrow_data.offer_data.strike_qty,
    )?;
    invoke(
        &strike_ix,
        &[strike_acc.clone(), seller_receive.clone(), buyer.clone()],
    )?;

    msg!("closing temp");
    let close_ix = instruction::close_account(
        &spl_token::id(),
        seller_temp.key,
        seller.key,
        escrow_pda.key,
        &[escrow_pda.key],
    )?;
    invoke_signed(
        &close_ix,
        &[seller_temp.clone(), seller.clone(), escrow_pda.clone()],
        &[&[&seed, &index_seed, &[bump]]],
    )?;
    msg!("closing PDA");
    *escrow_pda.data.borrow_mut() = &mut [];
    **seller.try_borrow_mut_lamports()? += escrow_pda.try_lamports()?;
    **escrow_pda.try_borrow_mut_lamports()? = 0;
    Ok(())
}

pub fn cancel_offer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    index_seed: [u8; 28],
) -> Result<(), ProgramError> {
    let accounts = &mut accounts.iter();
    let seller = next_account_info(accounts)?;
    let seller_temp = next_account_info(accounts)?;
    let seller_receive = next_account_info(accounts)?;
    let escrow_pda = next_account_info(accounts)?;
    let sys_program = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    let escrow_data = EscrowPDA::unpack_from_slice(*escrow_pda.try_borrow_data()?)?;
    let seed = escrow_data.offer_data.get_seed();
    let (_pda, bump) = Pubkey::find_program_address(&[&seed, &index_seed], program_id);

    msg!("asserting validity...");
    if !system_program::check_id(sys_program.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !spl_token::check_id(token_program.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *seller.key != escrow_data.seller_main {
        msg!("INCORRECT SELLER ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }
    if *seller_temp.key != escrow_data.seller_temp {
        msg!("INCORRECT SELLER_TEMP ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }
    if *seller_receive.key != escrow_data.seller_receive {
        msg!("INCORRECT SELLER_RECEIVE ACCOUNT");
        return Err(ProgramError::InvalidArgument);
    }
    if !seller.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    msg!("transferring temp ownership back to seller");
    let ix = instruction::set_authority(
        &spl_token::id(),
        seller_temp.key,
        Some(seller.key),
        instruction::AuthorityType::AccountOwner,
        escrow_pda.key,
        &[escrow_pda.key],
    )?;
    invoke_signed(
        &ix,
        &[seller_temp.clone(), seller.clone(), escrow_pda.clone()],
        &[&[&seed, &index_seed, &[bump]]],
    )?;

    msg!("closing PDA");
    *escrow_pda.data.borrow_mut() = &mut [];
    **seller.try_borrow_mut_lamports()? += escrow_pda.try_lamports()?;
    **escrow_pda.try_borrow_mut_lamports()? = 0;
    Ok(())
}
