//! Metalog distribution fitting and evaluation.
//!
//! The metalog (Keelin, 2016) is defined by its quantile function (inverse CDF).
//! Given cumulative probability y ∈ (0,1), the k-term metalog returns a value x:
//!
//!   M_k(y) = Σ a_i · g_i(y)
//!
//! where g_i are basis functions built from the logit ln(y/(1-y)) and polynomials
//! in (y - 0.5). The coefficients a_i are fitted from quantile data via OLS or,
//! for the 3-term SPT case, via closed-form formulas.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Bounds {
    /// Support on (-∞, +∞).
    Unbounded,
    /// Support on [lower, +∞).
    SemiLower(f64),
    /// Support on (-∞, upper].
    SemiUpper(f64),
    /// Support on [lower, upper].
    Bounded(f64, f64),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Metalog {
    /// Coefficients a_1, a_2, ..., a_k.
    pub coeffs: Vec<f64>,
    /// Boundedness constraint.
    pub bounds: Bounds,
}

#[derive(Debug, Error)]
pub enum MetalogError {
    #[error("need at least 2 quantile points, got {0}")]
    TooFewPoints(usize),
    #[error("requested {terms} terms but only {points} data points")]
    MoreTermsThanPoints { terms: usize, points: usize },
    #[error("quantile probabilities must be strictly between 0 and 1")]
    ProbabilityOutOfRange,
    #[error("quantile probabilities must be strictly increasing")]
    ProbabilitiesNotIncreasing,
    #[error("quantile values must be strictly increasing")]
    ValuesNotIncreasing,
    #[error("bounded metalog requires lower < upper, got [{0}, {1}]")]
    InvalidBounds(f64, f64),
    #[error("OLS system is singular — cannot fit metalog")]
    SingularSystem,
    #[error("metalog is infeasible — quantile function is not monotonically increasing")]
    Infeasible,
}

/// Logit function: ln(y / (1 - y)).
#[inline]
fn logit(y: f64) -> f64 {
    (y / (1.0 - y)).ln()
}

/// Evaluate the k metalog basis functions at cumulative probability y.
fn basis(y: f64, k: usize) -> Vec<f64> {
    let mut g = Vec::with_capacity(k);
    if k == 0 {
        return g;
    }
    // g_1 = 1
    g.push(1.0);
    if k == 1 {
        return g;
    }
    let l = logit(y);
    // g_2 = ln(y/(1-y))
    g.push(l);
    if k == 2 {
        return g;
    }
    let d = y - 0.5;
    // g_3 = (y - 0.5) · ln(y/(1-y))
    g.push(d * l);
    if k == 3 {
        return g;
    }
    // g_4 = (y - 0.5)
    g.push(d);
    // For k >= 5, alternate:
    //   odd i:  g_i = (y - 0.5)^((i-1)/2)
    //   even i: g_i = (y - 0.5)^((i-2)/2) · ln(y/(1-y))
    let mut power = 2u32;
    for i in 5..=k {
        let dp = d.powi(power as i32);
        if i % 2 == 1 {
            g.push(dp);
        } else {
            g.push(dp * l);
            power += 1;
        }
    }
    g
}

/// Evaluate the derivative of the k metalog basis functions at y.
/// These are dg_i/dy, used for PDF computation and feasibility checking.
fn basis_deriv(y: f64, k: usize) -> Vec<f64> {
    let mut dg = Vec::with_capacity(k);
    if k == 0 {
        return dg;
    }
    // dg_1/dy = 0
    dg.push(0.0);
    if k == 1 {
        return dg;
    }
    let inv = 1.0 / (y * (1.0 - y));
    let l = logit(y);
    // dg_2/dy = 1 / (y(1-y))
    dg.push(inv);
    if k == 2 {
        return dg;
    }
    let d = y - 0.5;
    // dg_3/dy = (y-0.5)/(y(1-y)) + ln(y/(1-y))
    dg.push(d * inv + l);
    if k == 3 {
        return dg;
    }
    // dg_4/dy = 1
    dg.push(1.0);
    let mut power = 2u32;
    for i in 5..=k {
        let p = power as i32;
        if i % 2 == 1 {
            // dg_i/dy = p · (y-0.5)^(p-1)
            dg.push(p as f64 * d.powi(p - 1));
        } else {
            // dg_i/dy = p · (y-0.5)^(p-1) · ln(y/(1-y)) + (y-0.5)^p / (y(1-y))
            dg.push(p as f64 * d.powi(p - 1) * l + d.powi(p) * inv);
            power += 1;
        }
    }
    dg
}

