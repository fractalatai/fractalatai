//! LAT sync with sertantai-legal (fractalatai #62).
//!
//! Legal publishes a per-law manifest (`row_count`, `lat_hash`) so the hub can
//! detect drift without trusting fire-once sync events, plus an old→new
//! section_id rename map for its re-parses. This module holds the pure parts:
//! the agreed hash and the per-law diff plan that decides, row by row, what
//! happens to the hub's copy and the tier data hanging off it.
//!
//! Hash contract (agreed with legal 2026-09-26, vectors in
//! `data/lat_hash_vectors.json`): over every row the LAT queryable serves,
//! ordered by section_id bytewise, one line per row
//! `section_id \t sort_key \t normalise(text) \n` (NULL → ""), lowercase hex
//! SHA-256 of the UTF-8 concatenation.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

/// The Unicode White_Space property, spelled out so Rust and Postgres agree
/// (Postgres `\s` misses U+00A0, U+1680, U+2007, U+202F).
pub fn is_contract_whitespace(c: char) -> bool {
    matches!(c,
        '\u{0009}'..='\u{000D}' | '\u{0020}' | '\u{0085}' | '\u{00A0}' | '\u{1680}'
        | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}')
}

/// NFC → collapse runs of contract whitespace to one U+0020 → trim.
/// Zero-width characters (U+200B, U+FEFF) are not whitespace and are kept.
pub fn normalise(text: Option<&str>) -> String {
    let nfc: String = text.unwrap_or("").nfc().collect();
    let mut out = String::with_capacity(nfc.len());
    let mut pending_space = false;
    for c in nfc.chars() {
        if is_contract_whitespace(c) {
            pending_space = true;
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(c);
        }
    }
    out
}

/// One LAT row as served by legal (or held in the hub).
#[derive(Debug, Clone, PartialEq)]
pub struct LatRow {
    pub section_id: String,
    pub sort_key: Option<String>,
    pub text: Option<String>,
}

/// SHA-256 over the rows' contract lines, lowercase hex.
fn hash_lines(mut lines: Vec<(&str, String)>) -> String {
    lines.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let mut h = Sha256::new();
    for (_, line) in &lines {
        h.update(line.as_bytes());
    }
    format!("{:x}", h.finalize())
}

/// The agreed `lat_hash` for one law.
pub fn lat_hash(rows: &[LatRow]) -> String {
    hash_lines(
        rows.iter()
            .map(|r| {
                let line = format!(
                    "{}\t{}\t{}\n",
                    r.section_id,
                    r.sort_key.as_deref().unwrap_or(""),
                    normalise(r.text.as_deref())
                );
                (r.section_id.as_str(), line)
            })
            .collect(),
    )
}

/// One row for [`struct_hash`]: section_id and the rendered [`STRUCT_COLUMNS`] values.
pub type StructRow = (String, Vec<Option<String>>);

/// Structural LAT columns covered by `struct_hash`, in contract order.
pub const STRUCT_COLUMNS: &[&str] = &[
    "section_type", "hierarchy_path", "depth", "position", "part", "chapter", "heading_group",
    "provision", "paragraph", "sub_paragraph", "schedule", "extent_code", "language",
    "amendment_count", "modification_count", "commencement_count", "extent_count", "editorial_count",
];

/// `struct_hash` for one law: `section_id` then each of [`STRUCT_COLUMNS`]
/// (already rendered: NULL → "", integers in plain decimal, strings as-is),
/// tab-separated. Agreed with legal 2026-09-26 (vectors in `data/lat_hash_vectors.json`).
pub fn struct_hash(rows: &[StructRow]) -> String {
    hash_lines(
        rows.iter()
            .map(|(sid, vals)| {
                let mut line = sid.clone();
                for v in vals {
                    line.push('\t');
                    line.push_str(v.as_deref().unwrap_or(""));
                }
                line.push('\n');
                (sid.as_str(), line)
            })
            .collect(),
    )
}

