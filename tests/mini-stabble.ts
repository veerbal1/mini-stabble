import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MiniStabble } from "../target/types/mini_stabble";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";
import BN from "bn.js";

describe("mini-stabble", () => {
  const AUTHORITY_SEED = Buffer.from("AUTHORITY");
  const MINT_SEED = Buffer.from("MINT");
  const WEIGHT_POOL_SEED = Buffer.from("WEIGHT_POOL");
  const STABLE_POOL_SEED = Buffer.from("STABLE_POOL");
  const POOL_VAULT_SEED = Buffer.from("POOL_VAULT");

  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  const payer = provider.wallet.payer;
  const program = anchor.workspace.miniStabble as Program<MiniStabble>;

  // Mints (will create in beforeEach)
  let mintA: PublicKey;
  let mintB: PublicKey;
  let lpMint: Keypair;

  // PDAs
  let authority: PublicKey;
  let authorityBump: number;

  // User Token accounts
  let userTokenA: PublicKey;
  let userTokenB: PublicKey;

  before(async () => {
    [authority, authorityBump] = PublicKey.findProgramAddressSync(
      [AUTHORITY_SEED],
      program.programId
    );

    let tempMintA = await createMint(
      provider.connection,
      payer,
      provider.wallet.publicKey,
      null,
      9
    );

    let tempMintB = await createMint(
      provider.connection,
      payer,
      provider.wallet.publicKey,
      null,
      9
    );

    if (tempMintA.toBase58() < tempMintB.toBase58()) {
      mintA = tempMintA;
      mintB = tempMintB;
    } else {
      mintA = tempMintB;
      mintB = tempMintA;
    }

    lpMint = Keypair.generate();

    userTokenA = await createAssociatedTokenAccount(
      provider.connection,
      payer,
      mintA,
      payer.publicKey
    );

    userTokenB = await createAssociatedTokenAccount(
      provider.connection,
      payer,
      mintB,
      payer.publicKey
    );

    await mintTo(
      provider.connection,
      payer,
      mintA,
      userTokenA,
      payer,
      1_000_000_000_000
    );
    await mintTo(
      provider.connection,
      payer,
      mintB,
      userTokenB,
      payer,
      1_000_000_000_000
    );
  });

  const getPoolPDA = () => {
    return PublicKey.findProgramAddressSync(
      [WEIGHT_POOL_SEED, lpMint.publicKey.toBuffer()],
      program.programId
    )[0];
  };

  const getVaultAPDA = (pool: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [POOL_VAULT_SEED, pool.toBuffer(), mintA.toBuffer()],
      program.programId
    )[0];
  };

  const getVaultBPDA = (pool: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [POOL_VAULT_SEED, pool.toBuffer(), mintB.toBuffer()],
      program.programId
    )[0];
  };

  describe("weighted pool", () => {
    it("initializes weighted pool", async () => {
      const pool = getPoolPDA();

      await program.methods
        .initializeWeightedPool(
          new BN(3_000_000), // swap_fee (0.3%)
          new BN(500000000) // weight_a (0.5 in SCALE = 5e17)
        )
        .accounts({
          lpMint: lpMint.publicKey,
          tokenMintA: mintA,
          tokenMintB: mintB,
          payer: payer.publicKey,
        })
        .signers([lpMint])
        .rpc();

      // Assert
      const poolAccount = await program.account.weightedPool.fetch(pool);
      expect(poolAccount.swapFee.toNumber()).to.equal(3_000_000);
      expect(poolAccount.tokens[0].weight.toNumber()).to.equal(500000000);
      expect(poolAccount.tokens[1].weight.toNumber()).to.equal(500000000);
      expect(poolAccount.lpMint.toBase58()).to.equal(lpMint.publicKey.toBase58());
      expect(poolAccount.tokens[0].mint.toBase58()).to.equal(mintA.toBase58());
      expect(poolAccount.tokens[1].mint.toBase58()).to.equal(mintB.toBase58());
      expect(poolAccount.tokens.length).to.equal(2);
      expect(poolAccount.bump).to.be.a("number");
      expect(poolAccount.isActive).to.be.true;
      expect(poolAccount.tokens[0].balance.toNumber()).to.equal(0);
      expect(poolAccount.tokens[1].balance.toNumber()).to.equal(0);
    });
    it("deposits liquidity", async () => {});
    it("swaps tokens", async () => {});
  });

  describe("Stable Pool", () => {
    it("initializes stable pool", async () => {});
    it("deposits liquidity", async () => {});
    it("swaps tokens", async () => {});
  });
});
