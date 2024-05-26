import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { SolanaCoinflipGame } from "../target/types/solana_coinflip_game";
import { Keypair, LAMPORTS_PER_SOL, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from "@solana/web3.js";
import { BN } from "bn.js";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";
import { networkStateAccountAddress, Orao, randomnessAccountAddress } from "@orao-network/solana-vrf";

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

  const room_id = randomString()
  const amount = LAMPORTS_PER_SOL * 0.1
  const [coinflip] = PublicKey.findProgramAddressSync(
    [Buffer.from("coinflip"), Buffer.from(room_id)],
    program.programId
  );
  const keypair = Keypair.generate();

  const vrf = new Orao(anchor.getProvider() as any);

    
  let force = Keypair.generate().publicKey;

  it("Initiate game", async () => {
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

  it("Play the game", async () => {   const vrf = new Orao(anchor.getProvider() as any);

    const random = randomnessAccountAddress(force.toBuffer());
    const treasury = new PublicKey("9ZTHWWZDpB36UFe1vszf2KEpt83vwi27jDqtHQ7NSXyR");

    const tx = await program.methods.playCoinflip( room_id, [...force.toBuffer()]).accounts({
      user: payer.publicKey,
      coinflip: coinflip,
      vrf: vrf.programId,
      config: networkStateAccountAddress(),
      treasury: treasury,
      random,
    }).signers([payer]).rpc();

  
    console.log(`Game has started, randomness is requested: `, tx)
    console.log("Program account data: ", await program.account.coinflip.fetch(coinflip))

  })

  it("Randomness fulfilled", async () => {
    let randomnessFulfilled = await vrf.waitFulfilled(force.toBuffer())
    console.log("Randomness is fulfilled, we can call the result function")
  })

  
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
});
