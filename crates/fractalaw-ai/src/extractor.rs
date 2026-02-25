//! ONNX-based DRRP structured extraction pipeline.
//!
//! Runs a fine-tuned model with three heads (clause span, qualifier span,
//! holder classification) to refine regex-extracted DRRP entries.
//!
//! The model directory must contain:
//! - `model.int8.onnx` (preferred) or `model.onnx`
//! - `tokenizer.json`
//! - `metadata.json`
//! - `holder_labels.json`

use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;
use tracing::info;

/// Metadata loaded from `metadata.json` alongside the model.
#[derive(Deserialize)]
struct ModelMetadata {
    #[allow(dead_code)]
    base_model: String,
    num_holder_classes: usize,
    max_length: usize,
}

/// ONNX-based DRRP structured extraction.
pub struct DrrpExtractor {
    session: Session,
    tokenizer: Tokenizer,
    holder_labels: Vec<String>,
    max_length: usize,
}

/// Result of ONNX extraction for a single DRRP entry.
///
/// Serialises to JSON matching the guest's expected `PolishedOutput` format:
/// `{ "holder": "...", "ai_clause": "...", "qualifier": ... | null, "clause_ref": "..." }`
#[derive(Debug, Clone, Serialize)]
pub struct DrrpExtraction {
    pub holder: String,
    pub ai_clause: String,
    pub qualifier: Option<String>,
    pub clause_ref: String,
    /// Softmax probability of the predicted holder class (not serialised to JSON).
    #[serde(skip)]
    pub confidence: f32,
}

impl DrrpExtraction {
    /// Serialize to JSON matching the guest's expected PolishedOutput format.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl DrrpExtractor {
    /// Load a DRRP extraction model from a directory.
    ///
    /// Prefers `model.int8.onnx` (quantised) over `model.onnx` (full precision).
    pub fn load(model_dir: &Path) -> anyhow::Result<Self> {
        let model_path = if model_dir.join("model.int8.onnx").exists() {
            model_dir.join("model.int8.onnx")
        } else {
            model_dir.join("model.onnx")
        };
        anyhow::ensure!(model_path.exists(), "no ONNX model found in {model_dir:?}");

        let tokenizer_path = model_dir.join("tokenizer.json");
        anyhow::ensure!(
            tokenizer_path.exists(),
            "tokenizer.json not found in {model_dir:?}"
        );

        let session = Session::builder()?.commit_from_file(&model_path)?;

        let metadata: ModelMetadata = {
            let f = std::fs::File::open(model_dir.join("metadata.json"))?;
            serde_json::from_reader(f)?
        };

        let holder_labels: Vec<String> = {
            let f = std::fs::File::open(model_dir.join("holder_labels.json"))?;
            serde_json::from_reader(f)?
        };
        anyhow::ensure!(
            holder_labels.len() == metadata.num_holder_classes,
            "holder_labels.json has {} entries but metadata says {}",
            holder_labels.len(),
            metadata.num_holder_classes
        );

        let max_length = metadata.max_length;

        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;

        // Configure truncation: only truncate the second segment (source_text).
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length,
                strategy: tokenizers::TruncationStrategy::OnlySecond,
                ..Default::default()
            }))
            .map_err(|e| anyhow::anyhow!("set truncation: {e}"))?;

        // Configure padding: pad to fixed max_length (model expects exact size).
        tokenizer.with_padding(Some(tokenizers::PaddingParams {
            strategy: tokenizers::PaddingStrategy::Fixed(max_length),
            ..Default::default()
        }));

        info!(
            model = %model_path.display(),
            max_length,
            num_holder_classes = holder_labels.len(),
            "loaded DRRP extraction model"
        );

        Ok(Self {
            session,
            tokenizer,
            holder_labels,
            max_length,
        })
    }

    /// Extract a refined DRRP entry from a regex-extracted input and source text.
    ///
    /// Returns a `DrrpExtraction` with the holder category, clause text, optional
    /// qualifier, clause reference, and confidence score.
    pub fn extract(
        &mut self,
        drrp_type: &str,
        regex_holder: &str,
        source_text: &str,
        article: &str,
    ) -> anyhow::Result<DrrpExtraction> {
        // 1. Build query string (matches training format).
        let query = format!("{drrp_type} : {regex_holder}");

        // 2. Tokenize as text pair: [CLS] query [SEP] source_text [SEP].
        let encoding = self
            .tokenizer
            .encode((query.as_str(), source_text), true)
            .map_err(|e| anyhow::anyhow!("tokenize: {e}"))?;

        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        anyhow::ensure!(
            ids.len() == self.max_length,
            "expected {} tokens after padding, got {}",
            self.max_length,
            ids.len()
        );

        // 3. Build input tensors [1, max_length].
        let input_ids: Vec<i64> = ids.iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = mask.iter().map(|&m| m as i64).collect();

        let shape = [1i64, self.max_length as i64];
        let ids_tensor = Tensor::from_array((shape, input_ids.into_boxed_slice()))?;
        let mask_tensor = Tensor::from_array((shape, attention_mask.into_boxed_slice()))?;

        // 4. Run inference.
        let outputs = self.session.run(ort::inputs![
            "input_ids" => ids_tensor,
            "attention_mask" => mask_tensor,
        ])?;

        // 5. Extract output tensors (all batch size 1).
        let (_, clause_start_logits) = outputs[0].try_extract_tensor::<f32>()?;
        let (_, clause_end_logits) = outputs[1].try_extract_tensor::<f32>()?;
        let (_, qual_start_logits) = outputs[2].try_extract_tensor::<f32>()?;
        let (_, qual_end_logits) = outputs[3].try_extract_tensor::<f32>()?;
        let (_, has_qual_logits) = outputs[4].try_extract_tensor::<f32>()?;
        let (_, holder_logits) = outputs[5].try_extract_tensor::<f32>()?;

        // 6. Decode holder classification.
        let num_classes = self.holder_labels.len();
        let (holder_idx, confidence) = softmax_argmax(&holder_logits[..num_classes]);
        let holder = self
            .holder_labels
            .get(holder_idx)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        // 7. Find source text start token (first [SEP] + 1).
        let sep_id = self.tokenizer.token_to_id("[SEP]").unwrap_or(102);
        let source_start_tok = ids
            .iter()
            .position(|&id| id == sep_id)
            .map(|p| p + 1)
            .unwrap_or(0);

        // 8. Decode clause span.
        let clause_start = argmax(&clause_start_logits[..self.max_length]);
        let clause_end = argmax(&clause_end_logits[..self.max_length]);
        let (c_start, c_end) = clamp_span(clause_start, clause_end, source_start_tok, mask);

        let clause_token_ids: Vec<u32> = ids[c_start..=c_end].to_vec();
        let ai_clause = self
            .tokenizer
            .decode(&clause_token_ids, true)
            .map_err(|e| anyhow::anyhow!("decode clause: {e}"))?;

        // 9. Decode qualifier.
        let has_qualifier = has_qual_logits[1] > has_qual_logits[0];
        let qualifier = if has_qualifier {
            let q_start = argmax(&qual_start_logits[..self.max_length]);
            let q_end = argmax(&qual_end_logits[..self.max_length]);
            let (qs, qe) = clamp_span(q_start, q_end, source_start_tok, mask);
            let qual_ids: Vec<u32> = ids[qs..=qe].to_vec();
            let text = self
                .tokenizer
                .decode(&qual_ids, true)
                .map_err(|e| anyhow::anyhow!("decode qualifier: {e}"))?;
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        } else {
            None
        };

        // 10. Build clause_ref from article.
        let clause_ref = format_clause_ref(article);

        Ok(DrrpExtraction {
            holder,
            ai_clause,
            qualifier,
            clause_ref,
            confidence,
        })
    }
}