/// One law in legal's manifest (`lat-manifest/{law}`).
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestEntry {
    pub law_name: String,
    pub row_count: u64,
    pub lat_hash: String,
    /// Absent until legal serves it
    pub struct_hash: Option<String>,
}

/// Render one Arrow cell as the contract string: NULL → None, integers in
/// plain decimal, strings as-is.
fn cell(col: &arrow::array::ArrayRef, row: usize) -> Option<String> {
    use arrow::array::{Array, AsArray};
    use arrow::datatypes::{DataType, Int16Type, Int32Type, Int64Type};
    if col.is_null(row) {
        return None;
    }
    match col.data_type() {
        DataType::Utf8 => Some(col.as_string::<i32>().value(row).to_string()),
        DataType::LargeUtf8 => Some(col.as_string::<i64>().value(row).to_string()),
        DataType::Utf8View => Some(col.as_string_view().value(row).to_string()),
        DataType::Int16 => Some(col.as_primitive::<Int16Type>().value(row).to_string()),
        DataType::Int32 => Some(col.as_primitive::<Int32Type>().value(row).to_string()),
        DataType::Int64 => Some(col.as_primitive::<Int64Type>().value(row).to_string()),
        _ => None,
    }
}

/// Rows (section_id, sort_key, text) from LAT batches as legal serves them.
pub fn lat_rows_from_batches(batches: &[arrow::record_batch::RecordBatch]) -> Result<Vec<LatRow>, String> {
    let mut rows = Vec::new();
    for b in batches {
        let col = |name: &str| b.column_by_name(name).ok_or_else(|| format!("LAT batch has no {name} column"));
        let (sid, sk, text) = (col("section_id")?, col("sort_key")?, col("text")?);
        for i in 0..b.num_rows() {
            let section_id = cell(sid, i).ok_or("LAT row with null section_id")?;
            rows.push(LatRow { section_id, sort_key: cell(sk, i), text: cell(text, i) });
        }
    }
    Ok(rows)
}

/// Rows for [`struct_hash`] from LAT batches; a column legal doesn't serve counts as NULL.
pub fn struct_rows_from_batches(
    batches: &[arrow::record_batch::RecordBatch],
) -> Result<Vec<StructRow>, String> {
    let mut rows = Vec::new();
    for b in batches {
        let sid = b.column_by_name("section_id").ok_or("LAT batch has no section_id column")?;
        let cols: Vec<_> = STRUCT_COLUMNS.iter().map(|c| b.column_by_name(c)).collect();
        for i in 0..b.num_rows() {
            let section_id = cell(sid, i).ok_or("LAT row with null section_id")?;
            rows.push((section_id, cols.iter().map(|c| c.and_then(|c| cell(c, i))).collect()));
        }
    }
    Ok(rows)
}

/// One entry of legal's rename log (`lat-renames/{law}`).
#[derive(Debug, Clone, PartialEq)]
pub struct Rename {
    pub old_section_id: String,
    pub new_section_id: Option<String>,
    pub status: RenameStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameStatus {
    /// Legal carried enrichment old → new
    Renamed,
    /// Text shared by unequal groups; legal carried nothing
    Ambiguous,
    /// No counterpart: a genuine removal
    Dropped,
}

impl RenameStatus {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "renamed" => Some(Self::Renamed),
            "ambiguous" => Some(Self::Ambiguous),
            "dropped" => Some(Self::Dropped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenameSource {
    /// From legal's rename log
    LegalMap,
    /// Fallback: unique exact match of normalised text within the law
    TextMatch,
}

/// What diff-apply does to one law's hub rows. Every hub and legal
/// section_id lands in exactly one bucket.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiffPlan {
    /// Same id, same normalised text: tier data kept, LAT columns refreshed
    pub unchanged: Vec<String>,
    /// Of `unchanged`, rows whose sort_key changed (metadata-only update)
    pub sort_key_changed: usize,
    /// Same id, different text: snapshot archived, tier data cleared, re-parse
    pub text_changed: Vec<String>,
    /// Hub id → legal id; tier data carried to the new id
    pub renamed: Vec<(String, String, RenameSource)>,
    /// Legal-only ids with no predecessor: new rows, need parse
    pub inserted: Vec<String>,
    /// Hub-only ids left untouched for review (ambiguous match)
    pub held: Vec<String>,
    /// Hub-only ids whose text is gone: archived, then removed
    pub archived: Vec<String>,
}

impl DiffPlan {
    /// True when applying the plan changes nothing but LAT metadata.
    pub fn is_noop(&self) -> bool {
        self.text_changed.is_empty()
            && self.renamed.is_empty()
            && self.inserted.is_empty()
            && self.held.is_empty()
            && self.archived.is_empty()
            && self.sort_key_changed == 0
    }

