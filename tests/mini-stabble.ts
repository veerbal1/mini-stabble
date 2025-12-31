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
  getMint,
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
  let stableLpMint: Keypair;

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

    stableLpMint = Keypair.generate();
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

  const getStablePoolPDA = () => {
    return PublicKey.findProgramAddressSync(
      [STABLE_POOL_SEED, stableLpMint.publicKey.toBuffer()],
      program.programId
    )[0];
  };

  const getStableVaultAPDA = (pool: PublicKey) => {
    return PublicKey.findProgramAddressSync(
      [POOL_VAULT_SEED, pool.toBuffer(), mintA.toBuffer()],
      program.programId
    )[0];
  };

  const getStableVaultBPDA = (pool: PublicKey) => {
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
      expect(poolAccount.lpMint.toBase58()).to.equal(
        lpMint.publicKey.toBase58()
      );
      expect(poolAccount.tokens[0].mint.toBase58()).to.equal(mintA.toBase58());
      expect(poolAccount.tokens[1].mint.toBase58()).to.equal(mintB.toBase58());
      expect(poolAccount.tokens.length).to.equal(2);
      expect(poolAccount.bump).to.be.a("number");
      expect(poolAccount.isActive).to.be.true;
      expect(poolAccount.tokens[0].balance.toNumber()).to.equal(0);
      expect(poolAccount.tokens[1].balance.toNumber()).to.equal(0);
    });
    it("deposits liquidity", async () => {
      const pool = getPoolPDA();
      const vaultA = getVaultAPDA(pool);
      const vaultB = getVaultBPDA(pool);

      const lpMintAccount = await getMint(
        provider.connection,
        lpMint.publicKey
      );
      expect(Number(lpMintAccount.supply)).to.be.eq(0);

      const depositAmount = new BN(100000000000);
      await program.methods
        .deposit(
          new BN(0), // lp_amount (0 = first deposit, calculated internally)
          depositAmount, // max token A
          depositAmount // max token B
        )
        .accounts({
          pool,
          user: payer.publicKey,
          lpMint: lpMint.publicKey,
          tokenAMint: mintA,
          tokenBMint: mintB,
          userTokenA,
          userTokenB,
        })
        .rpc();

      // Expect statements
      const poolAccount = await program.account.weightedPool.fetch(pool);
      expect(poolAccount.tokens[0].balance.toString()).to.equal(
        depositAmount.toString()
      );
      expect(poolAccount.tokens[1].balance.toString()).to.equal(
        depositAmount.toString()
      );
      expect(poolAccount.lpMint.toBase58()).to.equal(
        lpMint.publicKey.toBase58()
      );
      expect(poolAccount.tokens.length).to.equal(2);
      expect(poolAccount.isActive).to.be.true;

      // Check LP mint supply after deposit
      const lpMintAccount1 = await getMint(
        provider.connection,
        lpMint.publicKey
      );
      // The LP supply after first deposit should be > 0
      expect(Number(lpMintAccount1.supply)).to.be.greaterThan(0);
    });
    it("swaps tokens", async () => {
      const pool = getPoolPDA();
      const vaultA = getVaultAPDA(pool);
      const vaultB = getVaultBPDA(pool);

      const amountIn = new BN(100000000000);
      const minAmountOut = new BN(1);

      // Get balances BEFORE
      const userABefore = await getAccount(provider.connection, userTokenA);
      const userBBefore = await getAccount(provider.connection, userTokenB);

      await program.methods
        .swap(amountIn, minAmountOut)
        .accounts({
          pool,
          mintIn: mintA,
          mintOut: mintB,
          userTokenIn: userTokenA,
          userTokenOut: userTokenB,
          vaultTokenIn: vaultA,
          vaultTokenOut: vaultB,
          user: payer.publicKey,
        })
        .rpc();

      // Get balances AFTER
      const userAAfter = await getAccount(provider.connection, userTokenA);
      const userBAfter = await getAccount(provider.connection, userTokenB);

      expect(Number(userAAfter.amount)).to.be.lessThan(
        Number(userABefore.amount)
      );
      expect(Number(userBAfter.amount)).to.be.greaterThan(
        Number(userBBefore.amount)
      );
    });
  });

  describe("Stable Pool", async () => {
    it("initializes stable pool", async () => {
      let tx = await program.methods
        .initializeStablePool(
          new BN(3_000_000), // swap_fee
          new BN(100) // amp (100 is typical for stables)
        )
        .accounts({
          lpMint: stableLpMint.publicKey,
          tokenMintA: mintA,
          tokenMintB: mintB,
          payer: payer.publicKey,
        })
        .signers([stableLpMint])
        .rpc();

      // Assert stable pool PDA created and mints correct
      const stablePool = getStablePoolPDA();

      const stablePoolAccount = await provider.connection.getAccountInfo(
        stablePool
      );

      expect(stablePoolAccount.owner.toBase58()).to.be.eq(
        program.programId.toBase58()
      );

      // Also check the LP mint exists
      const stableLpMintAccount = await getMint(
        provider.connection,
        stableLpMint.publicKey
      );
      expect(stableLpMintAccount.mintAuthority.toBase58()).to.equal(
        authority.toBase58()
      );
      expect(Number(stableLpMintAccount.supply)).to.equal(0);

      const poolAccount = await program.account.stablePool.fetch(stablePool);
      expect(poolAccount.amp.toNumber()).to.equal(100 * 1000);
      expect(poolAccount.swapFee.toNumber()).to.equal(3_000_000);
      expect(poolAccount.isActive).to.be.true;
    });

    it("deposits liquidity", async () => {
      const pool = getStablePoolPDA();
      const vaultA = getStableVaultAPDA(pool);
      const vaultB = getStableVaultBPDA(pool);

      // Check LP supply before
      const lpMintBefore = await getMint(provider.connection, stableLpMint.publicKey);
      expect(Number(lpMintBefore.supply)).to.equal(0);

      const depositAmount = new BN(100_000_000_000); // 100 tokens

      await program.methods
        .stableDeposit(
          depositAmount,  // max_amount_a
          depositAmount,  // max_amount_b
          new BN(0)       // lp_amount (0 for first deposit)
        )
        .accounts({
          pool,
          mintA: mintA,
          mintB: mintB,
          lpMint: stableLpMint.publicKey,
          vaultTokenA: vaultA,
          vaultTokenB: vaultB,
          userTokenA,
          userTokenB,
          user: payer.publicKey,
        })
        .rpc();

      // Assert LP minted
      const lpMintAfter = await getMint(provider.connection, stableLpMint.publicKey);
      expect(Number(lpMintAfter.supply)).to.be.greaterThan(0);

      // Assert pool balances updated
      const poolAccount = await program.account.stablePool.fetch(pool);
      expect(poolAccount.tokens[0].balance.toString()).to.equal(depositAmount.toString());
      expect(poolAccount.tokens[1].balance.toString()).to.equal(depositAmount.toString());
    });

    it("swaps tokens", async () => {
      const pool = getStablePoolPDA();
      const vaultA = getStableVaultAPDA(pool);
      const vaultB = getStableVaultBPDA(pool);

      // Get balances BEFORE
      const userABefore = await getAccount(provider.connection, userTokenA);
      const userBBefore = await getAccount(provider.connection, userTokenB);

      const amountIn = new BN(10_000_000_000); // 10 tokens
      const minAmountOut = new BN(1);

      await program.methods
        .stableSwap(amountIn, minAmountOut)
        .accounts({
          pool,
          mintIn: mintA,
          mintOut: mintB,
          vaultTokenIn: vaultA,
          vaultTokenOut: vaultB,
          userTokenIn: userTokenA,
          userTokenOut: userTokenB,
          user: payer.publicKey,
        })
        .rpc();

      // Get balances AFTER
      const userAAfter = await getAccount(provider.connection, userTokenA);
      const userBAfter = await getAccount(provider.connection, userTokenB);

      // Assert: user A decreased, user B increased
      expect(Number(userAAfter.amount)).to.be.lessThan(Number(userABefore.amount));
      expect(Number(userBAfter.amount)).to.be.greaterThan(Number(userBBefore.amount));
    });
  });
});
