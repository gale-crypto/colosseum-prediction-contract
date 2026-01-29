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

  const feeRecipient = new PublicKey("6yRZk5bb5nedXSwvpHERNVzePCsVQ4t3isPLEd7e4qRN");
  const usdtMint = new PublicKey("2mfQgc4tf8vzcBeMKzEYMvWwgA3zt2Zf5v2QCeyaCtT7");
  const usdcMint = new PublicKey("BRYjq2hyLJsTEZfxmDZMjrpFDvptNSRyaqgyQD9HmQ7Z");

  const secondAdmin = new PublicKey("53RTLbmTdqAmpLBsmmF9QVbWGsUdsJ1YFzZntXxRiUZn");

  const wallet1 = new PublicKey("2GFD9nM9pmBVifcXiZtfGG124gg9ZskYFnqXCy5SGmJN");
  const wallet2 = new PublicKey("56NECkZWVMwTTUxL2mTaBkhGPRkhfmA5PsrgwkvJThQF");
  const wallet3 = new PublicKey("9dh3jDZGjnQfWzsSiinKna5zb5t9tLKQcfAiwqVRd4JV");

  const [adminConfigPDA, bump] = await PublicKey.findProgramAddress(
    [Buffer.from("admin_config")],
    program.programId
  );

  console.log("Admin Config PDA: ", adminConfigPDA.toBase58());

  const marketId = 'ae32380a-9551-42ef-9bdc-2498a8faf7af';
  const [marketPDA, marketBump] = await PublicKey.findProgramAddress(
    [Buffer.from("market"), Buffer.from(marketId.slice(0, 32))],
    program.programId
  );

  const marketUsdtAccount = await getAssociatedTokenAddress(usdtMint, marketPDA, true);
  console.log('Market USDT Account: ', marketUsdtAccount.toBase58());
  const marketUsdtBalance = await provider.connection.getTokenAccountBalance(marketUsdtAccount).then(res => res.value.uiAmount);
  console.log('Market USDT Balance: ', marketUsdtBalance);
  const marketUsdcAccount = await getAssociatedTokenAddress(usdcMint, marketPDA, true);
  console.log('Market USDC Account: ', marketUsdcAccount.toBase58());
  const marketUsdcBalance = await provider.connection.getTokenAccountBalance(marketUsdcAccount).then(res => res.value.uiAmount);
  console.log('Market USDC Balance: ', marketUsdcBalance);

  const marketIdSeed = prepareMarketIdSeed(marketId);
  
  const marketData = await program.account.market.fetch(marketPDA);
  console.log('Market Data:', marketData.yesPrice.toNumber(), marketData.noPrice.toNumber(), marketData.yesVolume.toNumber(), marketData.noVolume.toNumber());

  // const [position1PDA, positionBump] = await PublicKey.findProgramAddress(
  //   [Buffer.from("position"), wallet1.toBuffer(), Buffer.from(marketIdSeed)],
  //   program.programId
  // );

  // const position1Data = await program.account.position.fetch(position1PDA);
  // console.log('Position 1 Data:', position1Data.yesShares.toNumber() / 1e6, position1Data.noShares.toNumber() / 1e6);

  // const [position2PDA, position2Bump] = await PublicKey.findProgramAddress(
  //   [Buffer.from("position"), wallet2.toBuffer(), Buffer.from(marketIdSeed)],
  //   program.programId
  // );
  // const position2Data = await program.account.position.fetch(position2PDA);
  // console.log('Position 2 Data:', position2Data.yesShares.toNumber() / 1e6, position2Data.noShares.toNumber() / 1e6);

  // const [position3PDA, position3Bump] = await PublicKey.findProgramAddress(
  //   [Buffer.from("position"), wallet3.toBuffer(), Buffer.from(marketIdSeed)],
  //   program.programId
  // );
  // const position3Data = await program.account.position.fetch(position3PDA);
  // console.log('Position 3 Data:', position3Data.yesShares.toNumber() / 1e6, position3Data.noShares.toNumber() / 1e6);

  // console.log("Sum of Yes Shares:", (position1Data.yesShares.toNumber() + position2Data.yesShares.toNumber() + position3Data.yesShares.toNumber()) / 1e6);
  // console.log("Sum of No Shares:", (position1Data.noShares.toNumber() + position2Data.noShares.toNumber() + position3Data.noShares.toNumber()) / 1e6);

  // const result1 = await program.methods
  // .claimWinningsYesno()
  // .accounts({
  //   market: marketPDA,
  //   position: position1PDA,
  //   user: wallet1,
  //   adminConfig: adminConfigPDA,
  //   userUsdtTokenAccount: await getAssociatedTokenAddress(usdtMint, wallet1),
  //   userUsdcTokenAccount: await getAssociatedTokenAddress(usdcMint, wallet1),
  //   marketUsdtAccount: marketUsdtAccount,
  //   marketUsdcAccount: marketUsdcAccount,
  //   feeRecipientUsdtAccount: await getAssociatedTokenAddress(usdtMint, feeRecipient),
  //   feeRecipientUsdcAccount: await getAssociatedTokenAddress(usdcMint, feeRecipient),
  //   usdtMint: usdtMint,
  //   usdcMint: usdcMint,
  //   tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
  //   rent: SYSVAR_RENT_PUBKEY,
  //   systemProgram: anchor.web3.SystemProgram.programId,
  //   associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
  // })
  // .simulate();

  // console.log('Simulation Result:', result1.events[0].data.payoutBeforeFee.toNumber() / 1e6);

  // const result2 = await program.methods
  // .claimWinningsYesno()
  // .accounts({
  //   market: marketPDA,
  //   position: position2PDA,
  //   user: wallet2,
  //   adminConfig: adminConfigPDA,
  //   userUsdtTokenAccount: await getAssociatedTokenAddress(usdtMint, wallet2),
  //   userUsdcTokenAccount: await getAssociatedTokenAddress(usdcMint, wallet2),
  //   marketUsdtAccount: marketUsdtAccount,
  //   marketUsdcAccount: marketUsdcAccount,
  //   feeRecipientUsdtAccount: await getAssociatedTokenAddress(usdtMint, feeRecipient),
  //   feeRecipientUsdcAccount: await getAssociatedTokenAddress(usdcMint, feeRecipient),
  //   usdtMint: usdtMint,
  //   usdcMint: usdcMint,
  //   tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
  //   rent: SYSVAR_RENT_PUBKEY,
  //   systemProgram: anchor.web3.SystemProgram.programId,
  //   associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
  // })
  // .simulate();

  // console.log('Simulation Result:', result2.events[0].data.payoutBeforeFee.toNumber() / 1e6);  

  // const result3 = await program.methods
  // .claimWinningsYesno()
  // .accounts({
  //   market: marketPDA,
  //   position: position3PDA,
  //   user: wallet3,
  //   adminConfig: adminConfigPDA,
  //   userUsdtTokenAccount: await getAssociatedTokenAddress(usdtMint, wallet3),
  //   userUsdcTokenAccount: await getAssociatedTokenAddress(usdcMint, wallet3),
  //   marketUsdtAccount: marketUsdtAccount,
  //   marketUsdcAccount: marketUsdcAccount,
  //   feeRecipientUsdtAccount: await getAssociatedTokenAddress(usdtMint, feeRecipient),
  //   feeRecipientUsdcAccount: await getAssociatedTokenAddress(usdcMint, feeRecipient),
  //   usdtMint: usdtMint,
  //   usdcMint: usdcMint,
  //   tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
  //   rent: SYSVAR_RENT_PUBKEY,
  //   systemProgram: anchor.web3.SystemProgram.programId,
  //   associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
  // })
  // .simulate();

  // console.log('Simulation Result:', result3.events[0].data.payoutBeforeFee.toNumber() / 1e6);  

  // console.log('Sum of Simulation:', (result1.events[0].data.payoutBeforeFee.toNumber() + result2.events[0].data.payoutBeforeFee.toNumber() + result3.events[0].data.payoutBeforeFee.toNumber()) / 1e6);

  const test = await program.methods
  .simulateBuyBinary(true, new anchor.BN(3000000))
  .accounts({
    market: marketPDA,
    caller: provider.wallet.publicKey,
  })
  .simulate();

  console.log('Simulation Result:', test.events[0].data);

  // it("Is initialized", async () => {
  //   const tx = await program.methods
  //     .initializeAdminConfig(
  //       feeRecipient
  //     )
  //     .accounts({
  //       authority: provider.wallet.publicKey,
  //       adminConfig: adminConfigPDA,
  //       systemProgram: anchor.web3.SystemProgram.programId,
  //     })
  //     .signers([keypair])
  //     .rpc();
  //   console.log("Your transaction signature", tx);
  // })

  // it("Add Admin", async () => {
  //   const newAdmin = new PublicKey("Cits9FJaXscX2X2QRZbMvNZkfXgPx8jTytCgqbRkr5eN");
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

  //   const [positionPDA, positionBump] = await PublicKey.findProgramAddress(
  //     [Buffer.from("position"), provider.wallet.publicKey.toBuffer(), Buffer.from(marketIdSeed)],
  //     program.programId
  //   );
  //   console.log('Position PDA: ', positionPDA.toBase58());    

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
