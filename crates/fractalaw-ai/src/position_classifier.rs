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
/// Logistic regression with 4 classes and variable features:
/// v2: embedding(384) + modal(13) + drrp(3) + category(10) + offset(1) = 411
/// v3: + dep_parsing(7) + section_type(10) = 428
pub struct PositionClassifier {
    coef: Vec<Vec<f32>>,
    intercept: Vec<f32>,
    classes: Vec<String>,
    n_features: usize,
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

        let n_features = coef.first().map(|c| c.len()).unwrap_or(0);

        Ok(Self {
            coef,
            intercept,
            classes,
            n_features,
        })
    }

    /// Number of features this classifier expects.
    pub fn n_features(&self) -> usize {
        self.n_features
    }

    /// Predict position for a single (provision, actor) pair.
    pub fn predict(&self, features: &[f32]) -> PositionPrediction {
        assert_eq!(
            features.len(),
            self.n_features,
            "expected {} features, got {}",
            self.n_features,
            features.len()
        );

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

/// Section type labels for one-hot encoding (must match training order).
const SECTION_TYPES: &[&str] = &[
    "article", "sub_article", "section", "sub_section", "schedule",
    "part", "chapter", "heading", "table", "other",
];

/// Dep parsing features (7 floats, from provision_actors columns).
/// Order: is_subject, is_object, is_agent, is_attr, voice_passive, has_modal, verb_distance.
pub type DepFeatures = [f32; 7];

/// Build the feature vector for a (provision, actor) pair.
///
/// v2 (411 dims): embedding(384) + modal(13) + drrp(3) + category(10) + offset(1)
/// v3 (428 dims): + dep_parsing(7) + section_type(10)
pub fn build_position_features(
    embedding: &[f32],
    modal_features: &[f32; 13],
    drrp_types: &[String],
    actor_label: &str,
    actor_text_offset: f32,
    dep_features: Option<&DepFeatures>,
    section_type: Option<&str>,
) -> Vec<f32> {
    let capacity = if dep_features.is_some() { 428 } else { 411 };
    let mut features = Vec::with_capacity(capacity);

    // Embedding (384)
    features.extend_from_slice(embedding);

    // Modal features (13)
    features.extend_from_slice(modal_features);

    // DRRP one-hot (3)
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

    // Dep parsing features (7) — optional for v3
    if let Some(dep) = dep_features {
        features.extend_from_slice(dep);
    }

    // Section type one-hot (10) — optional for v3
    if dep_features.is_some() {
        let st = section_type.unwrap_or("other");
        for s in SECTION_TYPES {
            features.push(if *s == st { 1.0 } else { 0.0 });
        }
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_features_v2_length() {
        let embedding = vec![0.1f32; 384];
        let modals = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let drrp = vec!["Obligation".to_string()];
        let features = build_position_features(&embedding, &modals, &drrp, "Org: Employer", 0.1, None, None);
        assert_eq!(features.len(), 411);
    }

    #[test]
    fn build_features_v3_length() {
        let embedding = vec![0.1f32; 384];
        let modals = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let drrp = vec!["Obligation".to_string()];
        let dep: DepFeatures = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.3];
        let features = build_position_features(&embedding, &modals, &drrp, "Org: Employer", 0.1, Some(&dep), Some("section"));
        assert_eq!(features.len(), 428);
    }

    #[test]
    fn category_encoding() {
        let embedding = vec![0.0f32; 384];
        let modals = [0.0; 13];
        let drrp = vec![];

        let f_org = build_position_features(&embedding, &modals, &drrp, "Org: Employer", 0.0, None, None);
        let f_gvt = build_position_features(&embedding, &modals, &drrp, "Gvt: Minister", 0.0, None, None);

        // Category one-hot starts at index 384+13+3 = 400
        assert_eq!(f_org[400], 1.0); // Org
        assert_eq!(f_org[402], 0.0); // Gvt
        assert_eq!(f_gvt[400], 0.0); // Org
        assert_eq!(f_gvt[404], 1.0); // Gvt
    }
}
