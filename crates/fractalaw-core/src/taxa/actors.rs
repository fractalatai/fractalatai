//! Actor definitions and extraction for ESH legal text (UK domestic + EU retained).
//!
//! Identifies WHO is mentioned in legislative text, split into two groups:
//! - **Government actors**: Crown, authorities, agencies, ministers, devolved admins, EU institutions
//! - **Governed actors**: Businesses, individuals, specialists, supply-chain actors
//!
//! Actor definitions are loaded from `docs/actor-dictionary.yaml` (embedded at
//! compile time) and compiled into regex patterns on first use via `LazyLock`.
//!
//! Ported from `Taxa.ActorDefinitions`, `Taxa.ActorLib`, and `Taxa.DutyActor`.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

// ── Types ────────────────────────────────────────────────────────────

/// A single actor match: the structured label plus the raw keyword that matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorMatch {
    /// Structured label, e.g. "Org: Employer", "Gvt: Minister".
    pub label: String,
    /// The raw keyword text that the regex matched, e.g. "employer", "Secretary of State".
    /// Lowercased and trimmed of boundary characters.
    pub keyword: String,
    /// Byte offset of the keyword in the (padded) text.
    pub offset: usize,
}

/// Extraction result: governed + government actor labels found in text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedActors {
    /// Governed actor matches (businesses, individuals, supply-chain).
    pub governed: Vec<ActorMatch>,
    /// Government actor matches (authorities, agencies, ministers).
    pub government: Vec<ActorMatch>,
}

impl ExtractedActors {
    /// Governed actor labels only (backward-compatible with Vec<String> consumers).
    pub fn governed_labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = self.governed.iter().map(|m| m.label.clone()).collect();
        labels.sort();
        labels.dedup();
        labels
    }

    /// Government actor labels only (backward-compatible with Vec<String> consumers).
    pub fn government_labels(&self) -> Vec<String> {
        let mut labels: Vec<String> = self.government.iter().map(|m| m.label.clone()).collect();
        labels.sort();
        labels.dedup();
        labels
    }
}

// ── Blacklist ────────────────────────────────────────────────────────

static BLACKLIST: &[&str] = &[
    r"local authority collected municipal waste",
    r"[Pp]ublic (?:nature|sewer|importance|functions?|interest|[Ss]ervices)",
    r"[Rr]epresentatives? of",
    r"(?i)agency workers?",
    r"(?i)temporary work agency",
];

static BLACKLIST_COMPILED: LazyLock<Vec<Regex>> =
    LazyLock::new(|| BLACKLIST.iter().map(|p| Regex::new(p).unwrap()).collect());

fn apply_blacklist(text: &str) -> String {
    let mut result = text.to_string();
    for re in BLACKLIST_COMPILED.iter() {
        result = re.replace_all(&result, "").to_string();
    }
    result
}

// ── YAML-driven actor dictionary ────────────────────────────────────

/// Raw YAML actor definition (deserialized from docs/actor-dictionary.yaml).
#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ActorDef {
    label: String,
    #[serde(rename = "type")]
    actor_type: String,
    #[serde(default)]
    regex_patterns: Vec<String>,
    #[serde(default)]
    triggers: Vec<String>,
    #[serde(default)]
    drrp_keywords: Vec<String>,
    #[serde(default)]
    families: Vec<String>,
    #[serde(default)]
    category: String,
}

/// Compiled dictionary: all actor patterns parsed from YAML and ready to match.
struct CompiledDictionary {
    /// Core government actor patterns (no family gate).
    government: Vec<(String, Regex)>,
    /// Core governed actor patterns (no family gate).
    governed: Vec<(String, Regex)>,
    /// Family-gated specialist governed patterns.
    /// Key = family prefix to match with `starts_with`, value = compiled patterns.
    specialist: HashMap<String, Vec<(String, Regex)>>,
    /// Downcased keywords for `has_government_actor()` in duty_patterns.
    government_keywords: Vec<String>,
    /// All labels from every entry (core + specialist + trigger-only).
    all_labels: HashSet<String>,
}

