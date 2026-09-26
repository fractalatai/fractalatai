//! `pull-lat` against legal's LAT manifest (fractalatai #62).
//!
//! Legal serves a per-law manifest (`row_count`, `lat_hash`, `struct_hash`)
//! and a section_id rename log. A law is stale when its manifest hashes differ
//! from what the hub last applied (`lat_sync_state`). Stale laws are pulled,
//! checked against the manifest, planned with `fractalaw_core::lat_sync`, and
//! (with `--apply`) applied in one transaction that refuses to commit if tier
//! data on carried rows didn't survive.
//!
//! Laws the hub holds but legal doesn't are never removed here: they are
//! reported with a delete verdict. Only an approved list passed to
//! `--archive-laws` is archived (reversibly, `--restore-laws`).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use chrono::{DateTime, Utc};
use fractalaw_core::lat_sync::{self, DiffPlan, ManifestEntry, Rename, RenameSource, RenameStatus};
use fractalaw_store::{DuckStore, LatApplyReport, LatSyncState, PgStore};
use fractalaw_sync::{LatManifestEntry, ZenohSync};

use crate::{get_string_value, open_duck, ZenohArgs};

/// Archive reason for laws legal no longer holds.
const NOT_IN_LEGAL: &str = "not_in_legal";

pub(crate) struct PullLatOpts {
    pub laws: Option<Vec<String>>,
    pub stale: bool,
    pub apply: bool,
    pub allow_benchmark: bool,
    pub limit: Option<usize>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// Hub already applied this manifest version
    InSync,
    /// Neither legal nor the hub holds LAT
    NotHeld,
    /// Hub holds rows legal no longer serves: report + verdict only
    DeleteCandidate,
    /// Legal's rows changed between manifest and pull: retry later
    ManifestMoved,
    /// Planned (dry run, or benchmark held, or over --limit)
    Planned,
    Applied,
    /// Tier data would have been lost: rolled back
    GateFailed,
}

impl Action {
    fn as_str(self) -> &'static str {
        match self {
            Self::InSync => "in_sync",
            Self::NotHeld => "not_held",
            Self::DeleteCandidate => "delete_candidate",
            Self::ManifestMoved => "manifest_moved",
            Self::Planned => "planned",
            Self::Applied => "applied",
            Self::GateFailed => "gate_failed",
        }
    }
}

pub(crate) struct LawOutcome {
    pub law: String,
    pub action: Action,
    pub hub_rows: i64,
    pub legal_rows: u64,
    pub plan: Option<DiffPlan>,
    pub report: Option<LatApplyReport>,
    pub note: String,
}

impl LawOutcome {
    fn new(law: &str, action: Action, hub_rows: i64, legal_rows: u64) -> Self {
        Self { law: law.to_string(), action, hub_rows, legal_rows, plan: None, report: None, note: String::new() }
    }

    /// Text changed or rows inserted: the law needs parse → reconcile → backfill.
    pub fn needs_reparse(&self) -> bool {
        self.action == Action::Applied
            && self.plan.as_ref().is_some_and(|p| !p.text_changed.is_empty() || !p.inserted.is_empty())
    }
}

pub(crate) fn manifest_entry(m: &LatManifestEntry) -> ManifestEntry {
    ManifestEntry {
        law_name: m.law_name.clone(),
        row_count: m.row_count,
        lat_hash: m.lat_hash.clone(),
        struct_hash: m.struct_hash.clone(),
    }
}

fn in_sync(state: Option<&LatSyncState>, m: &ManifestEntry) -> bool {
    state.is_some_and(|s| s.lat_hash == m.lat_hash && s.struct_hash == m.struct_hash)
}

pub(crate) fn benchmark_laws(duck: &DuckStore) -> anyhow::Result<HashSet<String>> {
    let mut out = HashSet::new();
    for b in duck.query_arrow("SELECT name FROM legislation WHERE is_benchmark")? {
        if let Some(col) = b.column_by_name("name") {
            out.extend((0..b.num_rows()).filter_map(|i| get_string_value(col.as_ref(), i)));
        }
    }
    Ok(out)
}

