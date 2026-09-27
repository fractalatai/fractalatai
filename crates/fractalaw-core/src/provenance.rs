//! Enrichment provenance (fractalatai #63).
//!
//! Each pipeline stage records, per law, which run, fractalaw version, method
//! and model produced its output and which LAT version (`lat_hash` /
//! `struct_hash`) it read. At publish time the recorded stages of each family
//! become one entry in the `provenance` column of the law-level taxa payload,
//! the contract sertantai-legal implemented (legal `ae31d1e`):
//!
//! ```json
//! [{"family": "taxa", "enrichment_run_id": "…", "fractalaw_version": "…",
//!   "enriched_against": {"lat_hash": "…", "struct_hash": "…"},
//!   "run_started_at": "…", "stages": [{"stage": "parse", "method": "regex", …}],
//!   "provision_method_counts": {"slm": 412, "regex": 90}}]
//! ```

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Git commit (or crate version) of the running binary.
pub const VERSION: &str = env!("FRACTALAW_VERSION");

/// Families legal records one event for, per publish.
pub const TRIAGE: &str = "triage";
pub const TAXA: &str = "taxa";
pub const FITNESS: &str = "fitness";
pub const SIGNIFICANCE: &str = "significance";

/// Frozen law-level significance thresholds (Approach L), part of the roll-up's version.
pub const SIGNIFICANCE_ROLLUP_VERSION: &str = "approach-L;LOW<=6.14;HIGH>=11.06";

fn short_hash(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
    }
    format!("{:x}", h.finalize())[..12].to_string()
}

/// Content version of the rule data embedded in this binary (actor dictionary,
/// correlative rules): changes whenever a dictionary edit changes regex output.
pub fn rules_version() -> String {
    format!(
        "rules:{}",
        short_hash(&[crate::taxa::actors::ACTOR_YAML, crate::taxa::correlatives::RULES_YAML])
    )
}

/// Prompts are inline in code, so the binary's commit versions them.
pub fn code_prompt_version() -> String {
    format!("code:{VERSION}")
}

/// This process's command line, with connection URLs (which carry credentials) masked.
pub fn command_line() -> String {
    std::env::args()
        .map(|a| if a.contains("://") { "<url>".to_string() } else { a })
        .collect::<Vec<_>>()
        .join(" ")
}

/// What one stage is, independent of the laws it ran on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageInfo {
    pub family: &'static str,
    pub stage: &'static str,
    /// regex | classifier | embedding | slm | llm | inferred | reconciled | rollup | rule | …
    pub method: &'static str,
    pub model: String,
    pub model_version: Option<String>,
    pub prompt_version: Option<String>,
}

impl StageInfo {
    pub fn new(family: &'static str, stage: &'static str, method: &'static str, model: impl Into<String>) -> Self {
        Self { family, stage, method, model: model.into(), model_version: None, prompt_version: None }
    }

    pub fn version(mut self, v: impl Into<String>) -> Self {
        self.model_version = Some(v.into());
        self
    }

    pub fn prompt(mut self, v: impl Into<String>) -> Self {
        self.prompt_version = Some(v.into());
        self
    }
}

/// One recorded stage for one law, as stored in the hub.
#[derive(Debug, Clone, PartialEq)]
pub struct StageRecord {
    pub family: String,
    pub stage: String,
    pub run_id: String,
    pub run_started_at: String,
    pub version: String,
    pub method: Option<String>,
    pub model: Option<String>,
    pub model_version: Option<String>,
    pub prompt_version: Option<String>,
    pub lat_hash: Option<String>,
    pub struct_hash: Option<String>,
    /// ISO-8601; sorts chronologically
    pub ran_at: String,
}

