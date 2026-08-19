//! SIPmath 3.0 JSON serialization and deserialization.
//!
//! The SIPmath 3.0 standard stores distributions as metalog coefficients plus
//! HDR seeds, allowing any client to regenerate the full trial array on demand.
//! This reduces a 10,000-trial SIP from ~80 KB to ~200 bytes.

use serde::{Deserialize, Serialize};

use crate::metalog::{Bounds, Metalog};

/// A SIPmath 3.0 library — a collection of named SIPs with shared RNG config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SipLibrary {
    pub library_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub sips: Vec<SipDef>,
    pub rng: Vec<RngDef>,
}

/// Definition of a single SIP variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SipDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<String>,
    pub function: String,
    pub arguments: SipArguments,
}

/// Arguments for a metalog SIP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SipArguments {
    pub a_coefficients: Vec<f64>,
    pub boundedness: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_bound: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upper_bound: Option<f64>,
}

/// RNG definition (HDR 2.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RngDef {
    pub function: String,
    pub arguments: RngArguments,
}

/// HDR 2.0 seed arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RngArguments {
    pub counter: String,
    pub entity: u32,
    pub var_id: u32,
    #[serde(default)]
    pub seed3: u32,
    #[serde(default)]
    pub seed4: u32,
}

impl SipLibrary {
    /// Create a new empty SIPmath 3.0 library.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            library_type: "SIPmath_3_0".to_string(),
            name: Some(name.into()),
            sips: Vec::new(),
            rng: Vec::new(),
        }
    }

    /// Add a metalog SIP to the library.
    pub fn add_sip(
        &mut self,
        name: impl Into<String>,
        metalog: &Metalog,
        entity: u32,
        var_id: u32,
    ) {
        let (boundedness, lower, upper) = match metalog.bounds {
            Bounds::Unbounded => ("u".to_string(), None, None),
            Bounds::SemiLower(lb) => ("sl".to_string(), Some(lb), None),
            Bounds::SemiUpper(ub) => ("su".to_string(), None, Some(ub)),
            Bounds::Bounded(lb, ub) => ("b".to_string(), Some(lb), Some(ub)),
        };

        self.sips.push(SipDef {
            name: name.into(),
            ref_id: None,
            function: "Metalog_1_0".to_string(),
            arguments: SipArguments {
                a_coefficients: metalog.coeffs.clone(),
                boundedness,
                lower_bound: lower,
                upper_bound: upper,
            },
        });

        self.rng.push(RngDef {
            function: "HDR_2_0".to_string(),
            arguments: RngArguments {
                counter: "PM_Index".to_string(),
                entity,
                var_id,
                seed3: 0,
                seed4: 0,
            },
        });
    }

    /// Serialize to SIPmath 3.0 JSON.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("SipLibrary is always serializable")
    }

    /// Deserialize from SIPmath 3.0 JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Convert a SipDef back to a Metalog.
impl SipDef {
    pub fn to_metalog(&self) -> Metalog {
        let bounds = match self.arguments.boundedness.as_str() {
            "sl" => Bounds::SemiLower(self.arguments.lower_bound.unwrap_or(0.0)),
            "su" => Bounds::SemiUpper(self.arguments.upper_bound.unwrap_or(0.0)),
            "b" => Bounds::Bounded(
                self.arguments.lower_bound.unwrap_or(0.0),
                self.arguments.upper_bound.unwrap_or(1.0),
            ),
            _ => Bounds::Unbounded,
        };
        Metalog {
            coeffs: self.arguments.a_coefficients.clone(),
            bounds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let severity = Metalog::fit_spt(1.0, 3.0, 5.0, Bounds::SemiLower(0.0)).unwrap();
        let effectiveness =
            Metalog::fit_spt(0.75, 0.90, 0.97, Bounds::Bounded(0.0, 1.0)).unwrap();

        let mut lib = SipLibrary::new("fall_from_scaffold");
        lib.add_sip("severity", &severity, 1, 1);
        lib.add_sip("net_effectiveness", &effectiveness, 1, 2);

        let json = lib.to_json();
        let parsed = SipLibrary::from_json(&json).unwrap();

        assert_eq!(parsed.sips.len(), 2);
        assert_eq!(parsed.sips[0].name, "severity");
        assert_eq!(parsed.sips[1].name, "net_effectiveness");
        assert_eq!(parsed.sips[0].arguments.boundedness, "sl");
        assert_eq!(parsed.sips[1].arguments.boundedness, "b");

        // Reconstruct metalogs and check quantiles match.
        let sev2 = parsed.sips[0].to_metalog();
        assert!((sev2.quantile(0.50) - 3.0).abs() < 1e-6);

        let eff2 = parsed.sips[1].to_metalog();
        assert!((eff2.quantile(0.50) - 0.90).abs() < 1e-6);
    }

    #[test]
    fn json_format() {
        let m = Metalog::fit_spt(2.0, 5.0, 8.0, Bounds::Unbounded).unwrap();
        let mut lib = SipLibrary::new("test");
        lib.add_sip("x", &m, 1, 1);
        let json = lib.to_json();
        assert!(json.contains("SIPmath_3_0"));
        assert!(json.contains("Metalog_1_0"));
        assert!(json.contains("HDR_2_0"));
        assert!(json.contains("PM_Index"));
    }
}
