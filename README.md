# Creating a Solana Coinflip game with Orao Network's VRF 

Writing a game such as Coinflip in Solana may seem a bit of a taunting task given that you can not simply generate a random number based off the clock, or a blockhash. These are typically used for examples but those can be manipulated by a bad actor and as such are not secure. To build a secure game, any kind of game that relies on a random result in Solana must use an oracle. 

The main oracle providers in Solana today are Switchboard and Orao Network, I have used both but today I find Orao to be a bit more easy to use.


We'll start by initiating the Anchor project in our terminal: 

```anchor init solana-coinflip-game```

We'll first work on the contract itself and the PDA, then we'll slowly add VRF to it so it's fully secure and that it generates a verifiably random result. 

We need to add a few Cargo crates first, solana-program, anchor_spl and orao-network-vrf

```rust
[dependencies]
anchor-lang = "0.29.0"
orao-solana-vrf = {version="=0.3.0",default-features = false, features = ["cpi"]}
anchor-spl = "=0.29.0"
solana-program = "=1.18.14"
```
At the time of writing this postI am using ```anchor 0.29.0``` and this is how your [dependencies] should look in your Cargo.toml under programs/solana-coinflip-game, of course at the time of writing this is how it looks but depending on when you're reading it may be different. 

We'll create a new file named `pda.rs`, this file will store the pda details such as ***user_1*** and ***user_2***, ***winner*** and ***status*** of the game.

```rust

use anchor_lang::prelude::*;
use solana_program::{
    system_program::ID as SYSTEM_PROGRAM_ID,
    };
use anchor_spl::token::{self,
        ID as TOKEN_PROGRAM_ID};
    

#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Processing,
    Finished
}


#[account]
#[derive(Default)]
pub struct Coinflip {

    user_1: Pubkey,
    user_2: Pubkey,
    amount: u64,
    winner: Pubkey,
    status: Status
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
```

We have an enum Status for the game, Waiting meaning game is waiting for the 2nd player to join the room, Processing meaning that game has already started and that randomness is currently being generated. 

```CreateCoinflip``` will initiate the game and the PDA account, it'll place the bet and set user_1 and amount variable in the PDA, than in the next method we write we'll utilize these variables and require that amount when another user joins the room. 

Let us now write the create_coinflip function in `lib.rs`

```rust

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
```

Here we introduce an error ```InvalidAmount``` if a user tries to place a bet less than 0.05 SOL or 50000000 lamports then the error is thrown. 

```rust
#[error_code]
pub enum InvalidAmount {
    #[msg("Amount must be greater than 0.05 SOL")]
    InvalidAmount
}
```
The ```create_coinflip``` function will also transfer the amount inputted from the user to the coinflip account, this is essentially an escrow account that holds the funds while the bet is being processed, when it is we'll have the funds sent to the winner. 

Next function is ```joinroom_coinflip```, this'll mirror the create_coinflip function, it simply places the 2nd bet which means now we can write the final functions which will send for a randomness and process the result.

```rust

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
    #[account(address = TOKEN_PROGRAM_ID)]
    pub token_program: Program<'info, Token>,
}
```

You'll notice the PDA for this function is only different in that the Coinflip account no longer has init and space in its definition, now we can just edit it as it's already initialized and exists. 

```rust
// src/lib.rs

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
```

We'll start integrating Orao Network's verifiable randomness now, the next function will start the game and request a randomness from the oracle. 

The next method is `PlayCoinflip`, we'll add this in `pda.rs`

```rust
// pda.rs
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
```

We'll also start writing the tests now after we implement the function in ```lib.rs``` under ```play_coinflip```

```rust
// lib.rs

 pub fn play_coinflip(ctx: Context<PlayCoinflip>,room_id: String, force: [u8; 32]) -> Result<()> {
        let player = &ctx.accounts.user;
        let room = &mut ctx.accounts.coinflip;



        msg!("Coinflip in room {} game started", room_id);

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
```

You'll find the example on their site is not like this, it's much more complicated. Though it's actually far more simple if you take it apart and make it your own as I have here. 

With this function the game now starts, and the randomness is being processed by the oracle. Now we'll write some tests and then we'll conclude it with a final function which will use the randomness result and transfer the funds to the winner. 

When we initiated the project with Anchor, the command already generated some example tests for us so we'll start with the first function which initiates the game. We'll set the network variable to be Devnet in ```Anchor.toml``` 