/// Build the payload's provenance list for the given families, from one law's
/// stage records and per-family provision method counts. Families with no
/// recorded stages are left out (legal then records a verdict-only event).
///
/// - run id, version and run_started_at come from the family's most recent stage
///   (the run that produced its current output);
/// - `enriched_against` comes from the family's **earliest** stage: output built
///   on an older LAT is only as fresh as its oldest input, so a later re-parse
///   anywhere upstream shows as stale.
pub fn build_entries(
    families: &[&str],
    records: &[StageRecord],
    method_counts: &HashMap<String, BTreeMap<String, i64>>,
) -> Value {
    let mut out = Vec::new();
    for fam in families {
        let mut stages: Vec<&StageRecord> = records.iter().filter(|r| r.family == *fam).collect();
        if stages.is_empty() {
            continue;
        }
        stages.sort_by(|a, b| a.ran_at.cmp(&b.ran_at).then(a.stage.cmp(&b.stage)));
        let (first, last) = (stages[0], stages[stages.len() - 1]);
        out.push(json!({
            "family": fam,
            "enrichment_run_id": last.run_id,
            "fractalaw_version": last.version,
            "run_started_at": last.run_started_at,
            "enriched_against": {"lat_hash": first.lat_hash, "struct_hash": first.struct_hash},
            "stages": stages.iter().map(|s| json!({
                "stage": s.stage,
                "method": s.method,
                "model": s.model,
                "model_version": s.model_version,
                "prompt_version": s.prompt_version,
                "ran_at": s.ran_at,
                "run_id": s.run_id,
            })).collect::<Vec<_>>(),
            "provision_method_counts": method_counts.get(*fam).cloned().unwrap_or_default(),
        }));
    }
    Value::Array(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(family: &str, stage: &str, run: &str, hash: &str, at: &str) -> StageRecord {
        StageRecord {
            family: family.into(),
            stage: stage.into(),
            run_id: run.into(),
            run_started_at: format!("{at}-start"),
            version: format!("sha-{run}"),
            method: Some("regex".into()),
            model: Some("m".into()),
            model_version: None,
            prompt_version: None,
            lat_hash: Some(hash.into()),
            struct_hash: Some(format!("s{hash}")),
            ran_at: at.into(),
        }
    }

    #[test]
    fn entries_per_family_latest_run_earliest_hash() {
        let records = [
            rec("taxa", "parse", "r1", "old", "2026-09-01T00:00:00Z"),
            rec("taxa", "reconcile", "r2", "new", "2026-09-20T00:00:00Z"),
            rec("fitness", "compile", "r3", "new", "2026-09-21T00:00:00Z"),
        ];
        let mut counts = HashMap::new();
        counts.insert("taxa".to_string(), BTreeMap::from([("slm".to_string(), 3), ("regex".to_string(), 1)]));
        let v = build_entries(&[TAXA, FITNESS, SIGNIFICANCE], &records, &counts);
        let entries = v.as_array().unwrap();
        assert_eq!(entries.len(), 2, "no significance stages → no entry");
        let taxa = &entries[0];
        assert_eq!(taxa["family"], "taxa");
        assert_eq!(taxa["enrichment_run_id"], "r2");
        assert_eq!(taxa["fractalaw_version"], "sha-r2");
        assert_eq!(taxa["enriched_against"]["lat_hash"], "old", "oldest input wins");
        assert_eq!(taxa["stages"].as_array().unwrap().len(), 2);
        assert_eq!(taxa["stages"][0]["stage"], "parse");
        assert_eq!(taxa["provision_method_counts"]["slm"], 3);
        assert_eq!(entries[1]["provision_method_counts"], json!({}));
    }

    #[test]
    fn versions_are_stamped() {
        assert!(!VERSION.is_empty());
        assert!(rules_version().starts_with("rules:") && rules_version().len() == 18);
        let s = StageInfo::new(TAXA, "llm", "llm", "gemini-2.5-flash").prompt(code_prompt_version());
        assert_eq!(s.prompt_version.unwrap(), format!("code:{VERSION}"));
    }
}