/// Pull, verify and plan one law; apply when asked. Never removes a law.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn sync_law(
    pg: &PgStore,
    sync: &ZenohSync,
    law: &str,
    manifest: Option<&ManifestEntry>,
    state: Option<&LatSyncState>,
    hub_rows: i64,
    apply: bool,
    timeout: Duration,
) -> anyhow::Result<LawOutcome> {
    let Some(m) = manifest.filter(|m| m.row_count > 0) else {
        let action = if hub_rows > 0 { Action::DeleteCandidate } else { Action::NotHeld };
        return Ok(LawOutcome::new(law, action, hub_rows, 0));
    };
    if in_sync(state, m) {
        return Ok(LawOutcome::new(law, Action::InSync, hub_rows, m.row_count));
    }

    let batches = sync.query_lat(law, timeout).await.with_context(|| format!("query LAT {law}"))?;
    let legal = lat_sync::lat_rows_from_batches(&batches).map_err(anyhow::Error::msg)?;
    let mut out = LawOutcome::new(law, Action::ManifestMoved, hub_rows, m.row_count);
    if legal.len() as u64 != m.row_count || lat_sync::lat_hash(&legal) != m.lat_hash {
        out.note = format!("pulled {} rows, hash differs from manifest", legal.len());
        return Ok(out);
    }
    if let Some(expected) = &m.struct_hash {
        let rows = lat_sync::struct_rows_from_batches(&batches).map_err(anyhow::Error::msg)?;
        if &lat_sync::struct_hash(&rows) != expected {
            out.note = "struct_hash differs from manifest".into();
            return Ok(out);
        }
    }

    // Legal's renames since the last apply (oldest first)
    let since = state.and_then(|s| s.renames_through);
    let mut renames = Vec::new();
    let mut renames_through = since;
    for r in sync.query_lat_renames(law, timeout).await.with_context(|| format!("query renames {law}"))? {
        let Ok(at) = DateTime::parse_from_rfc3339(&r.created_at).map(|d| d.with_timezone(&Utc)) else {
            continue;
        };
        if since.is_some_and(|s| at <= s) {
            continue;
        }
        let Some(status) = RenameStatus::parse(&r.status) else { continue };
        renames_through = Some(renames_through.map_or(at, |t: DateTime<Utc>| t.max(at)));
        renames.push(Rename { old_section_id: r.old_section_id, new_section_id: r.new_section_id, status });
    }

    let hub = pg.hub_lat_rows(law).await?;
    let plan = lat_sync::plan_diff(&hub, &legal, &renames);
    out.action = Action::Planned;
    if apply {
        let report = pg.apply_lat_diff(law, &batches, &plan, m, renames_through).await?;
        out.action = if report.committed { Action::Applied } else { Action::GateFailed };
        if !report.committed {
            out.note = format!("tier data before {:?} after {:?}", report.before, report.after);
        }
        out.report = Some(report);
    }
    out.plan = Some(plan);
    Ok(out)
}

// ── delete verification (deterministic pass) ──

/// fractalaw's DuckDB view of a law's revocation audit trail.
#[derive(Debug, Clone, Default)]
pub(crate) struct DuckLrt {
    pub status: Option<String>,
    pub status_conflict: bool,
    pub latest_rescind_date: Option<String>,
    pub rescinded_by: i64,
}

/// Verdict for a law the hub holds but legal serves no LAT for. Only
/// `verified_revoked` is eligible for the approval batch; everything else
/// goes to review. Absence alone never qualifies.
pub(crate) fn delete_verdict(legal_live: Option<&str>, duck: Option<&DuckLrt>) -> &'static str {
    let Some(live) = legal_live else { return "unknown_to_legal" };
    let revoked = live.contains("Revoked") || live.contains("Repealed");
    if !revoked {
        return "legal_gap_in_force";
    }
    match duck {
        Some(d)
            if d.status.as_deref() == Some("revoked")
                && !d.status_conflict
                && d.rescinded_by > 0
                && d.latest_rescind_date.is_some() =>
        {
            "verified_revoked"
        }
        _ => "needs_review",
    }
}

fn duck_lrt(duck: &DuckStore, laws: &[String]) -> anyhow::Result<HashMap<String, DuckLrt>> {
    if laws.is_empty() {
        return Ok(HashMap::new());
    }
    let list: Vec<String> = laws.iter().map(|l| format!("'{}'", l.replace('\'', "''"))).collect();
    let sql = format!(
        "SELECT name, status::VARCHAR AS status, coalesce(status_conflict, false)::VARCHAR AS conflict,
                latest_rescind_date::VARCHAR AS rescind_date,
                coalesce(len(rescinded_by), 0)::VARCHAR AS rescinded_by
         FROM legislation WHERE name IN ({})",
        list.join(",")
    );
    let mut out = HashMap::new();
    for b in duck.query_arrow(&sql)? {
        let col = |n: &str| b.column_by_name(n).cloned();
        let (Some(name), Some(status), Some(conflict), Some(date), Some(by)) =
            (col("name"), col("status"), col("conflict"), col("rescind_date"), col("rescinded_by"))
        else {
            continue;
        };
        for i in 0..b.num_rows() {
            let Some(law) = get_string_value(name.as_ref(), i) else { continue };
            out.insert(law, DuckLrt {
                status: get_string_value(status.as_ref(), i),
                status_conflict: get_string_value(conflict.as_ref(), i).as_deref() == Some("true"),
                latest_rescind_date: get_string_value(date.as_ref(), i),
                rescinded_by: get_string_value(by.as_ref(), i).and_then(|s| s.parse().ok()).unwrap_or(0),
            });
        }
    }
    Ok(out)
}

