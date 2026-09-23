use crate::params::Params;
use subtle::{Choice, ConditionallySelectable};

/// Multiply by a table root using its precomputed Shoup quotient.
/// `root < modulus`, `modulus > 0`, and `quotient = floor(root * 2^64 / modulus)`.
/// With B=2^64 and operand<B, floor(operand*quotient/B) underestimates
/// floor(operand*root/modulus) by at most one. The residual is therefore in
/// [0, 2*modulus), so one conditional subtraction is exactly the old remainder.
/// Keep the residual wide; no additional restriction on the modulus is needed
/// here. The surrounding butterflies retain their existing lazy-range limits.
/// This is the precomputed-operand reduction used by Microsoft SEAL's
/// util/uintarithsmallmod.h multiply_uint_mod(x, MultiplyUIntModOperand, modulus).
#[inline(always)]
fn multiply_root(operand: u64, root: u64, quotient: u64, modulus: u64) -> u64 {
    let estimate = (u128::from(operand) * u128::from(quotient)) >> 64;
    let modulus = u128::from(modulus);
    let residual = u128::from(root) * u128::from(operand) - estimate * modulus;
    // Choice carries an optimization barrier: an ordinary bool comparison
    // became coefficient-dependent jumps in the x86-64-v3 release build.
    u128::conditional_select(
        &residual.wrapping_sub(modulus),
        &residual,
        Choice::from((residual < modulus) as u8),
    ) as u64
}

pub fn ntt_forward_alt(params: &Params, operand_overall: &mut [u64]) {
    let log_n = params.poly_len_log2;
    let n = 1 << log_n;

    for coeff_mod in 0..params.crt_count {
        let operand = &mut operand_overall[coeff_mod * n..coeff_mod * n + n];

        let forward_table = params.get_ntt_forward_table(coeff_mod);
        let forward_table_prime = params.get_ntt_forward_prime_table(coeff_mod);
        let modulus_small = params.moduli[coeff_mod];
        let two_times_modulus_small = 2 * modulus_small;

        for mm in 0..log_n {
            let m = 1 << mm;
            let t = n >> (mm + 1);

            let mut it = operand.chunks_exact_mut(2 * t);

            for i in 0..m {
                let w = forward_table[m + i];
                let w_prime = forward_table_prime[m + i];

                let op = it.next().unwrap();

                for j in 0..t {
                    let x: u64 = op[j] as u64;
                    let y: u64 = op[t + j] as u64;

                    let curr_x: u64 =
                        x - (two_times_modulus_small * ((x >= two_times_modulus_small) as u64));
                    let q_new = multiply_root(y, w, w_prime, modulus_small);

                    op[j] = curr_x as u64 + q_new;
                    op[t + j] = curr_x as u64 + ((two_times_modulus_small as u64) - q_new);
                }
            }
        }

        for i in 0..n {
            operand[i] -= ((operand[i] >= two_times_modulus_small as u64) as u64)
                * two_times_modulus_small as u64;
            operand[i] -= ((operand[i] >= modulus_small as u64) as u64) * modulus_small as u64;
        }
    }
}

pub fn ntt_inverse_alt(params: &Params, operand_overall: &mut [u64]) {
    for coeff_mod in 0..params.crt_count {
        let n = params.poly_len;

        let operand = &mut operand_overall[coeff_mod * n..coeff_mod * n + n];

        let inverse_table = params.get_ntt_inverse_table(coeff_mod);
        let inverse_table_prime = params.get_ntt_inverse_prime_table(coeff_mod);
        let modulus = params.moduli[coeff_mod];
        let two_times_modulus: u64 = 2 * modulus;

        for mm in (0..params.poly_len_log2).rev() {
            let h = 1 << mm;
            let t = n >> (mm + 1);

            let mut it = operand.chunks_exact_mut(2 * t);

            for i in 0..h {
                let w = inverse_table[h + i];
                let w_prime = inverse_table_prime[h + i];

                let op = it.next().unwrap();

                for j in 0..t {
                    let x = op[j];
                    let y = op[t + j];

                    let t_tmp = two_times_modulus - y + x;
                    let curr_x = x + y - (two_times_modulus * (((x << 1) >= t_tmp) as u64));
                    let res_x = (curr_x + (modulus * ((t_tmp & 1) as u64))) >> 1;
                    let res_y = multiply_root(t_tmp, w, w_prime, modulus);

                    op[j] = res_x;
                    op[t + j] = res_y;
                }
            }
        }

        for i in 0..n {
            operand[i] -= ((operand[i] >= two_times_modulus) as u64) * two_times_modulus;
            operand[i] -= ((operand[i] >= modulus) as u64) * modulus;
        }
    }
}

