//! SIP (Stochastic Information Packet) generation and composition.
//!
//! A SIP is an array of Monte Carlo trials from a metalog distribution.
//! SIPs compose via element-wise arithmetic — distributions that work like numbers.

use crate::hdr;
use crate::metalog::Metalog;

/// Generate N Monte Carlo trials from a metalog distribution using HDR seeds.
pub fn generate(metalog: &Metalog, n: usize, entity: u32, var_id: u32) -> Vec<f64> {
    (1..=n as u64)
        .map(|i| {
            let u = hdr::uniform(i, entity, var_id, 0, 0);
            metalog.quantile(u)
        })
        .collect()
}

/// Element-wise multiplicative mitigation chain.
///
/// `residual[i] = severity[i] * ∏(1 - effectiveness_j[i])`
///
/// Each mitigation effectiveness array reduces the severity. If a mitigation's
/// effectiveness is 0 for trial i (e.g., barrier not activated), it has no effect
/// and remaining barriers face the full residual severity.
pub fn chain_mitigations(severity: &[f64], mitigations: &[&[f64]]) -> Vec<f64> {
    let n = severity.len();
    let mut residual = severity.to_vec();
    for eff in mitigations {
        assert_eq!(eff.len(), n, "mitigation array length must match severity");
        for i in 0..n {
            residual[i] *= 1.0 - eff[i];
        }
    }
    residual
}

/// Apply a Bernoulli gate to an effectiveness SIP.
///
/// For each trial, the barrier is active with probability `p_active`.
/// When inactive, effectiveness is 0 (barrier not present).
/// When active, effectiveness is drawn from the metalog.
pub fn gated_effectiveness(
    metalog: &Metalog,
    p_active: f64,
    n: usize,
    entity: u32,
    var_id_gate: u32,
    var_id_eff: u32,
) -> Vec<f64> {
    (1..=n as u64)
        .map(|i| {
            let gate = hdr::uniform(i, entity, var_id_gate, 0, 0);
            if gate < p_active {
                let u = hdr::uniform(i, entity, var_id_eff, 0, 0);
                metalog.quantile(u)
            } else {
                0.0
            }
        })
        .collect()
}

/// P(X ≥ threshold) — exceedance probability from a trial array.
pub fn exceedance_probability(trials: &[f64], threshold: f64) -> f64 {
    let count = trials.iter().filter(|&&x| x >= threshold).count();
    count as f64 / trials.len() as f64
}

/// Compute a percentile from a trial array.
///
/// `p` is in [0, 1]. Uses linear interpolation between adjacent sorted values.
pub fn percentile(trials: &[f64], p: f64) -> f64 {
    assert!(!trials.is_empty(), "cannot compute percentile of empty array");
    assert!(
        (0.0..=1.0).contains(&p),
        "percentile must be in [0, 1], got {p}"
    );

    let mut sorted = trials.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }

    let idx = p * (n - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    let frac = idx - lo as f64;

    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

/// Summary statistics for a trial array.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Summary {
    pub mean: f64,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    pub min: f64,
    pub max: f64,
}

/// Compute summary statistics for a trial array.
pub fn summarise(trials: &[f64]) -> Summary {
    let n = trials.len() as f64;
    let mean = trials.iter().sum::<f64>() / n;
    Summary {
        mean,
        p10: percentile(trials, 0.10),
        p50: percentile(trials, 0.50),
        p90: percentile(trials, 0.90),
        min: trials.iter().cloned().fold(f64::INFINITY, f64::min),
        max: trials.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metalog::Bounds;

    #[test]
    fn generate_correct_count() {
        let m = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        let trials = generate(&m, 10_000, 1, 1);
        assert_eq!(trials.len(), 10_000);
    }

    #[test]
    fn generate_deterministic() {
        let m = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        let a = generate(&m, 100, 1, 1);
        let b = generate(&m, 100, 1, 1);
        assert_eq!(a, b);
    }

    #[test]
    fn chain_mitigations_reduces_severity() {
        let severity_m = Metalog::fit_spt(1.0, 3.0, 5.0, Bounds::SemiLower(0.0)).unwrap();
        let net_m = Metalog::fit_spt(0.75, 0.90, 0.97, Bounds::Bounded(0.0, 1.0)).unwrap();

        let n = 10_000;
        let severity = generate(&severity_m, n, 1, 1);
        let net_eff = generate(&net_m, n, 1, 2);

        let residual = chain_mitigations(&severity, &[&net_eff]);

        let sev_mean = severity.iter().sum::<f64>() / n as f64;
        let res_mean = residual.iter().sum::<f64>() / n as f64;

        // Residual should be ~10% of severity (90% effective net).
        assert!(res_mean < sev_mean * 0.20, "residual mean {res_mean} not much less than severity mean {sev_mean}");
        assert!(res_mean > 0.0);
    }

    #[test]
    fn gated_effectiveness_respects_gate() {
        let eff_m = Metalog::fit_spt(0.80, 0.90, 0.97, Bounds::Bounded(0.0, 1.0)).unwrap();
        let n = 10_000;

        // Gate probability 0.0 → all zeros.
        let eff = gated_effectiveness(&eff_m, 0.0, n, 1, 10, 11);
        assert!(eff.iter().all(|&e| e == 0.0));

        // Gate probability 1.0 → all non-zero.
        let eff = gated_effectiveness(&eff_m, 1.0, n, 1, 10, 11);
        assert!(eff.iter().all(|&e| e > 0.0));

        // Gate probability 0.5 → roughly half zero.
        let eff = gated_effectiveness(&eff_m, 0.5, n, 1, 10, 11);
        let zero_count = eff.iter().filter(|&&e| e == 0.0).count();
        let frac = zero_count as f64 / n as f64;
        assert!(frac > 0.40 && frac < 0.60, "zero fraction = {frac}");
    }

    #[test]
    fn exceedance_basic() {
        let trials = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((exceedance_probability(&trials, 3.0) - 0.6).abs() < 1e-10);
        assert!((exceedance_probability(&trials, 6.0) - 0.0).abs() < 1e-10);
        assert!((exceedance_probability(&trials, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn percentile_basic() {
        let trials = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&trials, 0.0) - 1.0).abs() < 1e-10);
        assert!((percentile(&trials, 0.5) - 3.0).abs() < 1e-10);
        assert!((percentile(&trials, 1.0) - 5.0).abs() < 1e-10);
    }
}