async fn legal_live(sync: &ZenohSync, law: &str, timeout: Duration) -> Option<String> {
    let batches = sync.query_lrt(law, timeout).await.ok()?;
    batches.iter().find_map(|b| {
        let col = b.column_by_name("live")?;
        (0..b.num_rows()).find_map(|i| get_string_value(col.as_ref(), i))
    })
}

// ── reports ──

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

fn write_law_report(path: &Path, outcomes: &[LawOutcome]) -> anyhow::Result<()> {
    let mut s = String::from(
        "law,action,hub_rows,legal_rows,unchanged,sort_key_changed,text_changed,renamed_map,renamed_text,inserted,held,archived,actors_carried,note\n",
    );
    for o in outcomes {
        let p = o.plan.clone().unwrap_or_default();
        let by = |src| p.renamed.iter().filter(|r| r.2 == src).count();
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            o.law, o.action.as_str(), o.hub_rows, o.legal_rows, p.unchanged.len(), p.sort_key_changed,
            p.text_changed.len(), by(RenameSource::LegalMap), by(RenameSource::TextMatch), p.inserted.len(),
            p.held.len(), p.archived.len(),
            o.report.as_ref().map(|r| r.after.provision_actors.to_string()).unwrap_or_default(),
            csv_field(&o.note)
        );
    }
    std::fs::write(path, s).with_context(|| format!("writing {}", path.display()))
}

fn report_dir(data_dir: &Path) -> anyhow::Result<PathBuf> {
    let dir = data_dir.join("lat-sync");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Comma/newline-separated law names, or a path to a file of them.
pub(crate) fn parse_law_list(s: &str) -> anyhow::Result<Vec<String>> {
    let text = if Path::new(s).is_file() { std::fs::read_to_string(s)? } else { s.to_string() };
    Ok(text.split([',', '\n']).map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
}

pub(crate) async fn connect(zenoh: &ZenohArgs) -> anyhow::Result<ZenohSync> {
    let config = zenoh.build_zenoh_config()?;
    ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))
}

pub(crate) async fn open_pg(pg_url: Option<&str>) -> anyhow::Result<PgStore> {
    let url = pg_url.context("pull-lat diff-apply needs the Postgres hub (--pg / FRACTALAW_PG)")?;
    let pg = PgStore::connect(url).await.context("connecting to PostgreSQL")?;
    pg.ensure_lat_sync_tables().await?;
    Ok(pg)
}

/// Compare, plan and (optionally) apply a set of laws. Stops at the first
/// carry-over gate failure.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_pass(
    pg: &PgStore,
    sync: &ZenohSync,
    benchmarks: &HashSet<String>,
    laws: &[String],
    manifest: &HashMap<String, ManifestEntry>,
    opts: &PullLatOpts,
) -> anyhow::Result<Vec<LawOutcome>> {
    let states = pg.lat_sync_states().await?;
    let hub_counts = pg.hub_law_row_counts().await?;
    let mut outcomes = Vec::new();
    let mut applied = 0usize;
    for law in laws {
        let hub_rows = hub_counts.get(law).copied().unwrap_or(0);
        let benchmark = benchmarks.contains(law) && !opts.allow_benchmark;
        let under_limit = opts.limit.is_none_or(|l| applied < l);
        let apply = opts.apply && !benchmark && under_limit;
        let mut o = match sync_law(pg, sync, law, manifest.get(law), states.get(law), hub_rows, apply, opts.timeout).await {
            Ok(o) => o,
            Err(e) => {
                let mut o = LawOutcome::new(law, Action::ManifestMoved, hub_rows, 0);
                o.note = format!("error: {e:#}");
                o
            }
        };
        if o.action == Action::Planned && opts.apply {
            o.note = if benchmark { "benchmark: report only".into() } else { "over --limit".into() };
        }
        if matches!(o.action, Action::Applied | Action::GateFailed) {
            applied += 1;
        }
        let stop = o.action == Action::GateFailed;
        if !matches!(o.action, Action::InSync | Action::NotHeld) {
            eprintln!("  {law}: {}{}", o.action.as_str(), if o.note.is_empty() { String::new() } else { format!(" ({})", o.note) });
        }
        outcomes.push(o);
        if stop {
            eprintln!("STOPPED: carry-over gate failed for {law}; nothing committed for it. Investigate before continuing.");
            break;
        }
    }
    Ok(outcomes)
}

