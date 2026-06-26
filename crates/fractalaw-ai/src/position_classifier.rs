//! Position classifier for per-actor Hohfeldian position prediction.
//!
//! Predicts whether an actor in a provision is active (duty-bearer),
//! counterparty (claim-holder), or other (beneficiary/mentioned).
//! Input: 411-dim feature vector per (provision, actor) pair.
//! Output: active / counterparty / other.

use std::path::Path;

use tracing::info;

/// Predicted position for an actor in a provision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionClass {
    Active,
    Counterparty,
    Beneficiary,
    Mentioned,
}

impl PositionClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Counterparty => "counterparty",
            Self::Beneficiary => "beneficiary",
            Self::Mentioned => "mentioned",
        }
    }
}

/// Per-actor position classifier.
///
/// Logistic regression with 3 classes and 411 features:
/// embedding(384) + modal(13) + drrp(5) + category(10) + offset(1).
pub struct PositionClassifier {
    coef: Vec<Vec<f32>>,
    intercept: Vec<f32>,
    classes: Vec<String>,
}

/// Position prediction result.
pub struct PositionPrediction {
    pub class: PositionClass,
    pub confidence: f32,
}

/// Actor category labels for one-hot encoding (must match training order).
const CATEGORIES: &[&str] = &[
    "Org", "Ind", "Gvt", "SC", "Spc", "EU", "Svc", "Public", "Offshore", "other",
];

/// DRRP type labels for one-hot encoding (must match training order).
const DRRP_TYPES: &[&str] = &["Obligation", "Liberty", "none"];

impl PositionClassifier {
    /// Load classifier weights from a JSON file.
    pub fn load(weights_path: &Path) -> anyhow::Result<Self> {
        anyhow::ensure!(
            weights_path.exists(),
            "position classifier weights not found at {weights_path:?}"
        );

        let content = std::fs::read_to_string(weights_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let classes: Vec<String> = json["classes"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing 'classes'"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        let coef: Vec<Vec<f32>> = json["coef"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing 'coef'"))?
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
            .collect();

        let intercept: Vec<f32> = json["intercept"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing 'intercept'"))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        info!(
            path = %weights_path.display(),
            classes = ?classes,
            features = coef.first().map(|c| c.len()).unwrap_or(0),
            "loaded position classifier"
        );

        Ok(Self {
            coef,
            intercept,
            classes,
        })
    }

    /// Predict position for a single (provision, actor) pair.
    ///
    /// Feature vector must be 411 dimensions:
    /// embedding(384) + modal(13) + drrp(5) + category(10) + offset(1).
    pub fn predict(&self, features: &[f32]) -> PositionPrediction {
        assert_eq!(features.len(), 411, "expected 411 features");

        let n_classes = self.classes.len();
        let mut logits = vec![0.0f32; n_classes];
        for (i, (w, &b)) in self.coef.iter().zip(&self.intercept).enumerate() {
            logits[i] = w.iter().zip(features).map(|(wi, xi)| wi * xi).sum::<f32>() + b;
        }

        // Softmax
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp: Vec<f32> = logits.iter().map(|&z| (z - max_logit).exp()).collect();
        let sum: f32 = exp.iter().sum();
        let probs: Vec<f32> = exp.iter().map(|e| e / sum).collect();

        let (best_idx, &confidence) = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        let class = match self.classes[best_idx].as_str() {
            "active" => PositionClass::Active,
            "counterparty" => PositionClass::Counterparty,
            "beneficiary" => PositionClass::Beneficiary,
            _ => PositionClass::Mentioned,
        };

        PositionPrediction { class, confidence }
    }
}

/// Build the 411-dim feature vector for a (provision, actor) pair.
///
/// Combines provision embedding + modal features + DRRP type + actor category + text offset.
pub fn build_position_features(
    embedding: &[f32],
    modal_features: &[f32; 13],
    drrp_types: &[String],
    actor_label: &str,
    actor_text_offset: f32,
) -> Vec<f32> {
    let mut features = Vec::with_capacity(411);

    // Embedding (384)
    features.extend_from_slice(embedding);

    // Modal features (13)
    features.extend_from_slice(modal_features);

    // DRRP one-hot (5)
    for drrp in DRRP_TYPES {
        features.push(if drrp_types.iter().any(|d| d == drrp) {
            1.0
        } else {
            0.0
        });
    }

    // Actor category one-hot (10)
    let cat = if actor_label.contains(':') {
        actor_label.split(':').next().unwrap_or("other").trim()
    } else {
        "other"
    };
    for c in CATEGORIES {
        features.push(if *c == cat { 1.0 } else { 0.0 });
    }

    // Relative text offset (1)
    features.push(actor_text_offset);

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_features_correct_length() {
        let embedding = vec![0.1f32; 384];
        let modals = [
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let drrp = vec!["Duty".to_string()];
        let features = build_position_features(&embedding, &modals, &drrp, "Org: Employer", 0.1);
        assert_eq!(features.len(), 411);
    }

    #[test]
    fn category_encoding() {
        let embedding = vec![0.0f32; 384];
        let modals = [0.0; 13];
        let drrp = vec![];

        let f_org = build_position_features(&embedding, &modals, &drrp, "Org: Employer", 0.0);
        let f_gvt = build_position_features(&embedding, &modals, &drrp, "Gvt: Minister", 0.0);

        // Category one-hot starts at index 384+13+5 = 402
        assert_eq!(f_org[402], 1.0); // Org
        assert_eq!(f_org[404], 0.0); // Gvt
        assert_eq!(f_gvt[402], 0.0); // Org
        assert_eq!(f_gvt[404], 1.0); // Gvt
    }
}
