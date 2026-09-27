//! What each `fractalaw` stage command records as enrichment provenance (#63).
//!
//! Recording happens after a stage succeeds, only against the Postgres hub,
//! and never fails the command.

use fractalaw_core::provenance::{
    FITNESS, SIGNIFICANCE, SIGNIFICANCE_ROLLUP_VERSION, StageInfo, TAXA, VERSION, code_prompt_version,
    rules_version,
};

pub(crate) fn taxa_parse() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "parse", "regex", "fractalaw-taxa").version(rules_version())]
}

pub(crate) fn taxa_embed() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "embed", "embedding", "all-MiniLM-L6-v2").version("onnx-384")]
}

pub(crate) fn taxa_classify() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "classify", "classifier", "drrp_classifier+position_classifier").version("v8+v3")]
}

/// Escalation uses Gemini when `LLM_PROVIDER=gemini`, else local Ollama.
pub(crate) fn taxa_escalate() -> Vec<StageInfo> {
    let model = if std::env::var("LLM_PROVIDER").as_deref() == Ok("gemini") { "gemini-2.5-flash" } else { "gemma3:4b" };
    vec![StageInfo::new(TAXA, "llm", "llm", model).prompt(code_prompt_version())]
}

pub(crate) fn taxa_infer() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "infer", "inferred", "correlative-rules").version(rules_version())]
}

pub(crate) fn taxa_reconcile() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "reconcile", "reconciled", "fractalaw-reconcile").version(VERSION)]
}

pub(crate) fn taxa_slm() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "slm_local", "slm", "gemma3-position").version("ollama").prompt(code_prompt_version())]
}

pub(crate) fn taxa_validate() -> Vec<StageInfo> {
    vec![StageInfo::new(TAXA, "validate", "llm_audit", "gemini-2.5-flash").prompt(code_prompt_version())]
}

/// `taxa backfill` produces both law-level verdicts: the DRRP roll-up and the significance roll-up.
pub(crate) fn taxa_backfill() -> Vec<StageInfo> {
    vec![
        StageInfo::new(TAXA, "law_drrp_rollup", "rollup", "fractalaw-law-drrp").version(VERSION),
        StageInfo::new(SIGNIFICANCE, "law_rollup", "rollup", "fractalaw-law-significance")
            .version(SIGNIFICANCE_ROLLUP_VERSION),
    ]
}

pub(crate) fn fitness_extract() -> Vec<StageInfo> {
    vec![StageInfo::new(FITNESS, "extract", "regex", "fractalaw-fitness").version(VERSION)]
}

pub(crate) fn fitness_reconcile() -> Vec<StageInfo> {
    vec![StageInfo::new(FITNESS, "reconcile", "reconciled", "fractalaw-fitness-reconcile").version(VERSION)]
}

pub(crate) fn fitness_application() -> Vec<StageInfo> {
    vec![StageInfo::new(FITNESS, "application", "rule", "fractalaw-application").version(VERSION)]
}

pub(crate) fn fitness_compile() -> Vec<StageInfo> {
    vec![StageInfo::new(FITNESS, "compile", "rule", "fractalaw-fitness-compile").version(VERSION)]
}

/// Record `stages` for `laws` (all hub laws when `None`) in a new run.
pub(crate) async fn record(pg_url: Option<&str>, laws: Option<&[String]>, stages: Vec<StageInfo>) {
    let Some(url) = pg_url else { return };
    let result: anyhow::Result<usize> = async {
        let pg = fractalaw_store::PgStore::connect(url).await?;
        Ok(pg.record_run(laws, &stages).await?)
    }
    .await;
    match result {
        Ok(n) => tracing::info!(laws = n, stages = stages.len(), "provenance recorded"),
        Err(e) => eprintln!("warning: enrichment provenance not recorded: {e:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backfill_records_both_rollups() {
        let s = taxa_backfill();
        assert_eq!(s.len(), 2);
        assert_eq!((s[1].family, s[1].model_version.as_deref()), (SIGNIFICANCE, Some(SIGNIFICANCE_ROLLUP_VERSION)));
    }
}