```rust
[provider]
cluster = "Devnet"
```
Now we'll `anchor build` and `anchor deploy` and when we deploy we'll get something like this ![alt](https://photos.collectednotes.com/photos/29005/fcfb38ce-a6a1-4dc7-9594-02f9d36b3027?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T112058Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=c8cc4c8f432dba6ccabad7afe268d4e0be6df6580d5355e94940a3b998903f84) 

The program ID is different than what the example generates, so we'll need to replace that, in my case I'll replace 

```rust
declare_id("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
```
with the new program ID that we got by deploying it on devnet

```rust
declare_id("64CRrSCxSoEUDv2Sg3fKrwxotoiyD1bfce1AyCeuF582");
```
We can add this statement to `Anchor.toml` as well 

```rust
[programs.devnet]
solana_coinflip_game = "64CRrSCxSoEUDv2Sg3fKrwxotoiyD1bfce1AyCeuF582"
```

We'll need to build and deploy again otherwise it'll fail as it uses the old program id and we'd get `Error: AnchorError occurred. Error Code: DeclaredProgramIdMismatch. Error Number: 4100. Error Message: The declared program id does not match the actual program id.`, you can make it use the original program id Anchor generates when it first deploys, but I didn't do that here, so just redeploy and we can start testing.


You should also install @solana/web3.js for this part, using npm, let's write the test now: 

```rust
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { SolanaCoinflipGame } from "../target/types/solana_coinflip_game";
import { LAMPORTS_PER_SOL, PublicKey, SystemProgram } from "@solana/web3.js";
import { BN } from "bn.js";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

function randomString(length=8) {
  let result = '';
  const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  const charactersLength = characters.length;
  let counter = 0;
  while (counter < length) {
    result += characters.charAt(Math.floor(Math.random() * charactersLength));
    counter += 1;
  }
  return result;
}
describe("solana-coinflip-game", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SolanaCoinflipGame as Program<SolanaCoinflipGame>;
  const payer = anchor.Wallet.local().payer
  const keypair = Keypair.generate()

  const room_id = randomString()
  const amount = LAMPORTS_PER_SOL * 0.1
  const [coinflip] = PublicKey.findProgramAddressSync(
    [Buffer.from("coinflip"), Buffer.from(room_id)],
    program.programId
  );

  it("Is initialized!", async () => {
    // Add your test here.


  
    const tx = await program.methods.createCoinflip(room_id,new BN(amount)).accounts({
      coinflip,
      user: payer.publicKey,
      systemProgram: SystemProgram.programId,

    }).signers([
      payer
    ]).rpc({
      skipPreflight: true
    });
    console.log("Your transaction signature", tx);
    console.log("Program account data: ", await program.account.coinflip.fetch(coinflip))
  });
});
```

We'll find the PDA account with two strings, coinflip which is the constant and room_id being random, these two are seeds which are used to find the Coinflip account. 

The amount we bet is 0.1 SOL, and then we call the `createCoinflip` function, after that the transaction is sent and you can see the account data, immediately `amount` and `user_1` are defined, you'll also find that the Coinflip account now has 0.1 SOL which we just transferred by calling this function.



![alt](https://photos.collectednotes.com/photos/29005/7070b4f2-890b-439e-9c7b-fd15f5805d83?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T132642Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=22c8d8a27d094345d08e1f0398c9aee53bfcb269e7d9146aefa07f1075c12d05)

Now let's add our second player so we can execute `joinCoinflip` function, which will simply be a randomly generated keypair, we'll transfer some SOL to that address so we can place a bet. 

```ts 
it("Transfer SOL to player2", async () => {
   
    const transferTransaction = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: keypair.publicKey,
        lamports: LAMPORTS_PER_SOL*0.11,
      })
    )

    var tx =await sendAndConfirmTransaction(anchor.getProvider().connection, transferTransaction, [payer]);
    console.log("TX executed", tx)
})
```

This'll transfer 0.11 SOL to the 2nd player, the 2nd player being the keypair variable we defined just below the _program_ variable, a random keypair. This is the result after running `anchor test` for the 2nd time, you can run tests without deploying with `anchor test --skip-deploy --skip-build`.

![alt](https://photos.collectednotes.com/photos/29005/745f3f79-2fc6-42bd-95d1-d4e52d7b2d4a?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T133827Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=0e46b80d721d55886e72f192d4c5f680b578d18c91858a0a8f594a66c3eefb49)

Next test we'll add is to join the room, the test for initiating game is not much different to this, they are both essentially the same, except the user in this case becomes `keypair.publicKey`, that's our 2nd player.

```ts
it("Join game", async () => {
      const tx = await program.methods.joinCoinflip(room_id).accounts({
      coinflip,
      user: keypair.publicKey,
      systemProgram: SystemProgram.programId,

    }).signers([
      keypair
    ]).rpc({
      skipPreflight: true
    });

    console.log("Your transaction signature", tx);
    console.log("Program account data: ", await program.account.coinflip.fetch(coinflip))
  });
```

And this is the result, `keypair.publicKey` becomes _user2_ and now we can proceed to writing the last two functions and getting verifiable randomness.
![alt](https://photos.collectednotes.com/photos/29005/0f024ef6-0404-4629-8b01-931ad90e770e?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T134858Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=43098e8864546c1483981a63506a5e2ea5c168d3a86eb0e9c9e86a55e221bc2c)

Now let us proceed to testing ```play_coinflip``` function we created previously, for this you must install _@orao-network/solana-vrf_ with npm. Let's start with initiating the next test.

```ts
it("Play the game", async () => {
    const random = randomnessAccountAddress(force.toBuffer());
    const treasury = new PublicKey("9ZTHWWZDpB36UFe1vszf2KEpt83vwi27jDqtHQ7NSXyR");
})
```

```ts
const vrf = new Orao(anchor.getProvider() as any);
let force = Keypair.generate().publicKey;
```
We'll add these two variables on top so we may use them in other tests not just the one where we request a randomness. You can also simply define _treasury_ at the top. 

Force is a kind of a seed we pass to Orao, it's just a random public key converted to a buffer. In this devnet testing example the Orao treasury is `9ZTHWWZDpB36UFe1vszf2KEpt83vwi27jDqtHQ7NSXyR`.

Let's call the function now so we can wrap this up

```ts
const tx = await program.methods.playCoinflip( room_id, [...force.toBuffer()]).accounts({
      user: payer.publicKey,
      coinflip: coinflip,
      vrf: vrf.programId,
      config: networkStateAccountAddress(),
      treasury: treasury,
      random,
    }).signers([payer]).rpc();

    const tx = await program.methods.playCoinflip( room_id, [...force.toBuffer()]).accounts({
      user: payer.publicKey,
      coinflip: coinflip,
      vrf: vrf.programId,
      config: networkStateAccountAddress(),
      treasury: treasury,
      random,
    }).signers([payer]).rpc();

console.log(`Game has started, randomness is requested: `, tx)
```

As you may see everything went as planned here, we called the function successfully and the tx went through. That means that randomness has been requested, typically it takes less than 10seconds for it to resolve. We have also set the status to processing, this'll play into our last function

![alt](https://photos.collectednotes.com/photos/29005/66bf8d08-b23e-4a26-ae1e-31290b8c5239?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T135950Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=6ea86894d8eb55e8495b37e1ffc8ec927969fa4ff6bd39a15ec58c9c3de7ea39)

How do we know when it's ready? Simple, Orao has a function `waitFulfilled`, we pass the force variable to it which we used when we requested the randomness and then it'll resolve when it's fulfilled. 

```ts
it("Randomness fulfilled", async () => {
    let randomnessFulfilled = await vrf.waitFulfilled(force.toBuffer())
    console.log("Randomness is fulfilled, we can call the result function")
  })
```
Okay, now we can move forward with our last function which will get the winner out of the two, both players have a 50/50 chance of winning. First let's create a new file, _misc.rs_:

```rust
// misc.rs
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
```

And for the last part, we'll deal with the result and deciding who's the winner, let's create a new struct inside _pda.rs_

```rust
// pda.rs

#[derive(Accounts)]
#[instruction(room_id: String, force: [u8; 32])]
pub struct ResultCoinflip<'info> {
    #[account(
        mut, 
        seeds = [b"coinflip", room_id.as_bytes().as_ref()],
        constraint =
        coinflip.status == Status::Processing,
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
```
What is different about this compared to the `play_coinflip` function? Not much, really here we're just getting the result and deciding the winner, we also have a constraint that checks if the game has started, if not then the contract will throw an error.

```rust
// misc.rs

pub fn result_coinflip(ctx: Context<ResultCoinflip>,room_id: String, force: [u8; 32]) -> Result<()> {

        let rand_acc = crate::misc::get_account_data(&ctx.accounts.random)?;

        let randomness = current_state(&rand_acc);
        if (randomness == 0) {
            return err!(StillProcessing::StillProcessing)
        }
        let result = randomness % 2;

    }
```

We are also introducing a new function `current_state`, this'll get the VRF result, convert it to a number, and then % 2, that will end up in a result that is either 0 or 1. You can add this function inside _misc.rs_, as well as a new error for when randomness is still not fulfilled.

```rust
// misc.rs
pub fn current_state(randomness: &Randomness) ->u64 {
    if let Some(randomness) = randomness.fulfilled() {
        let value = randomness[0..size_of::<u64>()].try_into().unwrap();
        
        return u64::from_le_bytes(value);
    } else {
        return 0;

    }
}
```

``` rust
// lib.rs
#[error_code]
pub enum StillProcessing {
    #[msg("Randomness is still being fulfilled")]
    StillProcessing
}
```
Since we want to transfer the funds to the winner, we have to add both of the accounts to the struct, and also a constraint to make sure that this can't be abused by a third party that wants to fool the contract. 

We'll add `user_1` and `user_2` to the struct and then create a few constraints to prevent bad actors

```rust
// pda.rs

#[account(mut)]
pub user_1: AccountInfo<'info>,
   
#[account(mut)]
pub user_2: AccountInfo<'info>,

#[account(
        mut, 
        seeds = [b"coinflip", room_id.as_bytes().as_ref()],
        constraint =
        (coinflip.status == Status::Processing,
        coinflip.user_1 == user_1.key(),
        coinflip.user_2 == user_2.key()),

        bump
)] 
```

When we have done this let's finish our `result_coinflip` function 

```rust
// lib.rs @ result_coinflip 

msg!("VRF result is: {}", randomness);
if (result ==0) {
      coinflip.winner = coinflip.user_1.key();
            **ctx.accounts.user_1.lamports.borrow_mut() = ctx.accounts.user_1.lamports()
            .checked_add(coinflip.amount.clone())
            .unwrap();
             **coinflip.to_account_info().lamports.borrow_mut() -= coinflip.amount.clone();

             msg!("Winner is user_1: {}", coinflip.user_1.key().to_string())

} else {
      coinflip.winner = coinflip.user_2.key();
            **ctx.accounts.user_2.lamports.borrow_mut() = ctx.accounts.user_2.lamports()
            .checked_add(coinflip.amount.clone())
            .unwrap();
             **coinflip.to_account_info().lamports.borrow_mut() -= coinflip.amount.clone();
       msg!("Winner is user_2: {}", coinflip.user_2.key().to_string())


}

```

We already have the result with the result variable being `randomness % 2`, what these statements do is transfer the funds to the winner as well as marking them as the winner, and for the last trick we'll mark the game as finished, so the function should now look like this

```rust
// lib.rs

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
            .checked_add(coinflip.amount.clone() *2)
            .unwrap();
             **coinflip.to_account_info().lamports.borrow_mut() -= coinflip.amount.clone() * 2;
             msg!("Winner is user_2: {}", coinflip.user_2.key().to_string())


        }
        msg!("Coinflip game in room {} has concluded, the winner is {}", room_id, coinflip.winner.to_string());
        coinflip.status = Status::Finished;



        return Ok(())
}
```
Let's write the final test and we can see just how well everything runs

```ts
it("Get the result", async () => {   const vrf = new Orao(anchor.getProvider() as any);

    const random = randomnessAccountAddress(force.toBuffer());
    const treasury = new PublicKey("9ZTHWWZDpB36UFe1vszf2KEpt83vwi27jDqtHQ7NSXyR");

    const tx = await program.methods.resultCoinflip( room_id, [...force.toBuffer()]).accounts({
      user1: payer.publicKey,
      user2: keypair.publicKey,
      coinflip: coinflip,
      vrf: vrf.programId,
      config: networkStateAccountAddress(),
      treasury: treasury,
      random,
    }).signers([payer]).rpc();

  
    console.log(`Game is finished`, tx)
    console.log("Program account data: ", await program.account.coinflip.fetch(coinflip))

  })
```
And what do we get? 

![alt](https://photos.collectednotes.com/photos/29005/98d90ae8-3175-4f60-9f77-344ffd655706?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T144712Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=30dc15a6bcf535a5b3964bbfe815283460ee739bdbbe657175e264ab1af5eaa0)

This is how the account data should look after we get the result, the winner is decided, in this case the random user won over me, and status is set to finished. And this marks it, no insecure blockhashes or clocks, merely using the Orao Network's oracle to have a proper Coinflip game that can't be fooled by anyone. 

![alt](https://photos.collectednotes.com/photos/29005/f9be175b-c0a5-4972-ab74-f049e0e4b521?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T144926Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=0e069d7c087caac2e60599f64b93ea6e68a242f4b0854cde3e26ae171fc562b8)

The result is 12037561925398644525, hence 12037561925398644525 % 2 = 0, the winner is the second user .

![alt](https://photos.collectednotes.com/photos/29005/c6efdf89-5b57-45a2-8d98-279936dbc8a2?x-amz-acl=public-read&X-Amz-Expires=3600&X-Amz-Date=20240526T145457Z&X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA5W2WJOHUP7IHVD6I%2F20240526%2Fus-west-2%2Fs3%2Faws4_request&X-Amz-SignedHeaders=host&X-Amz-Signature=5bec2cdf6a5361c96b84c6558a4c4f0b0a2fbd235819d911c306046703fa3bff)

You can also see how the funds get transferred from the escrow coinflip account to the address of player ```user_2```, the winner gets all, so he gets 0.2 SOL.  

And that's it, a bit of a longer reader but as simple as it seems dealing with smart contracts and oracles takes some time and this post reflects. 

But as usual, I have my code on Github and you can feel free to take a stab it, just make sure to read what I have to say about the deployment of the contract in the beginning otherwise you might run into some difficulties. 

