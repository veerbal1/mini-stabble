/**
 * Same as Rust Invariant code
 */
// Constants matching Rust implementation
const AMP_PRECISION = 1000n;
const MAX_LOOP_LIMIT = 256;
const DEFAULT_INV_THRESHOLD = 100n;

// Weighted Pool spot price
// Formula: (weight_in / weight_out) × (balance_out / balance_in)
export function calcWeightedSpotPrice(
  balanceIn: bigint,
  weightIn: bigint,
  balanceOut: bigint,
  weightOut: bigint
): number {
  // Convert to number for simple calculation
  const price =
    (Number(weightIn) / Number(weightOut)) *
    (Number(balanceOut) / Number(balanceIn));
  return price;
}

/**
 * Calculate StableSwap invariant D using Newton-Raphson iteration.
 * Matches Rust: stable.rs calc_invariant
 *
 * Formula: An²(x+y) + D = AnD + D³/(4xy)
 */
function calcInvariant(amp: bigint, balances: bigint[]): bigint | null {
  const n = BigInt(balances.length);
  const sum = balances.reduce((a, b) => a + b, 0n);

  if (sum === 0n) return 0n;

  const ann = amp * n; // A * n
  let d = sum;

  for (let i = 0; i < MAX_LOOP_LIMIT; i++) {
    // D_P = D^n / (n^n * ∏balances)
    let dp = d;
    for (const balance of balances) {
      dp = (dp * d) / (n * balance);
    }

    // Numerator: (Ann * S + n * D_P * AMP_PRECISION) * D
    const num = (ann * sum + n * dp * AMP_PRECISION) * d;

    // Denominator: (Ann - AMP_PRECISION) * D + (n + 1) * D_P * AMP_PRECISION
    const den = (ann - AMP_PRECISION) * d + (n + 1n) * dp * AMP_PRECISION;

    const dNew = num / den;

    const diff = dNew > d ? dNew - d : d - dNew;
    if (diff <= DEFAULT_INV_THRESHOLD) {
      return dNew;
    }
    d = dNew;
  }

  return null;
}

/**
 * Calculate token balance given invariant and all other balances.
 * Used for swap calculations - finds y given x.
 */
function getTokenBalanceGivenInvariantAndOthers(
  amp: bigint,
  balances: bigint[],
  invariant: bigint,
  tokenIndex: number
): bigint | null {
  if (balances.length === 0 || tokenIndex >= balances.length) return null;

  const n = BigInt(balances.length);
  const ann = amp * n;

  // Calculate sum and product (excluding target token conceptually)
  let sum = 0n;
  const firstBalance = balances[0];
  if (firstBalance === undefined) return null;
  let p = firstBalance * n;

  for (let i = 0; i < balances.length; i++) {
    const balance = balances[i];
    if (balance === undefined) return null;
    if (i > 0) {
      const pi = balance * n;
      p = (p * pi) / invariant;
    }
    sum = sum + balance;
  }

  // Remove target token from sum
  const targetBalance = balances[tokenIndex];
  if (targetBalance === undefined) return null;
  sum = sum - targetBalance;

  const invariant2 = invariant * invariant;

  // c = D² * AMP_PRECISION / (Ann * P) * balance_at_index
  const c = ((invariant2 * AMP_PRECISION) / (ann * p)) * targetBalance;

  // b = D * AMP_PRECISION / Ann + sum
  const b = (invariant * AMP_PRECISION) / ann + sum;

  // Initial approximation
  let tokenBalance = (invariant2 + c) / (invariant + b);

  // Newton-Raphson: y = (y² + c) / (2y + b - D)
  for (let i = 0; i < 64; i++) {
    const prevBalance = tokenBalance;

    tokenBalance =
      (tokenBalance * tokenBalance + c) / (2n * tokenBalance + b - invariant);

    const diff =
      tokenBalance > prevBalance
        ? tokenBalance - prevBalance
        : prevBalance - tokenBalance;

    if (diff <= 1n) {
      return tokenBalance;
    }
  }

  return null;
}

/**
 * Calculate output amount for a given input amount (swap calculation).
 * Matches Rust: calc_out_given_in
 */
function calcOutGivenIn(
  amp: bigint,
  balances: bigint[],
  tokenIndexIn: number,
  tokenIndexOut: number,
  amountIn: bigint
): bigint | null {
  const invariant = calcInvariant(amp, balances);
  if (invariant === null) return null;

  // Create new balances with amountIn added
  const newBalances = [...balances];
  const currentIn = newBalances[tokenIndexIn];
  if (currentIn === undefined) return null;
  newBalances[tokenIndexIn] = currentIn + amountIn;

  const balanceOut = balances[tokenIndexOut];
  if (balanceOut === undefined) return null;

  // Calculate what output balance should be
  const finalBalanceOut = getTokenBalanceGivenInvariantAndOthers(
    amp,
    newBalances,
    invariant,
    tokenIndexOut
  );

  if (finalBalanceOut === null) return null;

  // Output = current - final - 1 (rounding protection)
  if (balanceOut <= finalBalanceOut) return 0n;
  return balanceOut - finalBalanceOut - 1n;
}

/**
 * StableSwap spot price using derivative approach.
 * Price = dy/dx for infinitesimal swap.
 *
 * We calculate this by doing a small swap (1e9 units) and taking the ratio.
 * This matches Rust's calc_spot_price approach.
 */
export function calcStableSpotPrice(
  balanceIn: bigint,
  balanceOut: bigint,
  amp: bigint = 100000n // Default: 100 * AMP_PRECISION
): number {
  const balances = [balanceIn, balanceOut];

  // Use a reference amount for spot price calculation
  // Larger = more accurate for larger pools
  const refAmount = 1_000_000_000n; // 1e9

  const amountOut = calcOutGivenIn(amp, balances, 0, 1, refAmount);

  if (amountOut === null || amountOut === 0n) {
    // Fallback to simple ratio if calculation fails
    return Number(balanceOut) / Number(balanceIn);
  }

  // Spot price = amountOut / amountIn
  return Number(amountOut) / Number(refAmount);
}
