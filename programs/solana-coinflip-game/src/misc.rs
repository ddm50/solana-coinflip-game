use std::mem::size_of;

use anchor_lang::{
    solana_program::{account_info::AccountInfo, program_error::ProgramError},
    AccountDeserialize,
};
use orao_solana_vrf::state::Randomness;

pub fn get_account_data(account_info: &AccountInfo) -> Result<Randomness, ProgramError> {
    if account_info.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let account = Randomness::try_deserialize(&mut &account_info.data.borrow()[..])?;

    if false {
        Err(ProgramError::UninitializedAccount)
    } else {
        Ok(account)
    }
}


pub fn current_state(randomness: &Randomness) ->u64 {
    if let Some(randomness) = randomness.fulfilled() {
        let value = randomness[0..size_of::<u64>()].try_into().unwrap();
        
        return u64::from_le_bytes(value);
    } else {
        return 0;

    }
}