/// Apply the log-transform for bounded variants.
fn transform(x: f64, bounds: Bounds) -> f64 {
    match bounds {
        Bounds::Unbounded => x,
        Bounds::SemiLower(lb) => (x - lb).ln(),
        Bounds::SemiUpper(ub) => -(ub - x).ln(),
        Bounds::Bounded(lb, ub) => ((x - lb) / (ub - x)).ln(),
    }
}

/// Inverse of the log-transform.
fn inv_transform(z: f64, bounds: Bounds) -> f64 {
    match bounds {
        Bounds::Unbounded => z,
        Bounds::SemiLower(lb) => lb + z.exp(),
        Bounds::SemiUpper(ub) => ub - (-z).exp(),
        Bounds::Bounded(lb, ub) => {
            let e = z.exp();
            (lb + ub * e) / (1.0 + e)
        }
    }
}

/// Derivative of the inverse transform (for PDF).
fn inv_transform_deriv(z: f64, bounds: Bounds) -> f64 {
    match bounds {
        Bounds::Unbounded => 1.0,
        Bounds::SemiLower(_) => z.exp(),
        Bounds::SemiUpper(_) => (-z).exp(),
        Bounds::Bounded(lb, ub) => {
            let e = z.exp();
            (ub - lb) * e / ((1.0 + e) * (1.0 + e))
        }
    }
}

impl Metalog {
    /// Fit a 3-term metalog from a Symmetric Percentile Triplet (P10, P50, P90).
    ///
    /// Closed-form — no OLS, no iteration.
    pub fn fit_spt(x10: f64, x50: f64, x90: f64, bounds: Bounds) -> Result<Self, MetalogError> {
        if let Bounds::Bounded(lb, ub) = bounds {
            if lb >= ub {
                return Err(MetalogError::InvalidBounds(lb, ub));
            }
        }

        // Transform data for bounded variants.
        let z10 = transform(x10, bounds);
        let z50 = transform(x50, bounds);
        let z90 = transform(x90, bounds);

        // Logit values at y=0.10 and y=0.90.
        let l10 = logit(0.10); // ≈ -2.197
        let l90 = logit(0.90); // ≈  2.197
        let delta_l = l90 - l10; // ≈ 4.394

        // Closed-form coefficients.
        // From the 3-term system at y=0.10, 0.50, 0.90:
        //   a1 = z50  (since g2(0.5)=0 and g3(0.5)=0)
        //   a2 = (z90 - z10) / (L90 - L10)
        //   a3 = (z90 + z10 - 2*z50) / (2 * (0.9-0.5) * L90)
        let a1 = z50;
        let a2 = (z90 - z10) / delta_l;
        let a3_denom = 2.0 * (0.90 - 0.50) * l90; // = 2 * 0.4 * 2.197 ≈ 1.758
        let a3 = (z90 + z10 - 2.0 * z50) / a3_denom;

        let m = Self {
            coeffs: vec![a1, a2, a3],
            bounds,
        };

        if !m.is_feasible() {
            return Err(MetalogError::Infeasible);
        }

        Ok(m)
    }

    /// Fit a k-term metalog from n quantile data points via OLS.
    ///
    /// `quantiles` is a slice of (cumulative_probability, value) pairs,
    /// sorted by probability. Probabilities must be in (0, 1).
    pub fn fit_ols(
        quantiles: &[(f64, f64)],
        terms: usize,
        bounds: Bounds,
    ) -> Result<Self, MetalogError> {
        let n = quantiles.len();
        if n < 2 {
            return Err(MetalogError::TooFewPoints(n));
        }
        if terms > n {
            return Err(MetalogError::MoreTermsThanPoints {
                terms,
                points: n,
            });
        }
        if let Bounds::Bounded(lb, ub) = bounds {
            if lb >= ub {
                return Err(MetalogError::InvalidBounds(lb, ub));
            }
        }

        // Validate probabilities.
        for (i, &(y, _)) in quantiles.iter().enumerate() {
            if y <= 0.0 || y >= 1.0 {
                return Err(MetalogError::ProbabilityOutOfRange);
            }
            if i > 0 && y <= quantiles[i - 1].0 {
                return Err(MetalogError::ProbabilitiesNotIncreasing);
            }
        }

        // Transform values for bounded variants.
        let z: Vec<f64> = quantiles.iter().map(|&(_, x)| transform(x, bounds)).collect();

        // Validate transformed values are increasing.
        for i in 1..z.len() {
            if z[i] <= z[i - 1] {
                return Err(MetalogError::ValuesNotIncreasing);
            }
        }

        // Build design matrix Y (n × k) and solve Y^T Y a = Y^T z.
        let k = terms;

        // Y^T Y (k × k)
        let mut yty = vec![0.0f64; k * k];
        // Y^T z (k × 1)
        let mut ytz = vec![0.0f64; k];

        for (j, &(y, _)) in quantiles.iter().enumerate() {
            let g = basis(y, k);
            for r in 0..k {
                ytz[r] += g[r] * z[j];
                for c in 0..k {
                    yty[r * k + c] += g[r] * g[c];
                }
            }
        }

        // Solve via Gaussian elimination with partial pivoting.
        let coeffs = gauss_solve(k, &mut yty, &mut ytz)?;

        let m = Self { coeffs, bounds };

        if !m.is_feasible() {
            return Err(MetalogError::Infeasible);
        }

        Ok(m)
    }

