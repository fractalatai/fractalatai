//! Law-level significance (Approach L + K profile).
//!
//! Rolls provision-level `significance_overall` (HIGH/MEDIUM/LOW, Approach B)
//! up to one rating per law, as designed in session
//! `cascade/07-01-26-significance-publish.md`:
//!
//! - score = avg_sig × log2(total + 1), avg_sig with HIGH=3, MEDIUM=2, LOW=1
//! - rating by the July 2026 percentile cut (top 20% HIGH, bottom 33% LOW),
//!   frozen as fixed score thresholds so existing ratings don't shift as laws
//!   are added (fractalatai #55)
//! - K profile = the high/medium/low/total counts

/// Scores at or below this are LOW. The July 2026 corpus had LOW ≤ 6.126 and MEDIUM ≥ 6.158.
pub const LOW_MAX_SCORE: f64 = 6.14;
/// Scores at or above this are HIGH. The July 2026 corpus had MEDIUM ≤ 11.017 and HIGH ≥ 11.103.
pub const HIGH_MIN_SCORE: f64 = 11.06;

#[derive(Debug, Clone, PartialEq)]
pub struct LawSignificance {
    pub rating: &'static str,
    pub score: f64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub total: i64,
}

/// Law-level significance from provision counts. `None` when no provision is rated.
pub fn law_significance(high: i64, medium: i64, low: i64) -> Option<LawSignificance> {
    let total = high + medium + low;
    if total <= 0 {
        return None;
    }
    let avg = (3 * high + 2 * medium + low) as f64 / total as f64;
    let score = avg * ((total + 1) as f64).log2();
    let rating = if score >= HIGH_MIN_SCORE {
        "HIGH"
    } else if score <= LOW_MAX_SCORE {
        "LOW"
    } else {
        "MEDIUM"
    };
    Some(LawSignificance { rating, score, high, medium, low, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.001
    }

    #[test]
    fn reproduces_july_values() {
        // Stored July values in DuckDB `legislation`
        let hswa = law_significance(31, 48, 93).unwrap(); // UK_ukpga_1974_37
        assert!(close(hswa.score, 12.189), "{}", hswa.score);
        assert_eq!(hswa.rating, "HIGH");
        assert_eq!(hswa.total, 172);

        let cdm = law_significance(87, 41, 39).unwrap(); // UK_uksi_2015_51
        assert!(close(cdm.score, 16.909), "{}", cdm.score);
        assert_eq!(cdm.rating, "HIGH");

        let ppp = law_significance(17, 19, 38).unwrap(); // UK_uksi_2012_1657
        assert!(close(ppp.score, 10.690), "{}", ppp.score);
        assert_eq!(ppp.rating, "MEDIUM");

        let mhsw = law_significance(23, 26, 4).unwrap(); // UK_uksi_1999_3242
        assert!(close(mhsw.score, 13.573), "{}", mhsw.score);
        assert_eq!(mhsw.rating, "HIGH");
    }

    #[test]
    fn thresholds_sit_between_july_bands() {
        assert!(LOW_MAX_SCORE > 6.126 && LOW_MAX_SCORE < 6.158);
        assert!(HIGH_MIN_SCORE > 11.017 && HIGH_MIN_SCORE < 11.103);
    }

    #[test]
    fn small_law_is_low() {
        // One LOW obligation: 1 × log2(2) = 1.0
        let s = law_significance(0, 0, 1).unwrap();
        assert!(close(s.score, 1.0));
        assert_eq!(s.rating, "LOW");
    }

    #[test]
    fn no_rated_provisions_is_none() {
        assert!(law_significance(0, 0, 0).is_none());
    }
}
