# Mini-Stabble DEX

> A Solana DEX implementation inspired by [Stabble](https://stabble.org), featuring **Weighted Pools**, **StableSwap Pools**, and an **Arbitrage Scanner** — demonstrating deep understanding of AMM mathematics and Stabble's Smart Liquidity Architecture (SLA).

---

## 🎯 Why I Built This

I studied Stabble's whitepaper and was impressed by their **internal arbitrage** concept — capturing MEV profits for LPs instead of external bots. This project demonstrates:

1. **Deep DeFi math understanding** — Implemented Newton-Raphson for StableSwap from scratch
2. **Multi-pool architecture** — Both Weighted and Stable pools with shared math library
3. **Arbitrage detection** — TypeScript scanner comparing prices across pool types (simplified SLA)

---

## 🏗️ Architecture

```
mini-stabble/
├── programs/mini-stabble/
│   └── src/
│       ├── lib.rs                    # Program entrypoint
│       ├── errors.rs                 # Custom errors
│       ├── constants.rs              # Fee precision, etc.
│       ├── math/
│       │   ├── fixed.rs              # Fixed-point arithmetic (SCALE = 10^9)
│       │   ├── weighted.rs           # Weighted pool math
│       │   └── stable.rs             # StableSwap math (Newton-Raphson)
│       ├── state/
│       │   ├── weighted_pool.rs      # WeightedPool account
│       │   └── stable_pool.rs        # StablePool account
│       └── instructions/
│           ├── initialize_weighted_pool.rs
│           ├── initialize_stable_pool.rs
│           ├── swap.rs               # Weighted swap
│           ├── stable_swap.rs        # Stable swap
│           ├── deposit.rs            # Weighted deposit
│           └── stable_deposit.rs     # Stable deposit
├── sdk/src/
│   ├── spotPrice.ts                  # Spot price calculation
│   └── scanner.ts                    # Arbitrage opportunity detector
└── tests/
    └── mini-stabble.ts               # 7 integration tests
```

---

## 📐 Mathematical Foundations

### Fixed-Point Arithmetic

All calculations use fixed-point integers to avoid floating-point non-determinism:

```
SCALE = 10^9 (1 billion)

Example: 0.5 is stored as 500,000,000
         1.0 is stored as 1,000,000,000
```

### Weighted Pool Math (Balancer-style)

**Invariant:**

$$K = \prod_{i=0}^{n} B_i^{W_i}$$

Where $B_i$ = balance, $W_i$ = normalized weight (weights sum to 1).

**Swap: Output Given Input**

$$\Delta_{out} = B_{out} \times \left(1 - \left(\frac{B_{in}}{B_{in} + \Delta_{in}}\right)^{\frac{W_{in}}{W_{out}}}\right)$$

**Spot Price:**

$$P = \frac{W_{in}}{W_{out}} \times \frac{B_{out}}{B_{in}}$$

---

### StableSwap Math (Curve-style)

**Invariant (Newton-Raphson):**

$$An^n\sum x_i + D = ADn^n + \frac{D^{n+1}}{n^n \prod x_i}$$

For 2 tokens, simplified:

$$4A(x + y) + D = 4AD + \frac{D^3}{4xy}$$

**Newton's Iteration:**

$$D_{new} = \frac{(Ann \cdot S + n \cdot D_P \cdot P) \times D}{(Ann - P) \times D + (n+1) \cdot D_P \cdot P}$$

Where:
- $Ann = A \times n$ (amplification factor scaled)
- $S = \sum B_i$ (sum of balances)
- $D_P = \frac{D^n}{\prod (n \cdot B_i)}$
- $P = 1000$ (AMP_PRECISION constant)

**Calculating Y (token balance):**

Given invariant $D$ and all other balances, find $y$ using Newton's method:

$$y_{new} = \frac{y^2 + c}{2y + b - D}$$

Where:
- $c = \frac{D^2 \cdot P}{Ann \cdot \Pi} \times B_y$
- $b = \frac{D \cdot P}{Ann} + S'$ (sum excluding $y$)

**Spot Price (Numerical Approximation):**

$$SpotPrice \approx \frac{AmountOut}{AmountIn}$$

We calculate `calc_out_given_in(ref_amount)` with a small reference amount (10^9) to get instantaneous price.

---

### Arbitrage Detection

**Price Difference:**

$$\text{priceDiff\%} = \frac{|P_{weighted} - P_{stable}|}{\min(P_{weighted}, P_{stable})} \times 100$$

**Fee Consideration:**

$$\text{totalFees\%} = \text{weightedFee\%} + \text{stableFee\%}$$

$$\text{netProfit\%} = \text{priceDiff\%} - \text{totalFees\%}$$

**Profitable if:**

$$\text{netProfit\%} > \text{minThreshold\%}$$

---

## ✅ What's Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| **Fixed-Point Math** | ✅ | `mul_down`, `mul_up`, `div_down`, `div_up`, `pow_down`, `pow_up`, `complement` |
| **Weighted Pool** | ✅ | Initialize, Swap, Deposit |
| **StableSwap Pool** | ✅ | Initialize, Swap, Deposit with Newton-Raphson |
| **Spot Price (SDK)** | ✅ | Both pool types |
| **Arbitrage Scanner** | ✅ | Fee-aware detection |
| **Integration Tests** | ✅ | 7 tests passing |

---

## 🧪 Running Tests

```bash
cd mini-stabble

# Build
anchor build

# Run all tests
anchor test

# Run Rust unit tests only
cargo test --lib
```

**Test Output:**
```
  mini-stabble
    weighted pool
      ✔ initializes weighted pool
      ✔ deposits liquidity
      ✔ swaps tokens
    Stable Pool
      ✔ initializes stable pool
      ✔ deposits liquidity
      ✔ swaps tokens
    arbitrage
      ✔ detects arbitrage opportunity

  7 passing
```

---

## 📊 Arbitrage Scanner Usage

```typescript
import { detectArbitrage } from './sdk/src/scanner';

const result = detectArbitrage(
  {
    balanceA: 100_000_000_000n,
    balanceB: 95_000_000_000n,
    weightA: 500_000_000n,  // 0.5
    weightB: 500_000_000n,  // 0.5
    swapFee: 3_000_000n,    // 0.3%
  },
  {
    balanceA: 100_000_000_000n,
    balanceB: 100_000_000_000n,
    swapFee: 3_000_000n,
    amp: 100_000n,          // 100 * AMP_PRECISION
  }
);

// Output:
// {
//   weightedPrice: 0.95,
//   stablePrice: 1.0,
//   priceDiffPercent: 5.26,
//   totalFeesPercent: 0.6,
//   netProfitPercent: 4.66,
//   profitable: true,
//   direction: 'weighted_to_stable'
// }
```

---

## 🔗 Technical Decisions

| Decision | Why |
|----------|-----|
| **Newton-Raphson for StableSwap** | Matches Curve's approach for finding invariant D |
| **U192 for intermediate math** | Prevents overflow in D² calculations |
| **Spot price via small swap** | More accurate than derivative for imbalanced pools |
| **Fee-aware arbitrage** | Real-world profitability requires considering both swap fees |
| **Shared math library** | Code reuse between pool types |

---

## 📚 References

- [Stabble Whitepaper](https://stabble.org)
- [Curve StableSwap Paper](https://curve.fi/files/stableswap-paper.pdf)
- [Balancer V2 Weighted Math](https://github.com/balancer-labs/balancer-v2-monorepo)

---

## 🚀 Future Work

- [ ] `execute_arbitrage` instruction (atomic buy→sell)
- [ ] Profit routing to LPs (Stabble's SLA innovation)
- [ ] Multi-hop router
- [ ] Devnet deployment

---

**Built by Veerbal Singh** | Inspired by Stabble's Smart Liquidity Architecture
