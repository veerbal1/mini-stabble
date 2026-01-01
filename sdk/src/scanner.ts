import { PublicKey } from "@solana/web3.js";
// For Buffer function
import * as anchor from "@coral-xyz/anchor";
// @ts-expect-error
import { calcWeightedSpotPrice, calcStableSpotPrice } from "./spotPrice.ts";

const PROGRAM_ID = new PublicKey(
  "FURtuxyXWgpnETkNho8PL6mpuRh9mCnVsWgUY14JzusX"
);

const WEIGHT_POOL_SEED = Buffer.from("WEIGHT_POOL");
const STABLE_POOL_SEED = Buffer.from("STABLE_POOL");

// Fee precision: 1_000_000_000 = 100%
const FEE_SCALE = 1_000_000_000;

interface ArbitrageOpportunity {
  weightedPrice: number;
  stablePrice: number;
  priceDiffPercent: number;
  totalFeesPercent: number;
  netProfitPercent: number;
  profitable: boolean;
  direction: "weighted_to_stable" | "stable_to_weighted";
}

export interface PoolData {
  balanceA: bigint;
  balanceB: bigint;
  weightA?: bigint; // only for weighted
  weightB?: bigint; // only for weighted
  swapFee: bigint; // in FEE_SCALE (e.g., 3_000_000 = 0.3%)
  amp?: bigint; // only for stable pools (amp * AMP_PRECISION)
}

export function detectArbitrage(
  weightedPool: PoolData,
  stablePool: PoolData,
  minProfitPercent: number = 0.1 // 0.1% minimum net profit after fees
): ArbitrageOpportunity | null {
  const weightedPrice = calcWeightedSpotPrice(
    weightedPool.balanceA,
    weightedPool.weightA!,
    weightedPool.balanceB,
    weightedPool.weightB!
  );

  // Use amp from pool, default to 100 * 1000 = 100000 if not provided
  const amp = stablePool.amp ?? 100000n;
  const stablePrice = calcStableSpotPrice(
    stablePool.balanceA,
    stablePool.balanceB,
    amp
  );

  // Calculate price difference percentage
  const priceDiff = Math.abs(weightedPrice - stablePrice);
  const priceDiffPercent =
    (priceDiff / Math.min(weightedPrice, stablePrice)) * 100;

  // Calculate total fees (both pools charge fees)
  // Convert from FEE_SCALE to percentage
  const weightedFeePercent =
    (Number(weightedPool.swapFee) / FEE_SCALE) * 100;
  const stableFeePercent =
    (Number(stablePool.swapFee) / FEE_SCALE) * 100;

  // Total fees = buy fee + sell fee
  const totalFeesPercent = weightedFeePercent + stableFeePercent;

  // Net profit = price diff - total fees
  const netProfitPercent = priceDiffPercent - totalFeesPercent;

  // Only profitable if net profit exceeds minimum threshold
  if (netProfitPercent < minProfitPercent) {
    return null;
  }

  const direction =
    weightedPrice > stablePrice
      ? "stable_to_weighted" // Buy cheap on stable, sell on weighted
      : "weighted_to_stable"; // Buy cheap on weighted, sell on stable

  return {
    weightedPrice,
    stablePrice,
    priceDiffPercent,
    totalFeesPercent,
    netProfitPercent,
    profitable: true,
    direction,
  };
}
