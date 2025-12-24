# Mini-Stabble: Weighted Pool DEX Implementation

A Solana DEX implementation inspired by Stabble, featuring weighted pools with production-grade fixed-point arithmetic.

## Overview

This project implements the core math libraries for a decentralized exchange (DEX) supporting **weighted pools** — pools where tokens can have different weights (e.g., 80/20 ETH/USDC instead of the traditional 50/50).

## Architecture

```
programs/mini-stabble/
└── src/
    ├── lib.rs           # Program entrypoint
    ├── errors.rs        # Custom error types
    ├── math/
    │   ├── mod.rs       # Math module exports
    │   ├── fixed.rs     # Fixed-point arithmetic
    │   └── weighted.rs  # Weighted pool math
    ├── state/           # Account structures (coming soon)
    └── instructions/    # Program instructions (coming soon)
```

---

## Fixed-Point Arithmetic (`math/fixed.rs`)

### Why Fixed-Point?

Solana programs cannot use floating-point numbers (`f32`, `f64`). All calculations use **fixed-point integers** with a scale factor.

```
SCALE = 1,000,000,000 (10^9)

Example: 0.5 is stored as 500,000,000
         1.0 is stored as 1,000,000,000
```

### Implemented Traits

| Trait | Methods | Purpose |
|-------|---------|---------|
| `FixedMul` | `mul_down`, `mul_up` | Multiply two fixed-point numbers |
| `FixedDiv` | `div_down`, `div_up` | Divide two fixed-point numbers |
| `FixedComplement` | `complement` | Calculate `1 - x` |
| `FixedPow` | `pow_down`, `pow_up` | Raise to a power |

### Rounding Strategy

All operations that could result in fractional values must round. We always round **in favor of the protocol**:

- When calculating what user **receives** → round **DOWN** (user gets less)
- When calculating what user **pays** → round **UP** (user pays more)

---

## Weighted Pool Math (`math/weighted.rs`)

### The Invariant Formula

Weighted pools maintain a constant invariant:

$$K = \prod_{i=0}^{n} B_i^{W_i} = B_0^{W_0} \times B_1^{W_1} \times ... \times B_n^{W_n}$$

Where:
- $B_i$ = Balance of token $i$
- $W_i$ = Weight of token $i$ (weights sum to 1)
- $K$ = Invariant (constant before and after swaps)

**Example:** A 50/50 pool with 100 SOL and 400 USDC:
```
K = 100^0.5 × 400^0.5 = 10 × 20 = 200
```

### Swap Formula: `calc_out_given_in`

Given an input amount, calculate the output:

$$\Delta_{out} = B_{out} \times \left(1 - \left(\frac{B_{in}}{B_{in} + \Delta_{in}}\right)^{\frac{W_{in}}{W_{out}}}\right)$$

**Implementation:**
```rust
pub fn calc_out_given_in(
    balance_in: u128,
    weight_in: u128,
    balance_out: u128,
    weight_out: u128,
    amount_in: u128,
) -> Result<u128, MiniStabbleError> {
    // base = balance_in / (balance_in + amount_in)
    let base = balance_in.div_up(...)?;
    
    // exponent = weight_in / weight_out
    let exponent = weight_in.div_down(weight_out)?;
    
    // power = base ^ exponent
    let power = base.pow_up(exponent)?;
    
    // amount_out = balance_out × (1 - power)
    let complement = power.complement();
    let amount_out = balance_out.mul_down(complement)?;
    
    Ok(amount_out)
}
```

### Swap Formula: `calc_in_given_out`

Given a desired output, calculate the required input:

$$\Delta_{in} = B_{in} \times \left(\left(\frac{B_{out}}{B_{out} - \Delta_{out}}\right)^{\frac{W_{out}}{W_{in}}} - 1\right)$$

**Key Difference:** Here the base is > 1, so rounding logic changes accordingly.

---

## Rounding Strategy Deep Dive

### For `calc_out_given_in` (user receives LESS):

| Step | Operation | Rounding | Reasoning |
|------|-----------|----------|-----------|
| base | `div_up` | Larger base → larger power | |
| exponent | `div_down` | For base < 1: smaller exp → larger power | |
| power | `pow_up` | Larger power → smaller complement | |
| amount_out | `mul_down` | User receives less | Protects pool |

### For `calc_in_given_out` (user pays MORE):

| Step | Operation | Rounding | Reasoning |
|------|-----------|----------|-----------|
| base | `div_up` | Larger base → larger power | |
| exponent | `div_up` | For base > 1: larger exp → larger power | |
| power | `pow_up` | Larger power → larger complement | |
| amount_in | `mul_up` | User pays more | Protects pool |

---

## Dependencies

- `anchor-lang` - Solana program framework
- `fixed` - Fixed-point number types
- `fixed-exp` - Fixed-point exponentiation (adapted from Stabble)

---

## Building

```bash
cd mini-stabble
anchor build
```

---

## Roadmap

- [x] Fixed-point arithmetic library
- [x] Weighted pool invariant calculation
- [x] Weighted pool swap functions
- [ ] StableSwap math (Curve-style pools)
- [ ] Pool state accounts
- [ ] Swap instruction handlers
- [ ] Internal arbitrage logic
- [ ] TypeScript SDK

---

## References

- [Balancer V2 Weighted Math](https://github.com/balancer-labs/balancer-v2-monorepo/blob/master/pkg/pool-weighted/contracts/WeightedMath.sol)
- [Stabble DEX](https://stabble.org)
- [Curve StableSwap Whitepaper](https://curve.fi/files/stableswap-paper.pdf)
