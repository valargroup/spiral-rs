# Single-CRT NTT reduction candidate

This candidate changes only the final reduction of a precomputed-root product
in `src/ntt/alt.rs`. It preserves the ring, roots, root ordering, butterfly
schedule, canonical output range, client protocol and wire bytes.

The current 56-bit single-CRT path already computes Shoup's approximate
quotient, but then uses a general 128-bit remainder on the bounded residual.
The candidate replaces that remainder with one conditional subtraction.
Microsoft SEAL uses the same precomputed-operand reduction in
[`multiply_uint_mod(x, MultiplyUIntModOperand, modulus)`](https://github.com/microsoft/SEAL/blob/main/native/src/seal/util/uintarithsmallmod.h).

Let B=2^64, 0 <= x < B, 0 <= w < p, p > 0, and w'=floor(w B/p).
Write w B/p = w' + e with 0 <= e < 1. Then
x w/p = x w'/B + x e/B, where 0 <= x e/B < 1.
Consequently floor(x w'/B) is either floor(x w/p) or one less.
The exact integer residual x w - floor(x w'/B) p lies in [0, 2p).
Subtracting p precisely when the residual is at least p returns x w mod p.
Both products fit u128 and the subtraction cannot underflow. The conditional
subtraction is performed in u128 using `subtle::Choice` and
`ConditionallySelectable`, including the case 2p exceeds u64::MAX.
Only the canonical result is narrowed to u64.

The helper is private and only receives roots and quotients from the existing
NTT tables. The quotient is not accepted from a client. Existing butterfly
input/range requirements are unchanged. This is not a new constant-time claim:
target-machine assembly must be inspected before promotion, and the change
must not introduce coefficient-dependent branches or lookups. Existing
malformed-query and binding rejection tests still apply downstream.

Validation before promotion:

- Wide reference remainders for boundary operands, roots, and moduli (including
  the live 56-bit modulus and full-word moduli), plus 160,000 seeded random pairs.
- Forward and inverse byte equality against test-only frozen division code
  from 6f5b66c; independently sampled inverse inputs avoid paired-error masking.
- Zero, maximum canonical coefficients, impulses, alternating extremes and
  dense inputs; a direct negacyclic-convolution oracle independent of the NTT.
- Full dependency tests, Linux target tests, downstream worker/query regressions,
  and isolated loaded publication measurements before any deployment.

The local 71-test release library suite and three focused Linux tests passed.
The downstream Linux prepare/activate/invalidate integration test also passed.
In the x86-64-v3 candidate, both changed reductions compile to `setb`, a
fixed-address `subtle::black_box` call and bit-mask selection, without new
coefficient-dependent jumps. This assembly observation is target-specific;
recheck it when compiler settings or dependencies change. The loaded performance
comparison is recorded separately in the spendability-pir M1 evidence.
The worker dependency pin and live fleet have not been changed by this commit.
