
use std::default;

use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use solana_program::{
    system_program::ID as SYSTEM_PROGRAM_ID,
    };
use crate::pda::{self};
use orao_solana_vrf::program::OraoVrf;
use orao_solana_vrf::state::NetworkState;
use orao_solana_vrf::CONFIG_ACCOUNT_SEED;
use orao_solana_vrf::RANDOMNESS_ACCOUNT_SEED;

#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Processing,
    Finished
}

impl<> Default for Status {
    fn default() -> Self {
        return Status::Waiting
    } 
}

#[account]
pub struct Coinflip {

    pub user_1: Pubkey,
    pub user_2: Pubkey,
    pub amount: u64,
        pub force: [u8; 32],

    pub winner: Pubkey,
    pub status: Status
}


#[derive(Accounts,)]
#[instruction( room_id: String,amount: u64)]
pub struct CreateCoinflip<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
            init,
            space = 8 + std::mem::size_of::<Coinflip>(),

            payer = user,
            seeds = [b"coinflip", room_id.as_bytes().as_ref()],
            bump
    )]
    pub coinflip: Account<'info, Coinflip>,


    pub system_program: Program<'info, System>,
}


#[derive(Accounts,)]
#[instruction(room_id: String)]
pub struct JoinRoomCoinflip<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
            mut,
            seeds = [b"coinflip", room_id.as_bytes().as_ref()],
        
            bump
    )]
    pub coinflip: Account<'info, Coinflip>,



    #[account(address = SYSTEM_PROGRAM_ID)]
    pub system_program: Program<'info, System>,
}



#[derive(Accounts)]
#[instruction(room_id: String, force: [u8; 32])]
pub struct PlayCoinflip<'info> {
    #[account(mut)]
    pub user: Signer<'info>,


    #[account(
        mut, 
        seeds = [b"coinflip", room_id.as_bytes().as_ref()],
        constraint =
        coinflip.user_1 == user.to_account_info().key(),
        bump
    )] 
    pub coinflip: Account<'info, Coinflip>,



    /// CHECK: Treasury
    #[account(mut)]
    pub treasury: AccountInfo<'info>,


    /// CHECK: Randomness
    #[account(
        mut,
        seeds = [RANDOMNESS_ACCOUNT_SEED.as_ref(), &force],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub random: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [CONFIG_ACCOUNT_SEED.as_ref()],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub config: Account<'info, NetworkState>,

    pub vrf: Program<'info, OraoVrf>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(room_id: String, force: [u8; 32])]
pub struct ResultCoinflip<'info> {
    #[account(mut)]
    pub user_1: AccountInfo<'info>,

    #[account(mut)]
    pub user_2: AccountInfo<'info>,


    #[account(
        mut, 
        seeds = [b"coinflip", room_id.as_bytes().as_ref()],
        constraint =
        coinflip.status == Status::Processing &&
        coinflip.user_1 == user_1.key() &&
        coinflip.user_2 == user_2.key(),

        bump
    )] 
    pub coinflip: Account<'info, Coinflip>,



    /// CHECK: Treasury
    #[account(mut)]
    pub treasury: AccountInfo<'info>,


    /// CHECK: Randomness
    #[account(
        mut,
        seeds = [RANDOMNESS_ACCOUNT_SEED.as_ref(), &force],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub random: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [CONFIG_ACCOUNT_SEED.as_ref()],
        bump,
        seeds::program = orao_solana_vrf::ID
    )]
    pub config: Account<'info, NetworkState>,

    pub vrf: Program<'info, OraoVrf>,
    
    pub system_program: Program<'info, System>,
}