    /// Hub ids whose tier data must survive, paired with the id it lives under after apply.
    pub fn carried(&self) -> Vec<(String, String)> {
        self.unchanged
            .iter()
            .map(|s| (s.clone(), s.clone()))
            .chain(self.renamed.iter().map(|(o, n, _)| (o.clone(), n.clone())))
            .collect()
    }
}

/// Follow a rename chain (A→B, B→C) to its final id. Stops on a cycle.
fn resolve_chain(start: &str, map: &HashMap<&str, &str>) -> String {
    let mut cur = start;
    let mut seen = HashSet::new();
    while let Some(next) = map.get(cur) {
        if !seen.insert(cur) {
            break;
        }
        cur = next;
    }
    cur.to_string()
}

/// Plan the diff of one law: `hub` is the hub's current rows, `legal` the full
/// row set legal serves now, `renames` legal's rename log since the last apply
/// (oldest first). Legal's map is applied first; unique exact normalised-text
/// match is the fallback; anything unclear is held, never guessed.
pub fn plan_diff(hub: &[LatRow], legal: &[LatRow], renames: &[Rename]) -> DiffPlan {
    let mut plan = DiffPlan::default();
    let hub_by_id: HashMap<&str, &LatRow> = hub.iter().map(|r| (r.section_id.as_str(), r)).collect();
    let legal_by_id: HashMap<&str, &LatRow> = legal.iter().map(|r| (r.section_id.as_str(), r)).collect();

    // 1. Same id on both sides: compare text.
    for r in hub {
        if let Some(l) = legal_by_id.get(r.section_id.as_str()) {
            if normalise(r.text.as_deref()) == normalise(l.text.as_deref()) {
                if r.sort_key != l.sort_key {
                    plan.sort_key_changed += 1;
                }
                plan.unchanged.push(r.section_id.clone());
            } else {
                plan.text_changed.push(r.section_id.clone());
            }
        }
    }

    let mut hub_only: Vec<&str> = hub
        .iter()
        .map(|r| r.section_id.as_str())
        .filter(|s| !legal_by_id.contains_key(s))
        .collect();
    let mut legal_only: HashSet<&str> = legal
        .iter()
        .map(|r| r.section_id.as_str())
        .filter(|s| !hub_by_id.contains_key(s))
        .collect();

    // 2. Legal's rename log (resolved through chains) for hub-only ids.
    let chain: HashMap<&str, &str> = renames
        .iter()
        .filter(|r| r.status == RenameStatus::Renamed)
        .filter_map(|r| r.new_section_id.as_deref().map(|n| (r.old_section_id.as_str(), n)))
        .collect();
    let status_of: HashMap<&str, RenameStatus> =
        renames.iter().map(|r| (r.old_section_id.as_str(), r.status)).collect();

    let mut targets: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    let mut resolved: HashSet<&str> = HashSet::new();
    for &old in &hub_only {
        match status_of.get(old) {
            Some(RenameStatus::Renamed) => {
                let fin = resolve_chain(old, &chain);
                if legal_only.contains(fin.as_str()) {
                    targets.entry(fin).or_default().push(old);
                }
                // Target not legal-only (gone again, or already a hub id): fall back to text.
            }
            Some(RenameStatus::Ambiguous) => {
                plan.held.push(old.to_string());
                resolved.insert(old);
            }
            Some(RenameStatus::Dropped) => {
                plan.archived.push(old.to_string());
                resolved.insert(old);
            }
            None => {}
        }
    }
    for (new, olds) in targets {
        if olds.len() == 1 {
            plan.renamed.push((olds[0].to_string(), new.clone(), RenameSource::LegalMap));
            legal_only.remove(new.as_str());
        } else {
            // Two hub ids claiming one new id: don't guess
            plan.held.extend(olds.iter().map(|s| s.to_string()));
        }
        resolved.extend(olds);
    }
    hub_only.retain(|s| !resolved.contains(s));

    // 3. Fallback: unique exact match of normalised text among what's left.
    let mut hub_text: HashMap<String, Vec<&str>> = HashMap::new();
    for &sid in &hub_only {
        hub_text.entry(normalise(hub_by_id[sid].text.as_deref())).or_default().push(sid);
    }
    let mut legal_text: HashMap<String, Vec<&str>> = HashMap::new();
    for &sid in &legal_only {
        legal_text.entry(normalise(legal_by_id[sid].text.as_deref())).or_default().push(sid);
    }
    for (text, olds) in hub_text {
        let news = legal_text.get(&text).filter(|_| !text.is_empty());
        match news {
            Some(news) if olds.len() == 1 && news.len() == 1 => {
                plan.renamed.push((olds[0].to_string(), news[0].to_string(), RenameSource::TextMatch));
                legal_only.remove(news[0]);
            }
            Some(_) => plan.held.extend(olds.iter().map(|s| s.to_string())),
            None => plan.archived.extend(olds.iter().map(|s| s.to_string())),
        }
    }

    plan.inserted = legal_only.into_iter().map(String::from).collect();
    for v in [&mut plan.unchanged, &mut plan.text_changed, &mut plan.inserted, &mut plan.held, &mut plan.archived] {
        v.sort();
    }
    plan.renamed.sort();
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(sid: &str, sk: &str, text: &str) -> LatRow {
        LatRow { section_id: sid.into(), sort_key: Some(sk.into()), text: Some(text.into()) }
    }

    fn vectors() -> serde_json::Value {
        serde_json::from_str(include_str!("../data/lat_hash_vectors.json")).unwrap()
    }

    fn vector_rows(v: &serde_json::Value) -> Vec<LatRow> {
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| LatRow {
                section_id: r["section_id"].as_str().unwrap().into(),
                sort_key: r["sort_key"].as_str().map(String::from),
                text: r["text"].as_str().map(String::from),
            })
            .collect()
    }

