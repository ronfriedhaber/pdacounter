#![deny(clippy::all)]
#![forbid(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    borsh::try_from_slice_unchecked,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
    pub bump: u8,
}

impl Counter {
    pub const LEN: usize = 32 + 8 + 1;
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum CounterIx {
    Init,
    Increment,
    Decrement,
}

entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ix_data: &[u8],
) -> ProgramResult {
    let ix =
        CounterIx::try_from_slice(ix_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    match ix {
        CounterIx::Init => process_init(program_id, accounts),
        CounterIx::Increment => process_crement(program_id, accounts, true),
        CounterIx::Decrement => process_crement(program_id, accounts, false),
    }
}
fn process_init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let acc_iter = &mut accounts.iter();
    let authority = next_account_info(acc_iter)?;
    let counter_pda = next_account_info(acc_iter)?;
    let system_program = next_account_info(acc_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected_pda, bump) =
        Pubkey::find_program_address(&[b"counter", authority.key.as_ref()], program_id);
    if *counter_pda.key != expected_pda {
        msg!("Counter PDA does not match derived address");
        return Err(ProgramError::InvalidSeeds);
    }
    if counter_pda.owner != program_id {
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(Counter::LEN);

        let create_ix = system_instruction::create_account(
            authority.key,
            counter_pda.key,
            required_lamports,
            Counter::LEN as u64,
            program_id,
        );

        invoke_signed(
            &create_ix,
            &[
                authority.clone(),
                counter_pda.clone(),
                system_program.clone(),
            ],
            &[&[b"counter", authority.key.as_ref(), &[bump]]],
        )?;
    }

    let mut data: Counter = try_from_slice_unchecked::<Counter>(&counter_pda.data.borrow())?;
    data.authority = *authority.key;
    data.count = 0;
    data.bump = bump;
    data.serialize(&mut &mut counter_pda.data.borrow_mut()[..])?;

    Ok(())
}

fn process_crement(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    increment: bool,
) -> ProgramResult {
    let acc_iter = &mut accounts.iter();
    let authority = next_account_info(acc_iter)?;
    let counter_pda = next_account_info(acc_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if counter_pda.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut data: Counter = try_from_slice_unchecked::<Counter>(&counter_pda.data.borrow())?;
    let (expected_pda, _bump) =
        Pubkey::find_program_address(&[b"counter", data.authority.as_ref()], program_id);
    if *counter_pda.key != expected_pda {
        return Err(ProgramError::InvalidSeeds);
    }
    if data.authority != *authority.key {
        return Err(ProgramError::IllegalOwner);
    }

    data.count = if increment {
        data.count.checked_add(1)
    } else {
        data.count.checked_sub(1)
    }
    .ok_or(ProgramError::InvalidInstructionData)?;
    data.serialize(&mut &mut counter_pda.data.borrow_mut()[..])?;

    Ok(())
}