    /// Number of terms in this metalog.
    pub fn terms(&self) -> usize {
        self.coeffs.len()
    }

    /// Evaluate the quantile function: cumulative probability y ∈ (0,1) → value x.
    ///
    /// This is the core operation for Monte Carlo: generate u ~ Uniform(0,1),
    /// then x = quantile(u).
    pub fn quantile(&self, y: f64) -> f64 {
        debug_assert!(y > 0.0 && y < 1.0, "y must be in (0, 1), got {y}");
        let g = basis(y, self.coeffs.len());
        let z: f64 = self.coeffs.iter().zip(g.iter()).map(|(a, g)| a * g).sum();
        inv_transform(z, self.bounds)
    }

    /// Numerical CDF: value x → cumulative probability P(X ≤ x).
    ///
    /// Uses bisection on the quantile function. Converges to ~1e-12 precision
    /// in ~40 iterations.
    pub fn cdf(&self, x: f64) -> f64 {
        let mut lo = 1e-15_f64;
        let mut hi = 1.0 - 1e-15;

        // Check bounds.
        if self.quantile(lo) >= x {
            return 0.0;
        }
        if self.quantile(hi) <= x {
            return 1.0;
        }

        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if self.quantile(mid) < x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) / 2.0
    }

    /// Check feasibility: the quantile function must be monotonically increasing.
    ///
    /// Tests at 100 points in (0, 1). For k ≤ 3, analytical feasibility could
    /// be checked, but the numerical approach is simple and general.
    pub fn is_feasible(&self) -> bool {
        let k = self.coeffs.len();
        let n_check = 200;
        for i in 1..n_check {
            let y = i as f64 / n_check as f64;
            let dg = basis_deriv(y, k);
            let deriv: f64 = self.coeffs.iter().zip(dg.iter()).map(|(a, d)| a * d).sum();
            // The derivative of the quantile function (in the transformed space)
            // must be positive for monotonicity.
            if deriv <= 0.0 {
                return false;
            }
        }
        true
    }

    /// Evaluate the PDF at value x.
    ///
    /// f(x) = 1 / (M'(y) · dT⁻¹/dz) where y = CDF(x).
    pub fn pdf(&self, x: f64) -> f64 {
        let y = self.cdf(x);
        if y <= 0.0 || y >= 1.0 {
            return 0.0;
        }
        let k = self.coeffs.len();
        let dg = basis_deriv(y, k);
        let m_prime: f64 = self.coeffs.iter().zip(dg.iter()).map(|(a, d)| a * d).sum();
        if m_prime <= 0.0 {
            return 0.0;
        }

        let g = basis(y, k);
        let z: f64 = self.coeffs.iter().zip(g.iter()).map(|(a, g)| a * g).sum();
        let dt_inv = inv_transform_deriv(z, self.bounds);

        1.0 / (m_prime * dt_inv)
    }
}

