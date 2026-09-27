//! Enrichment provenance in the hub (fractalatai #63).
//!
//! Recording lives in SQL functions so the Rust CLI and the Python/pod scripts
//! record identically:
//!
//! - `fractalaw_start_run(command, version) → uuid`
//! - `fractalaw_record_stage(run, laws[], family, stage, method, model, model_version, prompt_version)`
//!   upserts one row per (law, family, stage), stamping the `lat_hash` /
//!   `struct_hash` of the hub rows the stage just read;
//! - `fractalaw_lat_hash(law)` / `fractalaw_struct_hash(law)` implement the
//!   #62 contract (same vectors as `fractalaw_core::lat_sync`).

use std::collections::{BTreeMap, HashMap};

use fractalaw_core::provenance::{StageInfo, StageRecord};
use sqlx::Row;

use crate::{PgStore, StoreError};

fn db(ctx: &'static str) -> impl Fn(sqlx::Error) -> StoreError {
    move |e| StoreError::Other(format!("{ctx}: {e}"))
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS enrichment_runs (
    run_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    version     TEXT NOT NULL,
    command     TEXT NOT NULL,
    host        TEXT
);
CREATE TABLE IF NOT EXISTS enrichment_provenance (
    law_name        TEXT NOT NULL,
    family          TEXT NOT NULL,
    stage           TEXT NOT NULL,
    run_id          UUID NOT NULL REFERENCES enrichment_runs(run_id),
    method          TEXT,
    model           TEXT,
    model_version   TEXT,
    prompt_version  TEXT,
    lat_hash        TEXT,
    struct_hash     TEXT,
    ran_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (law_name, family, stage)
);
CREATE INDEX IF NOT EXISTS idx_enrichment_provenance_run ON enrichment_provenance (run_id);
ALTER TABLE legislation_text ADD COLUMN IF NOT EXISTS significance_method TEXT;

CREATE OR REPLACE FUNCTION fractalaw_norm(t TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE AS $f$
  SELECT regexp_replace(regexp_replace(
           regexp_replace(normalize(coalesce(t, ''), NFC),
             E'[\\u0009-\\u000D\\u0020\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000]+', ' ', 'g'),
           '^ ', ''), ' $', '')
$f$;

CREATE OR REPLACE FUNCTION fractalaw_lat_hash(law TEXT) RETURNS TEXT
LANGUAGE sql STABLE AS $f$
  SELECT encode(sha256(convert_to(coalesce(string_agg(
           section_id || E'\t' || coalesce(sort_key, '') || E'\t' || fractalaw_norm(text) || E'\n',
           '' ORDER BY section_id COLLATE "C"), ''), 'UTF8')), 'hex')
  FROM legislation_text WHERE law_name = law
$f$;

CREATE OR REPLACE FUNCTION fractalaw_struct_hash(law TEXT) RETURNS TEXT
LANGUAGE sql STABLE AS $f$
  SELECT encode(sha256(convert_to(coalesce(string_agg(
           concat_ws(E'\t', section_id,
             coalesce(section_type, ''), coalesce(hierarchy_path, ''), coalesce(depth::text, ''),
             coalesce(position::text, ''), coalesce(part, ''), coalesce(chapter, ''),
             coalesce(heading_group, ''), coalesce(provision, ''), coalesce(paragraph, ''),
             coalesce(sub_paragraph, ''), coalesce(schedule, ''), coalesce(extent_code, ''),
             coalesce(language, ''), coalesce(amendment_count::text, ''),
             coalesce(modification_count::text, ''), coalesce(commencement_count::text, ''),
             coalesce(extent_count::text, ''), coalesce(editorial_count::text, '')) || E'\n',
           '' ORDER BY section_id COLLATE "C"), ''), 'UTF8')), 'hex')
  FROM legislation_text WHERE law_name = law
$f$;

CREATE OR REPLACE FUNCTION fractalaw_start_run(p_command TEXT, p_version TEXT, p_host TEXT DEFAULT NULL)
RETURNS UUID LANGUAGE sql AS $f$
  INSERT INTO enrichment_runs (command, version, host) VALUES (p_command, p_version, p_host) RETURNING run_id
$f$;

CREATE OR REPLACE FUNCTION fractalaw_record_stage(
    p_run UUID, p_laws TEXT[], p_family TEXT, p_stage TEXT, p_method TEXT,
    p_model TEXT, p_model_version TEXT DEFAULT NULL, p_prompt_version TEXT DEFAULT NULL)
RETURNS INTEGER LANGUAGE sql AS $f$
  WITH ins AS (
    INSERT INTO enrichment_provenance
      (law_name, family, stage, run_id, method, model, model_version, prompt_version, lat_hash, struct_hash, ran_at)
    SELECT l, p_family, p_stage, p_run, p_method, p_model, p_model_version, p_prompt_version,
           fractalaw_lat_hash(l), fractalaw_struct_hash(l), now()
    FROM (SELECT DISTINCT unnest(p_laws) AS l) laws
    ON CONFLICT (law_name, family, stage) DO UPDATE SET
      run_id = EXCLUDED.run_id, method = EXCLUDED.method, model = EXCLUDED.model,
      model_version = EXCLUDED.model_version, prompt_version = EXCLUDED.prompt_version,
      lat_hash = EXCLUDED.lat_hash, struct_hash = EXCLUDED.struct_hash, ran_at = EXCLUDED.ran_at
    RETURNING 1)
  SELECT count(*)::int FROM ins
$f$;
"#;

impl PgStore {
    /// Create the provenance tables and functions (additive; idempotent).
    pub async fn ensure_provenance_schema(&self) -> Result<(), StoreError> {
        sqlx::raw_sql(SCHEMA).execute(self.pool()).await.map_err(db("provenance schema"))?;
        Ok(())
    }

    /// Start a run; returns its id.
    pub async fn start_enrichment_run(&self, command: &str, version: &str) -> Result<String, StoreError> {
        let host = std::env::var("HOSTNAME").ok();
        sqlx::query_scalar("SELECT fractalaw_start_run($1, $2, $3)::text")
            .bind(command)
            .bind(version)
            .bind(host)
            .fetch_one(self.pool())
            .await
            .map_err(db("start run"))
    }

    /// Record that `stage` ran on `laws` in `run_id`. Returns rows recorded.
    pub async fn record_stage(&self, run_id: &str, laws: &[String], stage: &StageInfo) -> Result<i32, StoreError> {
        sqlx::query_scalar("SELECT fractalaw_record_stage($1::uuid, $2, $3, $4, $5, $6, $7, $8)")
            .bind(run_id)
            .bind(laws)
            .bind(stage.family)
            .bind(stage.stage)
            .bind(stage.method)
            .bind(&stage.model)
            .bind(&stage.model_version)
            .bind(&stage.prompt_version)
            .fetch_one(self.pool())
            .await
            .map_err(db("record stage"))
    }

    /// Record `stages` for `laws` (every hub law when `None`) in a new run for
    /// this process. Returns the number of laws recorded.
    pub async fn record_run(&self, laws: Option<&[String]>, stages: &[StageInfo]) -> Result<usize, StoreError> {
        self.ensure_provenance_schema().await?;
        let laws = match laws {
            Some(l) => l.to_vec(),
            None => self.hub_law_names().await?,
        };
        let run = self
            .start_enrichment_run(&fractalaw_core::provenance::command_line(), fractalaw_core::provenance::VERSION)
            .await?;
        for st in stages {
            self.record_stage(&run, &laws, st).await?;
        }
        Ok(laws.len())
    }

    /// Every law with LAT in the hub (for stages run without a law filter).
    pub async fn hub_law_names(&self) -> Result<Vec<String>, StoreError> {
        sqlx::query_scalar("SELECT DISTINCT law_name FROM legislation_text ORDER BY 1")
            .fetch_all(self.pool())
            .await
            .map_err(db("hub laws"))
    }

    /// The hub's current (lat_hash, struct_hash) for a law, per the #62 contract.
    pub async fn hub_lat_hashes(&self, law_name: &str) -> Result<(String, String), StoreError> {
        let r = sqlx::query("SELECT fractalaw_lat_hash($1), fractalaw_struct_hash($1)")
            .bind(law_name)
            .fetch_one(self.pool())
            .await
            .map_err(db("hub hashes"))?;
        Ok((r.get(0), r.get(1)))
    }

    /// Recorded stages for one law.
    pub async fn stage_records(&self, law_name: &str) -> Result<Vec<StageRecord>, StoreError> {
        let rows = sqlx::query(
            "SELECT p.family, p.stage, p.run_id::text, to_char(r.started_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                    r.version, p.method, p.model, p.model_version, p.prompt_version, p.lat_hash, p.struct_hash,
                    to_char(p.ran_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')
             FROM enrichment_provenance p JOIN enrichment_runs r USING (run_id)
             WHERE p.law_name = $1",
        )
        .bind(law_name)
        .fetch_all(self.pool())
        .await
        .map_err(db("stage records"))?;
        Ok(rows
            .into_iter()
            .map(|r| StageRecord {
                family: r.get(0),
                stage: r.get(1),
                run_id: r.get(2),
                run_started_at: r.get(3),
                version: r.get(4),
                method: r.get(5),
                model: r.get(6),
                model_version: r.get(7),
                prompt_version: r.get(8),
                lat_hash: r.get(9),
                struct_hash: r.get(10),
                ran_at: r.get(11),
            })
            .collect())
    }

    /// Per-provision method counts by family for one law:
    /// taxa from reconciled `provision_actors`, fitness per mention (the tier
    /// that supplied its entities, or `propagated`), significance per rated
    /// provision (`significance_method`, `unrecorded` before #63).
    pub async fn provision_method_counts(
        &self,
        law_name: &str,
    ) -> Result<HashMap<String, BTreeMap<String, i64>>, StoreError> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT 'taxa', coalesce(pa.extraction_method, 'unreconciled'), count(*)
             FROM provision_actors pa JOIN legislation_text lt USING (section_id)
             WHERE lt.law_name = $1 AND lt.scope IS DISTINCT FROM 'amendment'
             GROUP BY 2
             UNION ALL
             SELECT 'fitness',
                    CASE WHEN fm.extraction_method = 'propagated' THEN 'propagated'
                         WHEN coalesce(cardinality(fm.ft_entities), 0) > 0 THEN 'ft'
                         WHEN coalesce(cardinality(fm.llm_entities), 0) > 0 THEN 'llm'
                         WHEN coalesce(cardinality(fm.slm_entities), 0) > 0 THEN 'slm'
                         WHEN coalesce(cardinality(fm.regex_entities), 0) > 0 THEN 'regex'
                         ELSE 'polarity_only' END,
                    count(*)
             FROM fitness_mentions fm JOIN legislation_text lt USING (section_id)
             WHERE lt.law_name = $1 AND lt.scope IS DISTINCT FROM 'amendment'
             GROUP BY 2
             UNION ALL
             SELECT 'significance', coalesce(significance_method, 'unrecorded'), count(*)
             FROM legislation_text
             WHERE law_name = $1 AND significance_overall IS NOT NULL
             GROUP BY 2",
        )
        .bind(law_name)
        .fetch_all(self.pool())
        .await
        .map_err(db("method counts"))?;
        let mut out: HashMap<String, BTreeMap<String, i64>> = HashMap::new();
        for (fam, method, n) in rows {
            out.entry(fam).or_default().insert(method, n);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    //! Against the scratch database `fractalaw_lat_test` (never the hub).
    use super::*;
    use fractalaw_core::lat_sync::{lat_hash, struct_hash, StructRow, STRUCT_COLUMNS};
    use fractalaw_core::provenance::{TAXA, VERSION, build_entries};

    const TEST_DB: &str = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw_lat_test";

    async fn store() -> Option<PgStore> {
        let s = PgStore::connect(TEST_DB).await.ok()?;
        s.ensure_lat_sync_tables().await.ok()?;
        s.ensure_provenance_schema().await.ok()?;
        Some(s)
    }

    #[tokio::test]
    async fn sql_hashes_match_rust_contract() {
        let Some(s) = store().await else { eprintln!("test DB unavailable, skipping"); return };
        for law in ["UK_ssi_2016_88", "UK_uksi_2016_614", "UK_wsi_2021_77", "NO_SUCH_LAW"] {
            let rows = s.hub_lat_rows(law).await.unwrap();
            let cols = STRUCT_COLUMNS.iter().map(|c| format!("{c}::text")).collect::<Vec<_>>().join(", ");
            let struct_rows: Vec<StructRow> = sqlx::query(&format!(
                "SELECT section_id, {cols} FROM legislation_text WHERE law_name = $1"
            ))
            .bind(law)
            .fetch_all(s.pool())
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.get::<String, _>(0), (1..=STRUCT_COLUMNS.len()).map(|i| r.get::<Option<String>, _>(i)).collect()))
            .collect();
            let (lh, sh) = s.hub_lat_hashes(law).await.unwrap();
            assert_eq!(lh, lat_hash(&rows), "{law} lat_hash");
            assert_eq!(sh, struct_hash(&struct_rows), "{law} struct_hash");
        }
        // Where the hub is in sync with legal (#62 applied), it matches legal's manifest
        let states = s.lat_sync_states().await.unwrap();
        if let Some(st) = states.get("UK_ssi_2016_88") {
            let (lh, sh) = s.hub_lat_hashes("UK_ssi_2016_88").await.unwrap();
            assert_eq!(lh, st.lat_hash);
            assert_eq!(Some(sh), st.struct_hash.clone());
        }
    }

    #[tokio::test]
    async fn record_and_build_entries() {
        let Some(s) = store().await else { eprintln!("test DB unavailable, skipping"); return };
        let law = "UK_ssi_2016_88".to_string();
        let run = s.start_enrichment_run("test: taxa parse", VERSION).await.unwrap();
        let st = StageInfo::new(TAXA, "parse", "regex", "fractalaw-taxa").version("rules:test");
        assert_eq!(s.record_stage(&run, &[law.clone(), law.clone()], &st).await.unwrap(), 1);
        let recs = s.stage_records(&law).await.unwrap();
        let parse = recs.iter().find(|r| r.family == "taxa" && r.stage == "parse").unwrap();
        assert_eq!(parse.run_id, run);
        assert_eq!(parse.version, VERSION);
        assert_eq!(parse.lat_hash.as_deref(), Some(s.hub_lat_hashes(&law).await.unwrap().0.as_str()));
        let counts = s.provision_method_counts(&law).await.unwrap();
        let v = build_entries(&[TAXA], &recs, &counts);
        assert_eq!(v[0]["family"], "taxa");
        assert_eq!(v[0]["enriched_against"]["lat_hash"].as_str(), parse.lat_hash.as_deref());
    }
}