    #[test]
    fn legal_vectors_match() {
        let v = vectors();
        for key in ["synthetic", "empty", "fixture_law"] {
            let rows = vector_rows(&v[key]);
            assert_eq!(rows.len() as u64, v[key]["row_count"].as_u64().unwrap(), "{key} row_count");
            assert_eq!(lat_hash(&rows), v[key]["lat_hash"].as_str().unwrap(), "{key} lat_hash");
        }
    }

    #[test]
    fn legal_struct_vectors_match() {
        let v = vectors();
        for key in ["synthetic", "empty", "fixture_law"] {
            let rows: Vec<(String, Vec<Option<String>>)> = v[key]["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    let vals = STRUCT_COLUMNS
                        .iter()
                        .map(|c| match &r[*c] {
                            serde_json::Value::Null => None,
                            serde_json::Value::String(s) => Some(s.clone()),
                            other => Some(other.to_string()),
                        })
                        .collect();
                    (r["section_id"].as_str().unwrap().to_string(), vals)
                })
                .collect();
            assert_eq!(struct_hash(&rows), v[key]["struct_hash"].as_str().unwrap(), "{key} struct_hash");
        }
    }

    #[test]
    fn normalise_contract() {
        assert_eq!(normalise(Some("  A person  must   not\tdeploy.\u{200B} ")), "A person must not deploy.\u{200B}");
        assert_eq!(normalise(Some("a\u{00A0}b\u{202F}c\u{2007}d\u{1680}e")), "a b c d e");
        assert_eq!(normalise(Some("\u{FEFF}x")), "\u{FEFF}x");
        assert_eq!(normalise(None), "");
        // NFC: e + combining acute → é
        assert_eq!(normalise(Some("cafe\u{0301}")), "caf\u{00E9}");
    }