/// Argmax over a float slice.
fn argmax(logits: &[f32]) -> usize {
    logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Softmax + argmax: returns (index, probability of the winning class).
fn softmax_argmax(logits: &[f32]) -> (usize, f32) {
    let max_idx = argmax(logits);
    let max_val = logits[max_idx];
    let exp_sum: f32 = logits.iter().map(|&x| (x - max_val).exp()).sum();
    (max_idx, 1.0 / exp_sum)
}

/// Clamp a predicted span to valid bounds within the source text region.
///
/// Ensures start >= source_start, end >= start, and both are within
/// the non-padding, non-[SEP] token range.
fn clamp_span(
    start: usize,
    end: usize,
    source_start: usize,
    attention_mask: &[u32],
) -> (usize, usize) {
    // Find last real (non-padding) token, then subtract 1 to skip final [SEP].
    let last_real = attention_mask
        .iter()
        .rposition(|&m| m == 1)
        .unwrap_or(attention_mask.len().saturating_sub(1));
    let last_content = if last_real > 0 {
        last_real - 1
    } else {
        last_real
    };

    let s = start.max(source_start).min(last_content);
    let mut e = end.max(s).min(last_content);

    // If the model predicted end < start, collapse to a single token.
    if e < s {
        e = s;
    }

    (s, e)
}

/// Convert an article reference to a standard legal citation.
///
/// Examples: `"section-2"` → `"s.2"`, `"regulation/4"` → `"reg.4"`.
fn format_clause_ref(article: &str) -> String {
    if let Some(num) = article
        .strip_prefix("section-")
        .or_else(|| article.strip_prefix("section/"))
    {
        return format!("s.{num}");
    }
    if let Some(num) = article
        .strip_prefix("regulation-")
        .or_else(|| article.strip_prefix("regulation/"))
    {
        return format!("reg.{num}");
    }
    if let Some(num) = article
        .strip_prefix("rule-")
        .or_else(|| article.strip_prefix("rule/"))
    {
        return format!("r.{num}");
    }
    article.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn model_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("models")
            .join("deberta-v3-drrp")
    }

    fn require_model() -> PathBuf {
        let dir = model_dir();
        if !dir.join("tokenizer.json").exists() {
            panic!("DRRP model not found at {dir:?}. Run training + export first.");
        }
        dir
    }

    #[test]
    fn load_model() {
        let dir = require_model();
        let extractor = DrrpExtractor::load(&dir).unwrap();
        assert_eq!(extractor.max_length, 128);
        assert_eq!(extractor.holder_labels.len(), 27);
    }

    #[test]
    fn extract_hswa_s2() {
        let dir = require_model();
        let mut extractor = DrrpExtractor::load(&dir).unwrap();

        let result = extractor
            .extract(
                "DUTY",
                "Org: Employer",
                "It shall be the duty of every employer to ensure, \
                 so far as is reasonably practicable, the health, \
                 safety and welfare at work of all his employees.",
                "section-2",
            )
            .unwrap();

        // Model should produce some output (even if undertrained).
        assert!(!result.ai_clause.is_empty(), "clause should not be empty");
        assert!(!result.holder.is_empty(), "holder should not be empty");
        assert_eq!(result.clause_ref, "s.2");
        assert!(result.confidence > 0.0, "confidence should be positive");

        println!("Holder:    {}", result.holder);
        println!("Clause:    {}", result.ai_clause);
        println!("Qualifier: {:?}", result.qualifier);
        println!("Ref:       {}", result.clause_ref);
        println!("Conf:      {:.3}", result.confidence);
    }

    #[test]
    fn extract_returns_valid_json() {
        let dir = require_model();
        let mut extractor = DrrpExtractor::load(&dir).unwrap();

        let result = extractor
            .extract(
                "DUTY",
                "Org: Employer",
                "The employer shall provide adequate training.",
                "section-7",
            )
            .unwrap();

        let json = result.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["holder"].is_string());
        assert!(parsed["ai_clause"].is_string());
        assert!(parsed["clause_ref"].is_string());
        // confidence should NOT appear in JSON.
        assert!(parsed.get("confidence").is_none());
    }

    #[test]
    fn format_clause_ref_variants() {
        assert_eq!(format_clause_ref("section-2"), "s.2");
        assert_eq!(format_clause_ref("section/12"), "s.12");
        assert_eq!(format_clause_ref("regulation-4"), "reg.4");
        assert_eq!(format_clause_ref("regulation/3"), "reg.3");
        assert_eq!(format_clause_ref("rule-5"), "r.5");
        assert_eq!(format_clause_ref("schedule-1"), "schedule-1"); // no special handling
    }

    #[test]
    fn argmax_works() {
        assert_eq!(argmax(&[0.1, 0.9, 0.3]), 1);
        assert_eq!(argmax(&[5.0, 1.0, 2.0]), 0);
        assert_eq!(argmax(&[0.0]), 0);
    }

    #[test]
    fn softmax_argmax_probabilities() {
        let (idx, prob) = softmax_argmax(&[10.0, 0.0, 0.0]);
        assert_eq!(idx, 0);
        assert!(
            prob > 0.9,
            "dominant logit should have high probability: {prob}"
        );

        let (idx, prob) = softmax_argmax(&[1.0, 1.0, 1.0]);
        assert!(
            prob < 0.4,
            "uniform logits should have ~0.33 probability: {prob}"
        );
        let _ = idx; // any of 0,1,2 is fine
    }

    #[test]
    fn clamp_span_basic() {
        let mask: Vec<u32> = vec![1, 1, 1, 1, 1, 1, 1, 1, 0, 0]; // 8 real tokens
        // source starts at token 3, last content = 6 (7-1 for final SEP)
        let (s, e) = clamp_span(4, 6, 3, &mask);
        assert_eq!(s, 4);
        assert_eq!(e, 6);

        // Start before source region → clamped to source_start
        let (s, e) = clamp_span(1, 5, 3, &mask);
        assert_eq!(s, 3);
        assert_eq!(e, 5);

        // End beyond content → clamped
        let (s, e) = clamp_span(4, 9, 3, &mask);
        assert_eq!(s, 4);
        assert_eq!(e, 6);

        // End < start → collapsed
        let (s, e) = clamp_span(6, 4, 3, &mask);
        assert_eq!(s, 6);
        assert_eq!(e, 6);
    }
}
