# Changelog

## 0.5.3-rc.1 — 2026-09-23

- Replace wide division in the single-CRT NTT's root multiplication with
  precomputed-quotient reduction. The result is tested against the previous
  division implementation at boundary and dense inputs.
- Add independent forward/inverse NTT and polynomial-product comparisons for
  the single-CRT path used by IPIR-SP.

This release candidate is the backend revision tested by IPIR-SP 0.1.0-rc.1.
