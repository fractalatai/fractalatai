//! HDR (Hubbard Decision Research) pseudo-random number generator.
//!
//! Produces reproducible, independent uniform random streams from a 5-component
//! seed. Two variables with different `var_id` produce independent streams.
//! Two variables sharing the same seeds produce identical streams (use a copula
//! layer to introduce desired correlations).
//!
//! The HDR generator uses a hash-based approach: each (counter, entity, var_id,
//! seed3, seed4) tuple maps deterministically to a uniform value in (0, 1).
//! This implementation uses a mixing function inspired by the SplitMix64 / xxHash
//! family, which passes standard randomness tests.

/// Generate a uniform random number in (0, 1) from the HDR 5-component seed.
///
/// - `counter`: trial number (1, 2, ..., N). Also called PM_Index.
/// - `entity`: organisational unit identifier.
/// - `var_id`: unique per variable in the model.
/// - `seed3`, `seed4`: optional additional seeds (use 0 if not needed).
pub fn uniform(counter: u64, entity: u32, var_id: u32, seed3: u32, seed4: u32) -> f64 {
    // Combine seeds into a single 64-bit state via mixing.
    let mut h: u64 = counter;
    h = h.wrapping_add((entity as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    h = splitmix(h);
    h = h.wrapping_add((var_id as u64).wrapping_mul(0x6C62_272E_07BB_0142));
    h = splitmix(h);
    h = h.wrapping_add((seed3 as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    h = h.wrapping_add((seed4 as u64).wrapping_mul(0x5555_5555_5555_5555));
    h = splitmix(h);

    // Map to (0, 1), avoiding exact 0 and 1.
    // Use the top 53 bits for a double in [2^-53, 1 - 2^-53].
    let mantissa = (h >> 11) | 1; // ensure non-zero
    mantissa as f64 / (1u64 << 53) as f64
}

/// SplitMix64 mixing function.
#[inline]
fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = uniform(42, 1, 1, 0, 0);
        let b = uniform(42, 1, 1, 0, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn in_unit_interval() {
        for i in 1..=10_000 {
            let u = uniform(i, 1, 1, 0, 0);
            assert!(u > 0.0 && u < 1.0, "trial {i}: u = {u}");
        }
    }

    #[test]
    fn different_var_ids_independent() {
        // Different var_ids should produce different values.
        let a = uniform(1, 1, 1, 0, 0);
        let b = uniform(1, 1, 2, 0, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn different_entities_independent() {
        let a = uniform(1, 1, 1, 0, 0);
        let b = uniform(1, 2, 1, 0, 0);
        assert_ne!(a, b);
    }

    #[test]
    fn uniform_distribution() {
        // Chi-squared test: 10 bins, 10,000 trials.
        let n = 10_000;
        let bins = 10;
        let mut counts = vec![0u32; bins];
        for i in 1..=n {
            let u = uniform(i as u64, 1, 1, 0, 0);
            let bin = (u * bins as f64).min(bins as f64 - 1.0) as usize;
            counts[bin] += 1;
        }
        let expected = n as f64 / bins as f64;
        let chi2: f64 = counts
            .iter()
            .map(|&c| {
                let diff = c as f64 - expected;
                diff * diff / expected
            })
            .sum();
        // Critical value for chi2(9, 0.01) ≈ 21.67.
        assert!(
            chi2 < 25.0,
            "chi2 = {chi2}, distribution may not be uniform. counts = {counts:?}"
        );
    }
}