/// The YAML file, embedded at compile time.
static ACTOR_YAML: &str = include_str!("../../../../docs/actor-dictionary.yaml");

/// The single compiled dictionary, built on first access.
static DICTIONARY: LazyLock<CompiledDictionary> = LazyLock::new(|| {
    let defs: Vec<ActorDef> =
        serde_yaml::from_str(ACTOR_YAML).expect("actor-dictionary.yaml parse error");

    let mut government = Vec::new();
    let mut governed = Vec::new();
    let mut specialist: HashMap<String, Vec<(String, Regex)>> = HashMap::new();
    let mut government_keywords = Vec::new();
    let mut all_labels = HashSet::new();

    for def in &defs {
        all_labels.insert(def.label.clone());

        // Collect drrp_keywords from government-type actors.
        if def.actor_type == "government" {
            for kw in &def.drrp_keywords {
                if !government_keywords.contains(kw) {
                    government_keywords.push(kw.clone());
                }
            }
        }

        // Skip entries with no regex patterns (trigger-only for LLM matching).
        if def.regex_patterns.is_empty() {
            continue;
        }

        // Compile each regex pattern with boundary wrappers.
        let compiled: Vec<(String, Regex)> = def
            .regex_patterns
            .iter()
            .map(|pat| {
                let full = format!(r"(?:[\s[:punct:]]){pat}(?:[\s[:punct:]])");
                let re = Regex::new(&full)
                    .unwrap_or_else(|e| panic!("bad regex for '{}': {e}", def.label));
                (def.label.clone(), re)
            })
            .collect();

        // Route to the right bucket.
        if !def.families.is_empty() {
            // Specialist family-gated pattern.
            for family in &def.families {
                specialist
                    .entry(family.clone())
                    .or_default()
                    .extend(compiled.iter().cloned());
            }
        } else if def.actor_type == "government" {
            government.extend(compiled);
        } else {
            governed.extend(compiled);
        }
    }

    CompiledDictionary {
        government,
        governed,
        specialist,
        government_keywords,
        all_labels,
    }
});

// ── Public API ───────────────────────────────────────────────────────

/// All valid actor labels from every pattern library (core + specialist + trigger-only).
///
/// Used by Tier 3 LLM to validate that returned labels match the dictionary.
pub fn all_actor_labels() -> HashSet<String> {
    DICTIONARY.all_labels.clone()
}

/// Downcased government keywords for `has_government_actor()` in duty_patterns.
pub fn government_keywords() -> Vec<String> {
    DICTIONARY.government_keywords.clone()
}

/// Extract all actors (governed + government) from text.
///
/// Applies the blacklist first, then runs each pattern library.
/// Matched text is progressively removed to avoid duplicate matches.
pub fn extract_actors(text: &str) -> ExtractedActors {
    let cleaned = apply_blacklist(text);
    ExtractedActors {
        governed: run_patterns(&cleaned, &DICTIONARY.governed),
        government: run_patterns(&cleaned, &DICTIONARY.government),
    }
}

/// Extract all actors with family-gated specialist patterns.
///
/// Runs core patterns (same as `extract_actors`) plus any specialist
/// governed actor patterns that match the law family prefix.
pub fn extract_actors_for_family(text: &str, family: Option<&str>) -> ExtractedActors {
    let cleaned = apply_blacklist(text);
    let mut governed = run_patterns(&cleaned, &DICTIONARY.governed);

    if let Some(fam) = family {
        // Check each specialist family key — match if the law family starts with
        // the key (e.g., "OH&S: Offshore Safety" starts with "OH&S: Offshore")
        // or is an exact match (e.g., "PUBLIC" == "PUBLIC").
        for (family_key, patterns) in &DICTIONARY.specialist {
            if fam.starts_with(family_key.as_str()) || fam == family_key {
                let mut extra = run_patterns(&cleaned, patterns);
                governed.append(&mut extra);
            }
        }
        governed.sort_by(|a, b| a.label.cmp(&b.label));
        governed.dedup_by(|a, b| a.label == b.label);
    }

    ExtractedActors {
        governed,
        government: run_patterns(&cleaned, &DICTIONARY.government),
    }
}

