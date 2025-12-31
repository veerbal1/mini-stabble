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

  describe("weighted pool", () => {
    it("initializes weighted pool", async () => {});
    it("deposits liquidity", async () => {});
    it("swaps tokens", async () => {});
  });

  describe("Stable Pool", () => {
    it("initializes stable pool", async () => {});
    it("deposits liquidity", async () => {});
    it("swaps tokens", async () => {});
  });
});