/// Every law the hub holds or has applied, against legal's full manifest.
pub(crate) async fn stale_scope(
    pg: &PgStore,
    sync: &ZenohSync,
    timeout: Duration,
) -> anyhow::Result<(Vec<String>, HashMap<String, ManifestEntry>)> {
    let all = sync.query_lat_manifest("*", timeout).await?;
    anyhow::ensure!(!all.is_empty(), "legal's manifest is empty (server down?); nothing compared");
    let manifest = all.iter().map(|e| (e.law_name.clone(), manifest_entry(e))).collect();
    let laws: BTreeSet<String> = pg
        .hub_law_row_counts()
        .await?
        .into_keys()
        .chain(pg.lat_sync_states().await?.into_keys())
        .collect();
    Ok((laws.into_iter().collect(), manifest))
}

/// Write the pass report, delete-candidate verdicts and re-parse list; print a summary.
pub(crate) async fn report_pass(
    data_dir: &Path,
    sync: &ZenohSync,
    duck: &DuckStore,
    outcomes: &[LawOutcome],
    timeout: Duration,
) -> anyhow::Result<()> {
    let candidates: Vec<String> =
        outcomes.iter().filter(|o| o.action == Action::DeleteCandidate).map(|o| o.law.clone()).collect();
    let duck_view = duck_lrt(duck, &candidates)?;
    let mut del = String::from("law,hub_rows,legal_live,duck_status,status_conflict,rescinded_by,latest_rescind_date,verdict\n");
    let mut verdicts: HashMap<&'static str, usize> = HashMap::new();
    for o in outcomes.iter().filter(|o| o.action == Action::DeleteCandidate) {
        let live = legal_live(sync, &o.law, timeout).await;
        let d = duck_view.get(&o.law);
        let verdict = delete_verdict(live.as_deref(), d);
        *verdicts.entry(verdict).or_default() += 1;
        let _ = writeln!(
            del,
            "{},{},{},{},{},{},{},{}",
            o.law, o.hub_rows, csv_field(live.as_deref().unwrap_or("")),
            d.and_then(|d| d.status.clone()).unwrap_or_default(),
            d.map(|d| d.status_conflict).unwrap_or(false),
            d.map(|d| d.rescinded_by).unwrap_or(0),
            d.and_then(|d| d.latest_rescind_date.clone()).unwrap_or_default(),
            verdict
        );
    }

    let dir = report_dir(data_dir)?;
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let law_path = dir.join(format!("pull_lat_{ts}.csv"));
    write_law_report(&law_path, outcomes)?;
    let del_path = dir.join(format!("delete_candidates_{ts}.csv"));
    std::fs::write(&del_path, del)?;
    let reparse: Vec<&str> = outcomes.iter().filter(|o| o.needs_reparse()).map(|o| o.law.as_str()).collect();
    let reparse_path = dir.join(format!("reparse_{ts}.txt"));
    if !reparse.is_empty() {
        std::fs::write(&reparse_path, reparse.join("\n") + "\n")?;
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for o in outcomes {
        *counts.entry(o.action.as_str()).or_default() += 1;
    }
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort();
    println!("\nSummary: {counts:?}");
    let held: usize = outcomes.iter().filter_map(|o| o.plan.as_ref()).map(|p| p.held.len()).sum();
    if held > 0 {
        println!("  {held} hub rows held for review (ambiguous match)");
    }
    if !candidates.is_empty() {
        let mut v: Vec<_> = verdicts.into_iter().collect();
        v.sort();
        println!("  delete candidates: {} {v:?} (never applied here; see --archive-laws)", candidates.len());
    }
    println!("Report: {}", law_path.display());
    println!("Delete candidates: {}", del_path.display());
    if !reparse.is_empty() {
        println!("Needs parse → reconcile → backfill: {} laws ({})", reparse.len(), reparse_path.display());
    }
    Ok(())
}

/// `pull-lat --laws …` / `pull-lat --stale`: report (default) or apply.
pub(crate) async fn cmd_pull_lat(
    data_dir: &Path,
    zenoh: &ZenohArgs,
    opts: &PullLatOpts,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    let pg = open_pg(pg_url).await?;
    let sync = connect(zenoh).await?;
    let duck = open_duck(data_dir)?;
    let benchmarks = benchmark_laws(&duck)?;

    let (laws, manifest) = if let Some(laws) = &opts.laws {
        let mut m = HashMap::new();
        for law in laws {
            for e in sync.query_lat_manifest(law, opts.timeout).await? {
                m.insert(e.law_name.clone(), manifest_entry(&e));
            }
        }
        (laws.clone(), m)
    } else if opts.stale {
        stale_scope(&pg, &sync, opts.timeout).await?
    } else {
        anyhow::bail!("specify --laws or --stale");
    };

    println!(
        "LAT sync ({}): {} laws, tenant {}{}",
        if opts.apply { "apply" } else { "dry run" },
        laws.len(),
        zenoh.tenant,
        opts.limit.map(|l| format!(", apply limit {l}")).unwrap_or_default()
    );
    let outcomes = run_pass(&pg, &sync, &benchmarks, &laws, &manifest, opts).await?;
    report_pass(data_dir, &sync, &duck, &outcomes, opts.timeout).await
}

/// Archive approved laws legal no longer holds (reversible).
pub(crate) async fn cmd_archive_laws(
    data_dir: &Path,
    zenoh: &ZenohArgs,
    laws: &[String],
    timeout: Duration,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    let pg = open_pg(pg_url).await?;
    let sync = connect(zenoh).await?;
    let benchmarks = benchmark_laws(&open_duck(data_dir)?)?;
    let hub_counts = pg.hub_law_row_counts().await?;
    for law in laws {
        let rows = hub_counts.get(law).copied().unwrap_or(0);
        let legal_rows: u64 = sync.query_lat_manifest(law, timeout).await?.iter().map(|m| m.row_count).sum();
        if benchmarks.contains(law) {
            eprintln!("  {law}: REFUSED (benchmark law)");
        } else if legal_rows > 0 {
            eprintln!("  {law}: REFUSED (legal serves {legal_rows} rows; not a delete candidate)");
        } else if rows == 0 {
            eprintln!("  {law}: nothing in the hub");
        } else {
            let n = pg.archive_law(law, NOT_IN_LEGAL).await?;
            eprintln!("  {law}: archived {n} rows (restore with --restore-laws)");
        }
    }
    Ok(())
}

/// Restore laws archived by `--archive-laws`.
pub(crate) async fn cmd_restore_laws(laws: &[String], pg_url: Option<&str>) -> anyhow::Result<()> {
    let pg = open_pg(pg_url).await?;
    for law in laws {
        let n = pg.restore_archived_law(law, NOT_IN_LEGAL).await?;
        eprintln!("  {law}: restored {n} rows");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(status: &str, conflict: bool, by: i64, date: Option<&str>) -> DuckLrt {
        DuckLrt { status: Some(status.into()), status_conflict: conflict, latest_rescind_date: date.map(String::from), rescinded_by: by }
    }

    #[test]
    fn delete_verdicts() {
        let revoked = Some("❌ Revoked / Repealed / Abolished");
        assert_eq!(delete_verdict(revoked, Some(&d("revoked", false, 3, Some("2002-12-28")))), "verified_revoked");
        // Records disagree (DuckDB says partial) → agent review
        assert_eq!(delete_verdict(revoked, Some(&d("partial", false, 3, Some("2002-12-28")))), "needs_review");
        assert_eq!(delete_verdict(revoked, Some(&d("revoked", true, 3, Some("2002-12-28")))), "needs_review");
        assert_eq!(delete_verdict(revoked, Some(&d("revoked", false, 0, None))), "needs_review");
        assert_eq!(delete_verdict(revoked, None), "needs_review");
        // In force but no LAT: legal's gap, never a deletion
        assert_eq!(delete_verdict(Some("✔ In force"), Some(&d("in_force", false, 0, None))), "legal_gap_in_force");
        // Legal has no record (e.g. regnal-year duplicate)
        assert_eq!(delete_verdict(None, None), "unknown_to_legal");
    }

    #[test]
    fn law_lists() {
        assert_eq!(parse_law_list("A, B,,C").unwrap(), vec!["A", "B", "C"]);
    }
}