/// Gaussian elimination with partial pivoting for k×k system.
fn gauss_solve(k: usize, a: &mut [f64], b: &mut [f64]) -> Result<Vec<f64>, MetalogError> {
    // Forward elimination with partial pivoting.
    for col in 0..k {
        // Find pivot.
        let mut max_val = a[col * k + col].abs();
        let mut max_row = col;
        for row in (col + 1)..k {
            let v = a[row * k + col].abs();
            if v > max_val {
                max_val = v;
                max_row = row;
            }
        }
        if max_val < 1e-14 {
            return Err(MetalogError::SingularSystem);
        }

        // Swap rows.
        if max_row != col {
            for c in 0..k {
                a.swap(col * k + c, max_row * k + c);
            }
            b.swap(col, max_row);
        }

        // Eliminate below.
        let pivot = a[col * k + col];
        for row in (col + 1)..k {
            let factor = a[row * k + col] / pivot;
            for c in col..k {
                a[row * k + c] -= factor * a[col * k + c];
            }
            b[row] -= factor * b[col];
        }
    }

    // Back substitution.
    let mut x = vec![0.0; k];
    for row in (0..k).rev() {
        let mut sum = b[row];
        for c in (row + 1)..k {
            sum -= a[row * k + c] * x[c];
        }
        x[row] = sum / a[row * k + row];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spt_symmetric() {
        // Symmetric distribution: P10=2, P50=5, P90=8
        let m = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        assert_eq!(m.terms(), 3);
        // a1 = median
        assert!((m.coeffs[0] - 5.0).abs() < 1e-10);
        // a3 should be ~0 for symmetric input
        assert!(m.coeffs[2].abs() < 1e-10);
        // Quantile at 0.5 = median
        assert!((m.quantile(0.5) - 5.0).abs() < 1e-10);
        // Quantile at 0.10 ≈ 2.0
        assert!((m.quantile(0.10) - 2.0).abs() < 1e-8);
        // Quantile at 0.90 ≈ 8.0
        assert!((m.quantile(0.90) - 8.0).abs() < 1e-8);
    }

    #[test]
    fn spt_skewed() {
        // Right-skewed: P10=1, P50=3, P90=10
        let m = Metalog::fit_spt(1.0, 3.0, 10.0, Bounds::Unbounded).unwrap();
        assert!((m.quantile(0.10) - 1.0).abs() < 1e-8);
        assert!((m.quantile(0.50) - 3.0).abs() < 1e-8);
        assert!((m.quantile(0.90) - 10.0).abs() < 1e-8);
        // a3 > 0 for right-skew
        assert!(m.coeffs[2] > 0.0);
    }

    #[test]
    fn spt_bounded() {
        // Effectiveness distribution: bounded [0, 1], P10=0.75, P50=0.90, P90=0.97
        let m = Metalog::fit_spt(0.75, 0.90, 0.97, Bounds::Bounded(0.0, 1.0)).unwrap();
        let q10 = m.quantile(0.10);
        let q50 = m.quantile(0.50);
        let q90 = m.quantile(0.90);
        assert!((q10 - 0.75).abs() < 1e-6);
        assert!((q50 - 0.90).abs() < 1e-6);
        assert!((q90 - 0.97).abs() < 1e-6);
        // Values stay in bounds.
        assert!(m.quantile(0.01) >= 0.0);
        assert!(m.quantile(0.99) <= 1.0);
    }

    #[test]
    fn spt_semi_bounded() {
        // Severity: semi-bounded [0, ∞), P10=0.5, P50=2.0, P90=4.5
        let m = Metalog::fit_spt(0.5, 2.0, 4.5, Bounds::SemiLower(0.0)).unwrap();
        assert!((m.quantile(0.10) - 0.5).abs() < 1e-6);
        assert!((m.quantile(0.50) - 2.0).abs() < 1e-6);
        assert!((m.quantile(0.90) - 4.5).abs() < 1e-6);
        assert!(m.quantile(0.001) >= 0.0);
    }

    #[test]
    fn ols_matches_spt() {
        // OLS with 3 points should match SPT.
        let quantiles = vec![(0.10, 2.0), (0.50, 5.0), (0.90, 8.0)];
        let ols = Metalog::fit_ols(&quantiles, 3, Bounds::Unbounded).unwrap();
        let spt = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        for (a, b) in ols.coeffs.iter().zip(spt.coeffs.iter()) {
            assert!((a - b).abs() < 1e-8, "OLS {a} != SPT {b}");
        }
    }

    #[test]
    fn ols_five_term() {
        let quantiles = vec![
            (0.05, 1.0),
            (0.25, 3.0),
            (0.50, 5.0),
            (0.75, 7.5),
            (0.95, 12.0),
        ];
        let m = Metalog::fit_ols(&quantiles, 5, Bounds::Unbounded).unwrap();
        assert_eq!(m.terms(), 5);
        // Should interpolate the data points.
        for &(y, x) in &quantiles {
            assert!(
                (m.quantile(y) - x).abs() < 0.1,
                "quantile({y}) = {} ≠ {x}",
                m.quantile(y)
            );
        }
    }

    #[test]
    fn cdf_roundtrip() {
        let m = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        for &y in &[0.05, 0.10, 0.25, 0.50, 0.75, 0.90, 0.95] {
            let x = m.quantile(y);
            let y_back = m.cdf(x);
            assert!(
                (y - y_back).abs() < 1e-10,
                "CDF roundtrip failed: y={y}, x={x}, cdf(x)={y_back}"
            );
        }
    }

    #[test]
    fn pdf_integrates() {
        // Rough numerical integration of PDF should ≈ 1.
        let m = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        let lo = m.quantile(0.001);
        let hi = m.quantile(0.999);
        let n = 1000;
        let dx = (hi - lo) / n as f64;
        let integral: f64 = (0..n).map(|i| m.pdf(lo + (i as f64 + 0.5) * dx) * dx).sum();
        assert!(
            (integral - 0.998).abs() < 0.02,
            "PDF integral = {integral}, expected ≈ 0.998"
        );
    }
}