/// Extract only governed actors from text.
pub fn extract_governed(text: &str) -> Vec<ActorMatch> {
    let cleaned = apply_blacklist(text);
    run_patterns(&cleaned, &DICTIONARY.governed)
}

/// Extract only government actors from text.
pub fn extract_government(text: &str) -> Vec<ActorMatch> {
    let cleaned = apply_blacklist(text);
    run_patterns(&cleaned, &DICTIONARY.government)
}

// ── Internals ────────────────────────────────────────────────────────

fn run_patterns(text: &str, patterns: &[(String, Regex)]) -> Vec<ActorMatch> {
    // Pad with spaces so boundary patterns (?:[\s[:punct:]]) match at
    // start/end of string. text_cleaner::clean() trims whitespace, so
    // keywords like "Employer shall..." at position 0 would otherwise
    // fail the leading boundary check.
    let padded = format!(" {text} ");
    let mut remaining = padded.clone();
    let mut found = Vec::new();
    for (label, regex) in patterns {
        if let Some(m) = regex.find(&remaining) {
            // The match includes boundary chars — trim them to get the keyword.
            let raw = m.as_str();
            let keyword = raw.trim().trim_matches(|c: char| c.is_ascii_punctuation());
            // Offset in the original padded text (approximate — good enough for
            // distance calculations since we pad with 1 space).
            let offset = padded.find(keyword).unwrap_or(m.start());
            found.push(ActorMatch {
                label: label.to_string(),
                keyword: keyword.to_lowercase(),
                offset,
            });
            // Remove first match to prevent duplicate detection
            remaining = regex.replace(&remaining, "").to_string();
        }
    }
    found.sort_by(|a, b| a.label.cmp(&b.label));
    found.dedup_by(|a, b| a.label == b.label);
    found
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: check if an actor list contains a label.
    fn has_label(actors: &[ActorMatch], label: &str) -> bool {
        actors.iter().any(|a| a.label == label)
    }

    /// Helper: check if any actor label contains a substring.
    fn any_label_contains(actors: &[ActorMatch], substr: &str) -> bool {
        actors.iter().any(|a| a.label.contains(substr))
    }

    #[test]
    fn extract_employer() {
        let actors = extract_actors(" The employer shall ensure safety. ");
        assert!(has_label(&actors.governed, "Org: Employer"));
    }

    #[test]
    fn extract_employer_captures_keyword() {
        let actors = extract_actors(" The employer shall ensure safety. ");
        let m = actors
            .governed
            .iter()
            .find(|a| a.label == "Org: Employer")
            .unwrap();
        assert_eq!(m.keyword, "employer");
    }

    #[test]
    fn extract_secretary_of_state() {
        let actors = extract_actors(" The Secretary of State may make regulations. ");
        assert!(has_label(&actors.government, "Gvt: Minister"));
    }

    #[test]
    fn extract_secretary_of_state_captures_keyword() {
        let actors = extract_actors(" The Secretary of State may make regulations. ");
        let m = actors
            .government
            .iter()
            .find(|a| a.label == "Gvt: Minister")
            .unwrap();
        assert_eq!(m.keyword, "secretary of state");
    }

    #[test]
    fn extract_hse() {
        let actors = extract_actors(" The Health and Safety Executive shall. ");
        assert!(any_label_contains(
            &actors.government,
            "Health and Safety Executive"
        ));
    }

    #[test]
    fn extract_multiple_actors() {
        let text = " The employer shall consult the inspector and the employee. ";
        let actors = extract_actors(text);
        assert!(has_label(&actors.governed, "Org: Employer"));
        assert!(has_label(&actors.governed, "Ind: Employee"));
    }

    #[test]
    fn blacklist_removes_false_positives() {
        // "public interest" should be blacklisted
        let actors = extract_actors(" This is in the public interest. ");
        assert!(!has_label(&actors.governed, "Public"));
    }

    #[test]
    fn empty_text_returns_empty() {
        let actors = extract_actors("");
        assert!(actors.governed.is_empty());
        assert!(actors.government.is_empty());
    }

    #[test]
    fn extract_contractor() {
        let actors = extract_actors(" The contractor shall comply with requirements. ");
        assert!(any_label_contains(&actors.governed, "Contractor"));
    }

    #[test]
    fn extract_local_authority() {
        let actors = extract_actors(" The local authority may issue a notice. ");
        assert!(any_label_contains(&actors.government, "Local"));
    }

    #[test]
    fn extract_maritime_and_coastguard_agency() {
        let actors =
            extract_actors(" send the arrangements to the Maritime and Coastguard Agency. ");
        assert!(has_label(
            &actors.government,
            "Gvt: Agency: Maritime and Coastguard Agency"
        ));
    }

    #[test]
    fn extract_oil_and_gas_authority() {
        let actors = extract_actors(
            " before the submission of a field development plan to the Oil and Gas Authority ",
        );
        assert!(has_label(
            &actors.government,
            "Gvt: Agency: Oil and Gas Authority"
        ));
    }

    #[test]
    fn extract_nsta() {
        let actors = extract_actors(" a plan submitted to the NSTA for approval ");
        assert!(has_label(
            &actors.government,
            "Gvt: Agency: Oil and Gas Authority"
        ));
    }

    #[test]
    fn extract_dept_enterprise_trade_investment() {
        let actors = extract_actors(
            " Sealed with the Official Seal of the Department of Enterprise, Trade and Investment ",
        );
        assert!(has_label(
            &actors.government,
            "Gvt: Ministry: Department of Enterprise, Trade and Investment"
        ));
    }

    // ── Backward-compat label accessors ─────────────────────────────

    #[test]
    fn governed_labels_returns_sorted_strings() {
        let actors = extract_actors(" The employer shall consult the employee. ");
        let labels = actors.governed_labels();
        assert!(labels.contains(&"Org: Employer".to_string()));
        assert!(labels.contains(&"Ind: Employee".to_string()));
    }

    // ── Boundary matching tests ─────────────────────────────────────

    #[test]
    fn keyword_at_start_of_string() {
        let actors = extract_actors("Employer shall ensure safety.");
        assert!(
            has_label(&actors.governed, "Org: Employer"),
            "keyword at start of string should still be extracted, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn keyword_at_end_of_string() {
        let actors = extract_actors(" duties of the employer");
        assert!(
            has_label(&actors.governed, "Org: Employer"),
            "keyword at end of string should still be extracted, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn agency_worker_not_government_agency() {
        let actors = extract_actors(
            " Where, in the case of an individual agency worker, the taking \
              of any other action the hirer is required to take. ",
        );
        assert!(
            !has_label(&actors.government, "Gvt: Agency"),
            "agency worker should not be classified as Gvt: Agency, got: {:?}",
            actors.government
        );
    }

    #[test]
    fn temporary_work_agency_not_government_agency() {
        let actors = extract_actors(
            " the hirer shall inform the temporary work agency, who shall \
              then end the supply of the agency worker. ",
        );
        assert!(
            !has_label(&actors.government, "Gvt: Agency"),
            "temporary work agency should not be classified as Gvt: Agency, got: {:?}",
            actors.government
        );
    }

    // ── Family-gated specialist actors ───────────────────────────────

    #[test]
    fn licensee_extracted_for_offshore_family() {
        let text = "The licensee shall ensure that any operator is capable.";
        let actors = extract_actors_for_family(text, Some("OH&S: Offshore Safety"));
        assert!(
            has_label(&actors.governed, "Offshore: Licensee"),
            "licensee should be extracted for offshore family, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn licensee_not_extracted_without_family() {
        let text = "The licensee shall ensure that any operator is capable.";
        let actors = extract_actors(text);
        assert!(
            !has_label(&actors.governed, "Offshore: Licensee"),
            "licensee should not be extracted without family, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn licensee_not_extracted_for_other_family() {
        let text = "The licensee shall ensure that any operator is capable.";
        let actors = extract_actors_for_family(text, Some("AGRICULTURE"));
        assert!(
            !has_label(&actors.governed, "Offshore: Licensee"),
            "licensee should not be extracted for AGRICULTURE, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn offshore_family_still_extracts_core_actors() {
        let text = "The employer shall ensure safety. The licensee must comply.";
        let actors = extract_actors_for_family(text, Some("OH&S: Offshore Safety"));
        assert!(
            has_label(&actors.governed, "Org: Employer"),
            "core actors should still be extracted, got: {:?}",
            actors.governed
        );
        assert!(
            has_label(&actors.governed, "Offshore: Licensee"),
            "specialist actors should also be extracted, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn family_none_same_as_extract_actors() {
        let text = "The employer shall ensure safety.";
        let with_none = extract_actors_for_family(text, None);
        let without = extract_actors(text);
        assert_eq!(with_none.governed_labels(), without.governed_labels());
        assert_eq!(with_none.government_labels(), without.government_labels());
    }

    // ── PUBLIC family-gated specialist actors ────────────────────────

    #[test]
    fn provider_extracted_for_public_family() {
        let text =
            "A provider of a Part 3 service must carry out the first children's access assessment.";
        let actors = extract_actors_for_family(text, Some("PUBLIC"));
        assert!(
            has_label(&actors.governed, "Public: Provider"),
            "provider should be extracted for PUBLIC family, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn provider_not_extracted_without_family() {
        let text = "A provider of a Part 3 service must carry out the assessment.";
        let actors = extract_actors(text);
        assert!(
            !has_label(&actors.governed, "Public: Provider"),
            "provider should not be extracted without family, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn keeper_extracted_for_public_family() {
        let text = "The keeper of a dog shall ensure it is under control.";
        let actors = extract_actors_for_family(text, Some("PUBLIC"));
        assert!(
            has_label(&actors.governed, "Public: Keeper"),
            "keeper should be extracted for PUBLIC family, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn dealer_extracted_for_public_family() {
        let text = "A registered firearms dealer shall comply with this requirement.";
        let actors = extract_actors_for_family(text, Some("PUBLIC"));
        assert!(
            has_label(&actors.governed, "Public: Dealer"),
            "dealer should be extracted for PUBLIC family, got: {:?}",
            actors.governed
        );
    }

    // ── EU actors ─────────────────────────────────────────────────

    #[test]
    fn extract_member_states() {
        let actors = extract_actors(" Member States shall ensure compliance. ");
        assert!(has_label(&actors.government, "EU: Member State"));
    }

    #[test]
    fn extract_member_state_singular() {
        let actors = extract_actors(" Each Member State shall designate an authority. ");
        assert!(has_label(&actors.government, "EU: Member State"));
    }

    #[test]
    fn extract_echa() {
        let actors =
            extract_actors(" The applicant shall submit to the European Chemicals Agency. ");
        assert!(has_label(&actors.government, "EU: Agency: ECHA"));
    }

    #[test]
    fn extract_echa_abbreviation() {
        let actors = extract_actors(" ECHA shall publish the decision. ");
        assert!(has_label(&actors.government, "EU: Agency: ECHA"));
    }

    #[test]
    fn extract_registrant() {
        let actors = extract_actors(" The registrant shall submit a registration dossier. ");
        assert!(has_label(&actors.governed, "SC: Registrant"));
    }

    #[test]
    fn extract_downstream_user() {
        let actors = extract_actors(" A downstream user shall identify applicable conditions. ");
        assert!(has_label(&actors.governed, "SC: Downstream User"));
    }

    #[test]
    fn extract_applicant() {
        let actors = extract_actors(" The applicant shall provide sufficient information. ");
        assert!(has_label(&actors.governed, "SC: Applicant"));
    }

    #[test]
    fn extract_authorised_representative() {
        let actors = extract_actors(" An authorised representative shall fulfil the obligations. ");
        assert!(has_label(&actors.governed, "SC: Authorised Representative"));
    }

    #[test]
    fn extract_notified_body() {
        let actors = extract_actors(" The notified body shall assess conformity. ");
        assert!(has_label(&actors.governed, "SC: Notified Body"));
    }

    #[test]
    fn extract_distributor() {
        let actors = extract_actors(" A distributor shall verify the labelling. ");
        assert!(has_label(&actors.governed, "SC: Distributor"));
    }

    #[test]
    fn public_specialists_not_extracted_for_ohs() {
        let text = "The provider shall ensure the keeper is informed.";
        let actors = extract_actors_for_family(text, Some("OH&S: Occupational / Personal Safety"));
        assert!(
            !has_label(&actors.governed, "Public: Provider"),
            "provider should not be extracted for OH&S, got: {:?}",
            actors.governed
        );
        assert!(
            !has_label(&actors.governed, "Public: Keeper"),
            "keeper should not be extracted for OH&S, got: {:?}",
            actors.governed
        );
    }

    #[test]
    fn all_actor_labels_coverage() {
        let labels = all_actor_labels();
        // Should contain known labels from all pattern libraries
        assert!(labels.contains("Org: Employer"));
        assert!(labels.contains("Ind: Employee"));
        assert!(labels.contains("Gvt: Minister"));
        assert!(labels.contains("Gvt: Agency: Health and Safety Executive"));
        assert!(labels.contains("Ind: Responsible Person"));
        // Specialist patterns
        assert!(labels.contains("Offshore: Licensee"));
        assert!(labels.contains("Public: Keeper"));
        // Should have a reasonable count (50+)
        assert!(
            labels.len() > 50,
            "expected 50+ labels, got {}",
            labels.len()
        );
    }

    // ── New entity extraction tests ─────────────────────────────────

    #[test]
    fn extract_nda() {
        let actors = extract_actors(" The NDA must prepare a plan. ");
        assert!(has_label(&actors.government, "Gvt: Agency: NDA"));
    }

    #[test]
    fn extract_authorised_person() {
        let actors = extract_actors(" An authorised person must consult. ");
        assert!(has_label(&actors.governed, "Spc: Authorised Person"));
    }

    #[test]
    fn extract_scheme_administrator() {
        let actors = extract_actors(" The scheme administrator must publish. ");
        assert!(has_label(&actors.governed, "Spc: Administrator"));
    }

    #[test]
    fn extract_compliance_body() {
        let actors = extract_actors(" A compliance body must verify. ");
        assert!(has_label(&actors.governed, "Spc: Compliance Body"));
    }

    #[test]
    fn extract_certification_body() {
        let actors = extract_actors(" A certification body must provide details. ");
        assert!(has_label(&actors.governed, "Spc: Certification Body"));
    }

    #[test]
    fn extract_responsible_undertaking() {
        let actors = extract_actors(" The responsible undertaking must comply. ");
        assert!(has_label(&actors.governed, "Org: Responsible Undertaking"));
    }

    #[test]
    fn extract_manufacturers_plural() {
        let actors = extract_actors(" Manufacturers must ensure compliance. ");
        assert!(has_label(&actors.governed, "SC: Manufacturer"));
    }

    #[test]
    fn authorised_person_before_generic_person() {
        // Spc: Authorised Person should match before generic Ind: Person
        let actors = extract_actors(" The authorised person must inspect the premises. ");
        assert!(
            has_label(&actors.governed, "Spc: Authorised Person"),
            "should extract Spc: Authorised Person, got: {:?}",
            actors.governed.iter().map(|a| &a.label).collect::<Vec<_>>()
        );
    }
}
