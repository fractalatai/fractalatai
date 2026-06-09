//! DRRP classifier for provision-level classification.
//!
//! Implements the logistic regression directly from exported weights,
//! avoiding complex ONNX output parsing for a trivial model.
//! Input: 397-dim feature vector (384 embedding + 13 modal indicators).
//! Output: Obligation / Liberty / none.

use std::path::Path;

use tracing::info;

/// DRRP class predicted by the classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrrpClass {
    Obligation,
    Liberty,
    None,
}

impl DrrpClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Obligation => "Obligation",
            Self::Liberty => "Liberty",
            Self::None => "none",
        }
    }
}

/// Provision-level DRRP classifier.
///
/// Logistic regression with 3 classes and 397 features.
/// Weights loaded from a JSON file exported from scikit-learn.
pub struct DrrpClassifier {
    /// Shape: [3, 397] — one row per class
    coef: Vec<Vec<f32>>,
    /// Shape: [3] — one per class
    intercept: Vec<f32>,
    /// Class labels in order: ["Liberty", "Obligation", "none"]
    classes: Vec<String>,
}

/// Classification result for a single provision.
pub struct DrrpPrediction {
    pub class: DrrpClass,
    pub confidence: f32,
}

impl DrrpClassifier {
    /// Load the classifier weights from a JSON file.
    ///
    /// Expected format:
    /// ```json
    /// {
    ///   "classes": ["Liberty", "Obligation", "none"],
    ///   "coef": [[...397 floats...], [...], [...]],
    ///   "intercept": [f, f, f]
    /// }
    /// ```
    pub fn load(weights_path: &Path) -> anyhow::Result<Self> {
        anyhow::ensure!(
            weights_path.exists(),
            "DRRP classifier weights not found at {weights_path:?}"
        );

        let content = std::fs::read_to_string(weights_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let classes: Vec<String> = json["classes"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing 'classes' array"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        let coef: Vec<Vec<f32>> = json["coef"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing 'coef' array"))?
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
            .ok_or_else(|| anyhow::anyhow!("missing 'intercept' array"))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        anyhow::ensure!(
            classes.len() == 3,
            "expected 3 classes, got {}",
            classes.len()
        );
        anyhow::ensure!(coef.len() == 3, "expected 3 coef rows, got {}", coef.len());
        anyhow::ensure!(
            coef[0].len() == 397,
            "expected 397 features, got {}",
            coef[0].len()
        );

        info!(
            path = %weights_path.display(),
            classes = ?classes,
            features = coef[0].len(),
            "loaded DRRP classifier"
        );

        Ok(Self {
            coef,
            intercept,
            classes,
        })
    }

    /// Classify a single provision from its 397-dim feature vector.
    pub fn predict(&self, features: &[f32]) -> DrrpPrediction {
        assert_eq!(features.len(), 397, "expected 397 features");

        // Compute logits: z_i = X @ W_i + b_i
        let mut logits = [0.0f32; 3];
        for (i, (w, &b)) in self.coef.iter().zip(&self.intercept).enumerate() {
            logits[i] = w.iter().zip(features).map(|(wi, xi)| wi * xi).sum::<f32>() + b;
        }

        // Softmax
        let max_logit = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp: Vec<f32> = logits.iter().map(|&z| (z - max_logit).exp()).collect();
        let sum: f32 = exp.iter().sum();
        let probs: Vec<f32> = exp.iter().map(|e| e / sum).collect();

        // Argmax
        let (best_idx, &confidence) = probs
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        let class = match self.classes[best_idx].as_str() {
            "Obligation" => DrrpClass::Obligation,
            "Liberty" => DrrpClass::Liberty,
            _ => DrrpClass::None,
        };

        DrrpPrediction { class, confidence }
    }

    /// Classify a batch of provisions.
    pub fn predict_batch(&self, features: &[Vec<f32>]) -> Vec<DrrpPrediction> {
        features.iter().map(|f| self.predict(f)).collect()
    }
}

/// Build modal features from provision text.
///
/// Returns 13 binary indicators matching the classifier's training feature order.
pub fn modal_features(text: &str) -> [f32; 13] {
    let t = text.to_lowercase();
    [
        if t.contains("shall") { 1.0 } else { 0.0 },
        if t.contains("must") { 1.0 } else { 0.0 },
        if t.contains(" may ") { 1.0 } else { 0.0 },
        if t.contains("requir") { 1.0 } else { 0.0 },
        if t.contains("ensur") { 1.0 } else { 0.0 },
        if t.contains("prohibit") { 1.0 } else { 0.0 },
        if word_match(&t, &["duty", "duties"]) {
            1.0
        } else {
            0.0
        },
        if word_match(&t, &["right", "rights"]) {
            1.0
        } else {
            0.0
        },
        if word_match(&t, &["power", "powers"]) {
            1.0
        } else {
            0.0
        },
        if t.contains("responsib") { 1.0 } else { 0.0 },
        if t.contains("penalt") { 1.0 } else { 0.0 },
        if t.contains("offence") { 1.0 } else { 0.0 },
        if t.contains("exempt") { 1.0 } else { 0.0 },
    ]
}

fn word_match(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// Build the 397-dim feature vector from embedding + text.
pub fn build_features(embedding: &[f32], text: &str) -> Vec<f32> {
    let modals = modal_features(text);
    let mut features = Vec::with_capacity(397);
    features.extend_from_slice(embedding);
    features.extend_from_slice(&modals);
    features
}

/// Decompose a DrrpClass into specific DRRP types based on the active actor's category.
///
/// - Obligation + government actor → Responsibility
/// - Obligation + governed actor → Duty
/// - Liberty + government actor → Power
/// - Liberty + governed actor → Right
pub fn decompose_drrp(class: DrrpClass, is_government_actor: bool) -> Option<&'static str> {
    match (class, is_government_actor) {
        (DrrpClass::Obligation, true) => Some("Responsibility"),
        (DrrpClass::Obligation, false) => Some("Duty"),
        (DrrpClass::Liberty, true) => Some("Power"),
        (DrrpClass::Liberty, false) => Some("Right"),
        (DrrpClass::None, _) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_features_detects_shall() {
        let features = modal_features("The employer shall ensure safety");
        assert_eq!(features[0], 1.0); // has_shall
        assert_eq!(features[4], 1.0); // has_ensure
        assert_eq!(features[1], 0.0); // has_must
    }

    #[test]
    fn modal_features_detects_may() {
        let features = modal_features("An inspector may serve a notice");
        assert_eq!(features[2], 1.0); // has_may
        assert_eq!(features[0], 0.0); // has_shall
    }

    #[test]
    fn build_features_correct_length() {
        let embedding = vec![0.1f32; 384];
        let features = build_features(&embedding, "shall ensure");
        assert_eq!(features.len(), 397);
    }

    #[test]
    fn decompose_drrp_obligation_govt() {
        assert_eq!(
            decompose_drrp(DrrpClass::Obligation, true),
            Some("Responsibility")
        );
    }

    #[test]
    fn decompose_drrp_obligation_org() {
        assert_eq!(decompose_drrp(DrrpClass::Obligation, false), Some("Duty"));
    }

    #[test]
    fn decompose_drrp_liberty_govt() {
        assert_eq!(decompose_drrp(DrrpClass::Liberty, true), Some("Power"));
    }

    #[test]
    fn decompose_drrp_none() {
        assert_eq!(decompose_drrp(DrrpClass::None, false), None);
    }
}
