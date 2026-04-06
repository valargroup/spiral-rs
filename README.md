# spiral-rs

This is a fork of the [spiral-rs](https://github.com/menonsamir/spiral-rs) Rust implementation of some functionality in the [Spiral PIR scheme](https://eprint.iacr.org/2022/368). This fork provides core routines for use in [YPIR](https://github.com/valargroup/ypir).

This fork has been **audited by [Zellic](https://zellic.io)** as part of the YPIR audit. The audit report is available in the [YPIR repository](https://github.com/valargroup/ypir/blob/valar/artifact/audits/zellic-audit-report.pdf).

For a complete, working version of spiral-rs, please see [this repository](https://github.com/blyssprivacy/sdk/tree/main/lib).

## Building

This branch does not require AVX-512. AVX-512 support is compile-time gated behind `cfg(target_feature = "avx512f")`, and falls back to AVX2 or a generic implementation otherwise.

## Citing

Please cite the original work as:

```
@misc{spiral-rs,
  author = {Samir Menon},
  title = {spiral-rs},
  howpublished = {\url{https://github.com/menonsamir/spiral-rs}},
}
```

This fork is maintained by [valargroup](https://github.com/valargroup).
