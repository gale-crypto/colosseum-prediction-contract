import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { ColosseumPrediction } from "../target/types/colosseum_prediction";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from "@solana/spl-token";

function prepareMarketIdSeed(marketId: string): Buffer {
  const marketIdBytes = Buffer.from(marketId, 'utf8')
  const maxLength = Math.min(32, marketIdBytes.length)
  return marketIdBytes.slice(0, maxLength)
}

describe("km_raffle_contract", async () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.colosseumPrediction as Program<ColosseumPrediction>;
  const keypair = provider.wallet.payer;  

  console.log("Program: ", program.programId.toBase58());

  // const feeRecipient = new PublicKey("6yRZk5bb5nedXSwvpHERNVzePCsVQ4t3isPLEd7e4qRN");
  // const usdtMint = new PublicKey("2mfQgc4tf8vzcBeMKzEYMvWwgA3zt2Zf5v2QCeyaCtT7");
  // const usdcMint = new PublicKey("BRYjq2hyLJsTEZfxmDZMjrpFDvptNSRyaqgyQD9HmQ7Z");
  // const kmMint = new PublicKey("DqHczfUDH6d83aSZ9eez1TrJW3sGzBpmU9HyVyjrmGFv");

  const feeRecipient = new PublicKey("8z7Sx2LykUfHtKXU6xUe3ZgZetwqR45cqpLxNnA1ektK");
  const usdtMint = new PublicKey("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
  const usdcMint = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  const kmMint = new PublicKey("FThrNpdic79XRV6i9aCWQ2UTp7oRQuCXAgUWtZR2cs42");  

  const secondAdmin = new PublicKey("53RTLbmTdqAmpLBsmmF9QVbWGsUdsJ1YFzZntXxRiUZn");

  // const wallet1 = new PublicKey("2GFD9nM9pmBVifcXiZtfGG124gg9ZskYFnqXCy5SGmJN");
  // const wallet2 = new PublicKey("56NECkZWVMwTTUxL2mTaBkhGPRkhfmA5PsrgwkvJThQF");
  // const wallet3 = new PublicKey("9dh3jDZGjnQfWzsSiinKna5zb5t9tLKQcfAiwqVRd4JV");

  const [adminConfigPDA, bump] = await PublicKey.findProgramAddress(
    [Buffer.from("admin_config")],
    program.programId
  );

  console.log("Admin Config PDA: ", adminConfigPDA.toBase58());

  const feeRecipientUsdtAccount = await getAssociatedTokenAddress(usdtMint, feeRecipient, true);
  const feeRecipientUsdcAccount = await getAssociatedTokenAddress(usdcMint, feeRecipient, true);
  console.log("Fee Recipient USDT Account: ", feeRecipientUsdtAccount.toBase58());
  console.log("Fee Recipient USDC Account: ", feeRecipientUsdcAccount.toBase58());

  const [strikeReservePDA, strikeReserveBump] = await PublicKey.findProgramAddress(
    [Buffer.from("strike_reserve")],
    program.programId
  );
  console.log("Strike Reserve PDA: ", strikeReservePDA.toBase58());

  const strikeReserveUsdtAccount = await getAssociatedTokenAddress(usdtMint, strikeReservePDA, true);
  const strikeReserveUsdcAccount = await getAssociatedTokenAddress(usdcMint, strikeReservePDA, true);
  console.log("Strike Reserve USDT Account: ", strikeReserveUsdtAccount.toBase58());
  console.log("Strike Reserve USDC Account: ", strikeReserveUsdcAccount.toBase58());

  // const marketId = 'a63f9248-50ae-4684-b332-9eda17beb8d9';
  // const [marketPDA, marketBump] = await PublicKey.findProgramAddress(
  //   [Buffer.from("market"), Buffer.from(marketId.slice(0, 32))],
  //   program.programId
  // );

  // const marketUsdtAccount = await getAssociatedTokenAddress(usdtMint, marketPDA, true);
  // const marketKMAccount = await getAssociatedTokenAddress(kmMint, marketPDA, true);
  // console.log('Market USDT Account: ', marketUsdtAccount.toBase58());
  // console.log('Market KM Account: ', marketKMAccount.toBase58());
  // const marketUsdtBalance = await provider.connection.getTokenAccountBalance(marketUsdtAccount).then(res => res.value.uiAmount);
  // console.log('Market USDT Balance: ', marketUsdtBalance);
  // const marketUsdcAccount = await getAssociatedTokenAddress(usdcMint, marketPDA, true);
  // console.log('Market USDC Account: ', marketUsdcAccount.toBase58());
  // const referrerUsdtAccount = await getAssociatedTokenAddress(usdtMint, wallet1);
  // const marketUsdcBalance = await provider.connection.getTokenAccountBalance(marketUsdcAccount).then(res => res.value.uiAmount);
  // console.log('Market USDC Balance: ', marketUsdcBalance);

  // const userUsdtAccount = await getAssociatedTokenAddress(usdtMint, provider.wallet.publicKey);
  // const userKMAccount = await getAssociatedTokenAddress(kmMint, provider.wallet.publicKey);

  // const marketIdSeed = prepareMarketIdSeed(marketId);
  
  // const marketData = await program.account.market.fetch(marketPDA);
  // console.log('Market Data:', marketData.yesPrice.toNumber(), marketData.noPrice.toNumber(), marketData.yesVolume.toNumber(), marketData.noVolume.toNumber());

  // const [positionPDA, positionBump] = await PublicKey.findProgramAddress(
  //   [Buffer.from("position"), provider.wallet.publicKey.toBuffer(), Buffer.from(marketIdSeed)],
  //   program.programId
  // );
  // console.log('Position PDA: ', positionPDA.toBase58());



  // const buyResult = await program.methods
  // .buyKmWithUsdt(new anchor.BN(3000000))
  // .accounts({
  //   // market: marketPDA,
  //   // position: positionPDA,
  //   // adminConfig: adminConfigPDA,

  //   // userTokenAccount: await getAssociatedTokenAddress(usdtMint, provider.wallet.publicKey),

  //   ammProgram: new PublicKey("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8"),
  //   amm: new PublicKey("CWtf2nCwCD1ctJ7tF5dbgpv8GcnDm4mhWiXmc5Ev9jZ5"),
  //   ammAuthority: new PublicKey("AhSQwzpJMskCCQ3GkWDZSA69xS5qK5s7jLejhK6sgYf1"),
  //   ammOpenOrders: new PublicKey("9gUuaRpvNTi2hY4jAFxztWgzzCSxyJmpAwPKNg1QYZDB"),
  //   ammCoinVault: new PublicKey("3JBhqvoyi8WPqaE2RfxwAjRV3Fy5QnKZbjpn49tHCrQv"),
  //   ammPcVault: new PublicKey("8fWnHdmzicpUhmWWcexV8VDuqtu4Nkwoj5gEabLkC7ri"),
  //   marketProgram: new PublicKey("EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj"),
  //   market: new PublicKey("3AwVe6kUMLv3TVg9A2io298UrAERHTpE2vTqP2goZuKB"),
  //   marketBids: new PublicKey("9rJMksC8Dgyu2tDvUVQBCFsesqKVWuUKafRSxSxoDECw"),
  //   marketAsks: new PublicKey("BTqrkB6MSN5dK17kbCb7mNFs7WTkQKsC4kVB43hS8VWo"),
  //   marketEventQueue: new PublicKey("EizA17Etg8KBJ1f2tjmuRLqN2EbcgSipmjRMNpL7DhZq"),
  //   marketCoinVault: new PublicKey("7FJ1TKcL9rievDEvCaU2vv4iBGBb8fAyjAJhpm29byun"),
  //   marketPcVault: new PublicKey("48FC4KJrfZRSGCa9wbQGtECaqbqrCrQtBfMA3YctEmqY"),
  //   marketVaultSigner: new PublicKey("FKzm5NdGcW9SVYNi2MAkAAUspL2PeYH1pBh2JtUnhLWR"),

  //   userTokenSource: userUsdtAccount,
  //   userTokenDestination: userKMAccount,
  //   userSourceOwner: provider.wallet.publicKey,
  //   tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,

  //   // userUsdtVaultUnchecked: userUsdtAccount,
  //   // userKmVaultUnchecked: userKMAccount,
  //   // marketVault: marketUsdtAccount, 
  //   // kmUserVault: userKMAccount,
  //   // referrer: wallet1,
  //   // referrerUsdtAta: referrerUsdtAccount, 
  //   // feeRecipientTokenAccount: await getAssociatedTokenAddress(usdtMint, feeRecipient),
  //   // usdtMint: usdtMint,
  //   // kmMint: kmMint,
  //   // tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
  //   // rent: SYSVAR_RENT_PUBKEY,
  //   // systemProgram: anchor.web3.SystemProgram.programId,
  // })
  // .signers([keypair])
  // .simulate();

  // console.log('Buy Result:', buyResult)

  it("Is initialized", async () => {
    const tx = await program.methods
      .initializeAdminConfig(
        // feeRecipient
      )
      .accounts({
        authority: provider.wallet.publicKey,
        adminConfig: adminConfigPDA,
        feeRecipient: feeRecipient,
        feeRecipientUsdtAccount: feeRecipientUsdtAccount,
        feeRecipientUsdcAccount: feeRecipientUsdcAccount,
        strikeReserve: strikeReservePDA,
        strikeReserveUsdtAccount: strikeReserveUsdtAccount,
        strikeReserveUsdcAccount: strikeReserveUsdcAccount,
        usdtMint: usdtMint,
        usdcMint: usdcMint,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([keypair])
      .rpc();
    console.log("Your transaction signature", tx);
  })

  // it("Add Admin", async () => {
  //   const newAdmin = new PublicKey("8z7Sx2LykUfHtKXU6xUe3ZgZetwqR45cqpLxNnA1ektK");
  //   const tx = await program.methods
  //     .addAdmin(
  //       newAdmin
  //     )
  //     .accounts({
  //       authority: provider.wallet.publicKey,
  //       adminConfig: adminConfigPDA,
  //     })
  //     .signers([keypair])
  //     .rpc();
  //   console.log("Your transaction signature", tx);
  // })

  // it("Initialize Market", async () => {
  //   const marketId = '27c7a829-4660-48a0-9e19-fc23d2c2e681';
  //   const [marketPDA, marketBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("market"), Buffer.from(marketId.slice(0, 32))],
  //     program.programId
  //   );
  //   const initialYesPrice = new anchor.BN(50000);
  //   const initialNoPrice = new anchor.BN(50000);
  //   const creator = new PublicKey("Cits9FJaXscX2X2QRZbMvNZkfXgPx8jTytCgqbRkr5eN");

  //   console.log("Market PDA: ", marketPDA.toBase58());
  //   const result = await program.methods
  //   .initializeMarket(
  //     marketId,
  //     initialYesPrice,
  //     initialNoPrice,
  //   )
  //   .accounts({
  //     market: marketPDA,
  //     creator: creator,
  //     adminConfig: adminConfigPDA,
  //     feeRecipient: feeRecipient,
  //     systemProgram: anchor.web3.SystemProgram.programId,
  //   })
  //   .signers([keypair])
  //   .simulate();

  //   console.log('Simulation Result:', result)
  // })

  // it("initialize user position", async () => {
  //   const marketId = '061f5923-26a3-43b0-ade0-8005aa8ff8d7';
  //   const marketIdSeed = prepareMarketIdSeed(marketId);
  //   const [marketPDA, marketBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("market"), Buffer.from(marketIdSeed)],
  //     program.programId
  //   );
  //   console.log("Market PDA: ", marketPDA.toBase58());

  //   const result = await program.methods
  //   .initializePosition(
  //     marketId
  //   )
  //   .accounts({
  //     market: marketPDA,
  //     position: positionPDA,
  //     user: provider.wallet.publicKey,
  //     systemProgram: anchor.web3.SystemProgram.programId
  //   })
  //   .signers([keypair])
  //   .simulate();

  //   console.log('Simulation Result:', result)
  // });

  // it("Buy Yes", async () => {
  //   const marketId = '061f5923-26a3-43b0-ade0-8005aa8ff8d7';
  //   const marketIdSeed = prepareMarketIdSeed(marketId);
  //   const [marketPDA, marketBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("market"), Buffer.from(marketIdSeed)],
  //     program.programId
  //   );
  //   console.log("Market PDA: ", marketPDA.toBase58());

  //   const [positionPDA, positionBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("position"), provider.wallet.publicKey.toBuffer(), Buffer.from(marketIdSeed)],
  //     program.programId
  //   );
  //   console.log('Position PDA: ', positionPDA.toBase58());    

  //   const userUsdtAccount = await getAssociatedTokenAddress(usdtMint, provider.wallet.publicKey);
  //   console.log('User USDT Account: ', userUsdtAccount.toBase58());

  //   const [marketVault, marketVaultBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("market_vault"), Buffer.from(marketIdSeed), Buffer.from("usdt")],
  //     program.programId
  //   );
  //   console.log('Market Vault: ', marketVault.toBase58());

  //   const feeRecipientTokenAccount = await getAssociatedTokenAddress(usdtMint, feeRecipient);

  //   console.log('Fee Recipient USDT Account: ', feeRecipientTokenAccount.toBase58());

  //   const amount = new anchor.BN(3000000); // 3 USDT with 6 decimals

  //   const paymentTokenEnum = { usdt: {} }   

  //   const result = await program.methods
  //   .buyYes(
  //     amount,
  //     paymentTokenEnum
  //   )
  //   .accounts({
  //     market: marketPDA,
  //     position: positionPDA,
  //     user: provider.wallet.publicKey,
  //     userTokenAccount: userUsdtAccount,
  //     marketVault: marketVault,
  //     adminConfig: adminConfigPDA,
  //     feeRecipientTokenAccount: feeRecipientTokenAccount,
  //     usdtMint: usdtMint,
  //     usdcMint: usdcMint,
  //     marketVaultAuthority: marketVault,
  //     tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
  //     rent: SYSVAR_RENT_PUBKEY,
  //     systemProgram: anchor.web3.SystemProgram.programId,
  //   })
  //   .signers([keypair])
  //   .simulate();

  //   console.log('Simulation Result:', result)
  // })

  // it("Add Admin", async () => {
  //   const tx = await program.methods
  //   .addAdmin(
  //     secondAdmin
  //   )
  //   .accounts({
  //     authority: provider.wallet.publicKey,
  //     adminConfig: adminConfigPDA,
  //   })
  //   .signers([keypair])
  //   .rpc();

  //   console.log("Your transaction signature", tx);
  // });

});
