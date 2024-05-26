use anchor_lang::prelude::*;
mod pda;
mod misc;
use crate::pda::*;


declare_id!("64CRrSCxSoEUDv2Sg3fKrwxotoiyD1bfce1AyCeuF582");




#[program]
pub mod solana_coinflip_game {

    use solana_program::{program::invoke, system_instruction::transfer};

    use self::misc::current_state;

    use super::*;

    pub fn create_coinflip(ctx: Context<CreateCoinflip>, room_id: String, amount: u64) -> Result<()> {

        if (amount < 50000000) {
            return err!(InvalidAmount::InvalidAmount);
        }
        
        let coinflip = &mut ctx.accounts.coinflip;

        invoke(
            &transfer(
                ctx.accounts.user.to_account_info().key,
                coinflip.clone().to_account_info().key,
                amount,
            ),
            &[
                ctx.accounts.user.to_account_info(),
                coinflip.clone().to_account_info(),

                ctx.accounts.system_program.to_account_info(),
            ],
        );
        coinflip.user_1 = ctx.accounts.user.clone().to_account_info().key();
        coinflip.amount = amount;

        msg!("Coinflip game is initiated");

        Ok(())
    }

    pub fn join_coinflip(ctx: Context<JoinRoomCoinflip>, room_id: String) -> Result<()> {
        let coinflip = &mut ctx.accounts.coinflip;

        invoke(
            &transfer(
                ctx.accounts.user.to_account_info().key,
                coinflip.clone().to_account_info().key,
                coinflip.amount.clone(),
            ),
            &[
                ctx.accounts.user.to_account_info(),
                coinflip.clone().to_account_info(),

                ctx.accounts.system_program.to_account_info(),
            ],
        );
        coinflip.user_2 = ctx.accounts.user.clone().to_account_info().key();
        coinflip.amount =   coinflip.amount.clone();

        msg!("Coinflip game can start, user 2 has entered the game");

        Ok(())
    }

    pub fn play_coinflip(ctx: Context<PlayCoinflip>,room_id: String, force: [u8; 32]) -> Result<()> {
        let player = &ctx.accounts.user;
        let room = &mut ctx.accounts.coinflip;



        msg!("Room {} game started", room_id);

        let cpi_program = ctx.accounts.vrf.to_account_info();
        let cpi_accounts = orao_solana_vrf::cpi::accounts::Request {
            payer: ctx.accounts.user.to_account_info(),
            network_state: ctx.accounts.config.to_account_info(),
            treasury: ctx.accounts.treasury.to_account_info(),
            request: ctx.accounts.random.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = anchor_lang::context::CpiContext::new(cpi_program, cpi_accounts);
        orao_solana_vrf::cpi::request(cpi_ctx, force)?;

        room.force = force;
        room.status = Status::Processing;
        msg!("Started game in room {}", room_id);
        return Ok(());
    
    }

    pub fn result_coinflip(ctx: Context<ResultCoinflip>,room_id: String, force: [u8; 32]) -> Result<()> {
        let coinflip = &mut ctx.accounts.coinflip;
        let rand_acc = crate::misc::get_account_data(&ctx.accounts.random)?;

        let randomness = current_state(&rand_acc);
        if (randomness == 0) {
            return err!(StillProcessing::StillProcessing)
        }
        let result = randomness % 2;

        msg!("VRF result is: {}", randomness);
        if (result ==0) {
            coinflip.winner = coinflip.user_1.key();
            **ctx.accounts.user_1.lamports.borrow_mut() = ctx.accounts.user_1.lamports()
            .checked_add(coinflip.amount.clone() * 2)
            .unwrap();
             **coinflip.to_account_info().lamports.borrow_mut() -= coinflip.amount.clone() * 2;

             msg!("Winner is user_1: {}", coinflip.user_1.key().to_string())

        } else {
            coinflip.winner = coinflip.user_2.key();
            **ctx.accounts.user_2.lamports.borrow_mut() = ctx.accounts.user_2.lamports()
            .checked_add(coinflip.amount.clone() * 2)
            .unwrap();
             **coinflip.to_account_info().lamports.borrow_mut() -= coinflip.amount.clone() * 2;
             msg!("Winner is user_2: {}", coinflip.user_2.key().to_string())


        }
        msg!("Coinflip game in room {} has concluded, the winner is {}", room_id, coinflip.winner.to_string());
        coinflip.status = Status::Finished;



        return Ok(())
    }


}


#[error_code]
pub enum StillProcessing {
    #[msg("Randomness is still being fulfilled")]
    StillProcessing
}

#[error_code]
pub enum InvalidAmount {
    #[msg("Amount must be greater than 0.05 SOL")]
    InvalidAmount
}