    #[test]
    fn hash_ignores_row_order() {
        let a = [row("L:reg.2", "2", "b"), row("L:reg.1", "1", "a")];
        let b = [row("L:reg.1", "1", "a"), row("L:reg.2", "2", "b")];
        assert_eq!(lat_hash(&a), lat_hash(&b));
        assert_eq!(lat_hash(&[]), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn wester_ross_old_generation_rows() {
        // Hub: one old-generation row per article; legal: split paragraphs
        let hub = [
            row("L:reg.4(4)", "04", "4.—(1) Paragraphs (2) and (3) apply in order to further…"),
            row("L:reg.5(5)", "05", "5.—(1) The Scottish Ministers may … issue a permit"),
            row("L:reg.3(3)", "03", "3. For the purposes of this Order, the area protected"),
        ];
        let legal = [
            row("L:reg.3(3)", "03", "3.  For the purposes of this Order,  the area protected"),
            row("L:reg.4(1)", "041", "Paragraphs (2) and (3) apply in order to further…"),
            row("L:reg.4(2)", "042", "A person must not deploy (by any means) or use any fishing gear"),
            row("L:reg.4(4)", "044", "Paragraphs (2) and (3) do not apply to the deployment or use of—"),
            row("L:reg.5(1)", "051", "The Scottish Ministers may … issue a permit"),
        ];
        let p = plan_diff(&hub, &legal, &[]);
        assert_eq!(p.unchanged, vec!["L:reg.3(3)"]); // whitespace-only difference
        assert_eq!(p.text_changed, vec!["L:reg.4(4)"]); // id reused for different text
        assert_eq!(p.archived, vec!["L:reg.5(5)"]); // "5.—(1)" prefix: no exact match
        assert_eq!(p.inserted, vec!["L:reg.4(1)", "L:reg.4(2)", "L:reg.5(1)"]);
        assert!(p.renamed.is_empty() && p.held.is_empty());
    }

    #[test]
    fn legal_rename_map_first_then_text_fallback() {
        let hub = [
            row("L:reg.39(e)", "1", "any other prescribed matter"),
            row("L:reg.7", "2", "The operator must keep records"),
            row("L:reg.8", "3", "revoked"),
            row("L:reg.9", "4", "revoked"),
            row("L:reg.10", "5", "A notice must be given"),
        ];
        let legal = [
            row("L:reg.39(2)(e)", "1", "any other prescribed matter"),
            row("L:reg.7A", "2", "The operator must keep records"),
            row("L:reg.8A", "3", "revoked"),
            row("L:reg.9A", "4", "revoked"),
        ];
        let renames = [
            Rename { old_section_id: "L:reg.39(e)".into(), new_section_id: Some("L:reg.39(2)(e)".into()), status: RenameStatus::Renamed },
            Rename { old_section_id: "L:reg.10".into(), new_section_id: None, status: RenameStatus::Dropped },
        ];
        let p = plan_diff(&hub, &legal, &renames);
        assert_eq!(
            p.renamed,
            vec![
                ("L:reg.39(e)".into(), "L:reg.39(2)(e)".into(), RenameSource::LegalMap),
                ("L:reg.7".into(), "L:reg.7A".into(), RenameSource::TextMatch),
            ]
        );
        // Duplicate text ("revoked" ×2 on both sides) is never guessed
        assert_eq!(p.held, vec!["L:reg.8", "L:reg.9"]);
        assert_eq!(p.inserted, vec!["L:reg.8A", "L:reg.9A"]);
        assert_eq!(p.archived, vec!["L:reg.10"]);
        assert_eq!(p.carried().len(), 2);
    }

    #[test]
    fn rename_chain_and_collisions() {
        let hub = [row("L:a", "1", "x"), row("L:b", "2", "y"), row("L:c", "3", "z")];
        let legal = [row("L:a3", "1", "x"), row("L:bc", "2", "y2")];
        let renames = [
            Rename { old_section_id: "L:a".into(), new_section_id: Some("L:a2".into()), status: RenameStatus::Renamed },
            Rename { old_section_id: "L:a2".into(), new_section_id: Some("L:a3".into()), status: RenameStatus::Renamed },
            Rename { old_section_id: "L:b".into(), new_section_id: Some("L:bc".into()), status: RenameStatus::Renamed },
            Rename { old_section_id: "L:c".into(), new_section_id: Some("L:bc".into()), status: RenameStatus::Renamed },
        ];
        let p = plan_diff(&hub, &legal, &renames);
        assert_eq!(p.renamed, vec![("L:a".into(), "L:a3".into(), RenameSource::LegalMap)]);
        assert_eq!(p.held, vec!["L:b", "L:c"]);
        assert_eq!(p.inserted, vec!["L:bc"]);
    }

    #[test]
    fn duplicate_old_generation_row_is_archived() {
        // Same text under an old id and the current id: the old id goes
        let hub = [row("L:s.2(2)", "1", "Every employer must"), row("L:s.2", "1", "Every employer must")];
        let legal = [row("L:s.2", "1", "Every employer must")];
        let p = plan_diff(&hub, &legal, &[]);
        assert_eq!(p.unchanged, vec!["L:s.2"]);
        assert_eq!(p.archived, vec!["L:s.2(2)"]);
    }

    #[test]
    fn sort_key_only_is_metadata() {
        let hub = [row("L:a", "001", "x")];
        let legal = [row("L:a", "002", "x")];
        let p = plan_diff(&hub, &legal, &[]);
        assert_eq!(p.unchanged, vec!["L:a"]);
        assert_eq!(p.sort_key_changed, 1);
        assert!(!p.is_noop());
        assert!(plan_diff(&legal, &legal, &[]).is_noop());
    }

    #[test]
    fn rows_from_large_utf8_and_null_columns() {
        use arrow::array::{ArrayRef, Int32Array, LargeStringArray, NullArray};
        use arrow::datatypes::{DataType, Field, Schema};
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;
        let schema = Arc::new(Schema::new(vec![
            Field::new("section_id", DataType::LargeUtf8, false),
            Field::new("sort_key", DataType::LargeUtf8, true),
            Field::new("text", DataType::LargeUtf8, true),
            Field::new("depth", DataType::Int32, true),
            Field::new("part", DataType::Null, true),
        ]));
        let cols: Vec<ArrayRef> = vec![
            Arc::new(LargeStringArray::from(vec!["L:reg.1", "L:reg.2"])),
            Arc::new(LargeStringArray::from(vec![Some("1"), None])),
            Arc::new(LargeStringArray::from(vec![Some(" a "), None])),
            Arc::new(Int32Array::from(vec![Some(2), None])),
            Arc::new(NullArray::new(2)),
        ];
        let b = RecordBatch::try_new(schema, cols).unwrap();
        let rows = lat_rows_from_batches(&[b.clone()]).unwrap();
        assert_eq!(rows[0], row("L:reg.1", "1", " a "));
        assert_eq!(rows[1], LatRow { section_id: "L:reg.2".into(), sort_key: None, text: None });
        let s = struct_rows_from_batches(&[b]).unwrap();
        assert_eq!(s[0].1.len(), STRUCT_COLUMNS.len());
        let depth = STRUCT_COLUMNS.iter().position(|c| *c == "depth").unwrap();
        assert_eq!(s[0].1[depth].as_deref(), Some("2"));
        assert!(s[0].1.iter().enumerate().all(|(i, v)| i == depth || v.is_none()));
    }

    #[test]
    fn struct_hash_shape() {
        let rows = vec![("L:a".to_string(), vec![Some("article".to_string()), None, Some("2".to_string())])];
        let expected = format!("{:x}", Sha256::digest(b"L:a\tarticle\t\t2\n"));
        assert_eq!(struct_hash(&rows), expected);
    }
}