#[cfg(test)]
#[path = "alt_division_reference.rs"]
mod division_reference;

#[cfg(test)]
mod reduction_tests {
    use super::*;
    use rand::{Rng, SeedableRng};

    #[test]
    fn precomputed_root_product_matches_wide_remainder() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x53484f5550);
        for modulus in [
            2,
            3,
            17,
            12289,
            72_057_594_037_641_217,
            (1u64 << 62) - 1,
            (1u64 << 63) + 1,
            u64::MAX,
        ] {
            let operands = [0, 1, modulus - 1, modulus, u64::MAX - 1, u64::MAX];
            for root in [0, 1, modulus / 2, modulus - 1] {
                let quotient = ((u128::from(root) << 64) / u128::from(modulus)) as u64;
                for operand in operands {
                    assert_eq!(
                        multiply_root(operand, root, quotient, modulus),
                        ((u128::from(operand) * u128::from(root)) % u128::from(modulus)) as u64
                    );
                }
            }
            for _ in 0..20_000 {
                let root = rng.gen::<u64>() % modulus;
                let operand = rng.gen::<u64>();
                let quotient = ((u128::from(root) << 64) / u128::from(modulus)) as u64;
                assert_eq!(
                    multiply_root(operand, root, quotient, modulus),
                    ((u128::from(operand) * u128::from(root)) % u128::from(modulus)) as u64
                );
            }
        }
    }

    fn parameters(n: usize, q: u64) -> Params {
        Params::init(n, &[q], 6.4, 1, 4, 20, 4, 4, 4, 4, false, 0, 0, 1, n, 0)
    }

    #[test]
    fn transforms_match_division_reference_at_boundaries_and_dense_inputs() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x4e5454);
        for (n, q) in [
            (8, 17),
            (2048, 12_289),
            (2048, 72_057_594_037_641_217),
            (2048, 180_143_985_094_819_841),
        ] {
            let params = parameters(n, q);
            for pattern in 0..12 {
                let original: Vec<u64> = (0..n)
                    .map(|i| match pattern {
                        0 => 0,
                        1 => q - 1,
                        2 => {
                            if i == 0 {
                                1
                            } else {
                                0
                            }
                        }
                        3 => {
                            if i % 2 == 0 {
                                q - 1
                            } else {
                                0
                            }
                        }
                        _ => rng.gen::<u64>() % q,
                    })
                    .collect();
                let mut actual = original.clone();
                let mut expected = original.clone();
                ntt_forward_alt(&params, &mut actual);
                division_reference::ntt_forward_alt(&params, &mut expected);
                assert_eq!(actual, expected, "forward n={n} q={q} pattern={pattern}");
                ntt_inverse_alt(&params, &mut actual);
                division_reference::ntt_inverse_alt(&params, &mut expected);
                assert_eq!(actual, expected, "inverse n={n} q={q} pattern={pattern}");
                assert_eq!(actual, original);
                // Also compare inverse on independent spectra, avoiding reliance
                // on a forward/inverse pair cancelling the same mistake.
                let mut actual = original.clone();
                let mut expected = original;
                ntt_inverse_alt(&params, &mut actual);
                division_reference::ntt_inverse_alt(&params, &mut expected);
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn transform_product_matches_direct_negacyclic_convolution() {
        let params = parameters(8, 17);
        let a = vec![0, 1, 16, 3, 7, 0, 15, 2];
        let b = vec![16, 0, 1, 8, 4, 3, 2, 1];
        let mut expected = vec![0i64; 8];
        for i in 0..8 {
            for j in 0..8 {
                let product = a[i] as i64 * b[j] as i64;
                expected[(i + j) % 8] += if i + j < 8 { product } else { -product };
            }
        }
        let mut actual = a;
        let mut rhs = b;
        ntt_forward_alt(&params, &mut actual);
        ntt_forward_alt(&params, &mut rhs);
        for i in 0..8 {
            actual[i] = actual[i] * rhs[i] % 17;
        }
        ntt_inverse_alt(&params, &mut actual);
        assert_eq!(
            actual,
            expected
                .into_iter()
                .map(|v| v.rem_euclid(17) as u64)
                .collect::<Vec<_>>()
        );
    }
}
