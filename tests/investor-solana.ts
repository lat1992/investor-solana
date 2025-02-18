import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { InvestorSolana } from "../target/types/investor_solana";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("investor-solana", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.InvestorSolana as Program<InvestorSolana>;

  it("should create a session", async () => {
    const sessionAccount = Keypair.generate();
    const totalDeposit = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL);
    const basic_deposit_amount = new anchor.BN(1e9);

    await program.methods
      .createSession(basic_deposit_amount)
      .accountsStrict({
        session: sessionAccount.publicKey,
        signer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: sessionAccount.publicKey,
          lamports: totalDeposit.toNumber(),
        }),
      ])
      .signers([sessionAccount])
      .rpc();

    const sessionData = await program.account.session.fetch(
      sessionAccount.publicKey
    );

    assert.equal(sessionData.isActive, true);
    assert.ok(sessionData.totalDeposit.eq(totalDeposit));
  });

  it("should close a session", async () => {
    const sessionAccount = Keypair.generate();
    const token = Keypair.generate();
    const totalDeposit = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL);
    const basic_deposit_amount = new anchor.BN(1e9);

    await program.methods
      .createSession(basic_deposit_amount)
      .accountsStrict({
        session: sessionAccount.publicKey,
        signer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([
        SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: sessionAccount.publicKey,
          lamports: totalDeposit.toNumber(),
        }),
      ])
      .signers([sessionAccount])
      .rpc();

    const sessionDataBeforeClose = await program.account.session.fetch(
      sessionAccount.publicKey
    );
    assert.equal(sessionDataBeforeClose.isActive, true);

    await program.methods
      .closeSession(token.publicKey)
      .accountsStrict({
        session: sessionAccount.publicKey,
        signer: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const sessionDataAfterClose = await program.account.session.fetch(
      sessionAccount.publicKey
    );

    assert.ok(sessionDataAfterClose.winnerToken.equals(token.publicKey));
    assert.equal(sessionDataAfterClose.isActive, false);
  });
});
