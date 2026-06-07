//! Actor definitions and extraction for ESH legal text (UK domestic + EU retained).
//!
//! Identifies WHO is mentioned in legislative text, split into two groups:
//! - **Government actors**: Crown, authorities, agencies, ministers, devolved admins, EU institutions
//! - **Governed actors**: Businesses, individuals, specialists, supply-chain actors
//!
//! Ported from `Taxa.ActorDefinitions`, `Taxa.ActorLib`, and `Taxa.DutyActor`.

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

// ── Pattern definitions ──────────────────────────────────────────────
// We use a flat slice of (label, pattern) tuples compiled into LazyLock
// Regex objects. The boundary wrapper `(?:[\s[:punct:]])` replaces the
// Elixir `[[:blank:][:punct:]]` POSIX class.

macro_rules! actor {
    ($label:expr, $pat:expr) => {
        ($label, $pat)
    };
}

/// Government actor definitions (authorities, agencies, ministers, etc.)
/// Sorted roughly by specificity (more specific first).
const GOVERNMENT_DEFS: &[(&str, &str)] = &[
    actor!("Crown", r"(?:[\s[:punct:]])Crown(?:[\s[:punct:]])"),
    actor!(
        "Gvt: Minister: Secretary of State for Defence",
        r"(?:[\s[:punct:]])Secretary of State for Defence(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Minister: Secretary of State for Transport",
        r"(?:[\s[:punct:]])Secretary of State for Transport(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Minister: Attorney General",
        r"(?:[\s[:punct:]])Attorney General(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Minister",
        r"(?:[\s[:punct:]])(?:Secretary of State|[Mm]inisters?)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Health and Safety Executive for Northern Ireland",
        r"(?:[\s[:punct:]])(?:Health and Safety Executive for Northern Ireland|HSENI)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Health and Safety Executive",
        r"(?:[\s[:punct:]])(?:Health and Safety Executive|[Tt]he Executive)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Environment Agency",
        r"(?:[\s[:punct:]])Environment Agency(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Scottish Environment Protection Agency",
        r"(?:[\s[:punct:]])(?:Scottish Environment Protection Agency|SEPA)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Office for Nuclear Regulation",
        r"(?:[\s[:punct:]])(?:Office for Nuclear Regulations?|ONR)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Office for Environmental Protection",
        r"(?:[\s[:punct:]])(?:Office for Environmental Protection|OEP)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Office of Rail and Road",
        r"(?:[\s[:punct:]])Office of Rail and Road?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: OFCOM",
        r"(?:[\s[:punct:]])(?:Office of Communications?|OFCOM)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Natural Resources Body for Wales",
        r"(?:[\s[:punct:]])Natural Resources Body for Wales(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Maritime and Coastguard Agency",
        r"(?:[\s[:punct:]])(?:Maritime and Coastguard Agency|MCA)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency: Oil and Gas Authority",
        r"(?:[\s[:punct:]])(?:Oil and Gas Authority|North Sea Transition Authority|OGA|NSTA)(?:[\s[:punct:]])"
    ),
    actor!(
        "EU: Agency: ECHA",
        r"(?:[\s[:punct:]])(?:European Chemicals Agency|ECHA)(?:[\s[:punct:]])"
    ),
    actor!(
        "EU: Agency: EFSA",
        r"(?:[\s[:punct:]])(?:European Food Safety Authority|EFSA)(?:[\s[:punct:]])"
    ),
    actor!(
        "EU: Agency: EEA",
        r"(?:[\s[:punct:]])(?:European Environment Agency|EEA)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Agency",
        r"(?:[\s[:punct:]])[Aa]gency(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Enforcement",
        r"(?:[\s[:punct:]])(?:[Rr]egulati?on?r?y?|[Ee]nforce?(?:ment|ing)) [Aa]uthority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Local",
        r"(?:[\s[:punct:]])(?:[Ll]ocal [Aa]uthority?i?e?s?|council of a county|(?:county|district)(?: borough | )council|London Borough Council|council constituted)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Planning",
        r"(?:[\s[:punct:]])[Pp]lanning [Aa]uthority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Fire and Rescue",
        r"(?:[\s[:punct:]])(?:[Ff]ire and [Rr]escue [Aa]uthority?i?e?s?|[Ff]ire [Aa]uthority?i?e?s?)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Harbour",
        r"(?:[\s[:punct:]])(?:[Hh]arbour [Aa]uthority?i?e?s?|harbour master)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Licensing",
        r"(?:[\s[:punct:]])[Ll]icen[cs]ing [Aa]uthority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Waste",
        r"(?:[\s[:punct:]])(?:[Ww]aste collection|[Ww]aste disposal|[Dd]isposal) [Aa]uthority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Public",
        r"(?:[\s[:punct:]])[Pp]ublic [Aa]uthority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Traffic",
        r"(?:[\s[:punct:]])[Tt]raffic [Aa]uthority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority: Market",
        r"(?:[\s[:punct:]])(?:market surveillance|weights and measures) authority?i?e?s?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Authority",
        r"(?:[\s[:punct:]])(?:(?:[Tt]he|[Aa]n|appropriate|allocating|[Cc]ompetent|[Dd]esignated) authority?i?e?s?|[Rr]egulators?|[Mm]onitoring [Aa]uthority?i?e?s?|that authority)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Commissioners",
        r"(?:[\s[:punct:]])[Cc]ommissioners(?:[\s[:punct:]])"
    ),
    actor!(
        "EU: Commission",
        r"(?:[\s[:punct:]])[Cc]ommission(?:[\s[:punct:]])"
    ),
    actor!(
        "EU: Member State",
        r"(?:[\s[:punct:]])[Mm]ember [Ss]tates?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Officer",
        r"(?:[\s[:punct:]])(?:[Aa]uthorised [Oo]fficer|[Oo]fficer of a local authority|[Oo]fficer)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Judiciary",
        r"(?:[\s[:punct:]])(?:court|[Jj]ustice of the [Pp]eace|[Tt]ribunal|[Ss]heriff|[Mm]agistrate|prosecutor|Lord Advocate)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Emergency Services: Police",
        r"(?:[\s[:punct:]])(?:[Cc]onstable|[Cc]hief(?: officer | )of [Pp]olice|police force)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Emergency Services",
        r"(?:[\s[:punct:]])[Ee]mergency [Ss]ervices?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Appropriate Person",
        r"(?:[\s[:punct:]])[Aa]ppropriate [Pp]ersons?(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Ministry: Treasury",
        r"(?:[\s[:punct:]])[Tt]reasury(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Ministry: HMRC",
        r"(?:[\s[:punct:]])(?:customs officer|Her Majesty[''\u{2019}]s Commissioners for Revenue and Customs|Her Majesty's Revenue and Customs)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Ministry: Ministry of Defence",
        r"(?:[\s[:punct:]])Ministry of Defence(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Ministry: Department of Enterprise, Trade and Investment",
        r"(?:[\s[:punct:]])Department of Enterprise,? Trade and Investment(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Ministry",
        r"(?:[\s[:punct:]])(?:[Mm]inistry|[Tt]he Department)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Devolved Admin: National Assembly for Wales",
        r"(?:[\s[:punct:]])(?:National Assembly for Wales|Senedd|Welsh Parliament)(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Devolved Admin: Scottish Parliament",
        r"(?:[\s[:punct:]])Scottish Parliament(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Devolved Admin: Northern Ireland Assembly",
        r"(?:[\s[:punct:]])Northern Ireland Assembly(?:[\s[:punct:]])"
    ),
    actor!(
        "Gvt: Devolved Admin",
        r"(?:[\s[:punct:]])Assembly(?:[\s[:punct:]])"
    ),
];

/// Governed actor definitions (businesses, individuals, supply chain, etc.)
const GOVERNED_DEFS: &[(&str, &str)] = &[
    actor!(
        "HM Forces",
        r"(?:[\s[:punct:]])(?:(?:His|Her) Majesty[''\u{2019}]s forces|armed forces)(?:[\s[:punct:]])"
    ),
    actor!(
        "Org: Employer",
        r"(?:[\s[:punct:]])[Ee]mployers?(?:[\s[:punct:]])"
    ),
    actor!(
        "Org: Owner",
        r"(?:[\s[:punct:]])(?:[Oo]wners?|mine owner|owner of a non-production installation|installation owner)(?:[\s[:punct:]])"
    ),
    actor!(
        "Org: Occupier",
        r"(?:[\s[:punct:]])(?:[Oo]ccupiers?|[Pp]erson who is in occupation)(?:[\s[:punct:]])"
    ),
    actor!(
        "Org: Company",
        r"(?:[\s[:punct:]])(?:[Cc]ompany?i?e?s?|[Bb]usinesse?s?|[Ee]nterprises?|[Bb]ody?i?e?s? corporate)(?:[\s[:punct:]])"
    ),
    actor!(
        "Operator",
        r"(?:[\s[:punct:]])(?:[Oo]perators?|(?:berth|mine|well|economic|meter)[\s-]operator|operator of a production installation)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Employee",
        r"(?:[\s[:punct:]])[Ee]mployees?(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Worker",
        r"(?:[\s[:punct:]])(?:[Ww]orkers?|[Ww]orkmen|(?:members of the )?[Ww]orkforce)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Self-employed Worker",
        r"(?:[\s[:punct:]])[Ss]elf-employed (?:[Pp]ersons?|diver)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Responsible Person",
        r"(?:[\s[:punct:]])[Rr]esponsible [Pp]ersons?(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Competent Person",
        r"(?:[\s[:punct:]])(?:[Cc]ompetent [Pp]ersons?|person who is competent)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Duty Holder",
        r"(?:[\s[:punct:]])(?:[Dd]uty [Hh]olders?|[Dd]utyholder)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Manager",
        r"(?:[\s[:punct:]])(?:managers?|mine manager|manager of a mine|installation manager)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Supervisor",
        r"(?:[\s[:punct:]])(?:[Ss]upervisor|[Pp]erson in control|individual in charge)(?:[\s[:punct:]])"
    ),
    actor!(
        "Ind: Person",
        r"(?:[\s[:punct:]])(?:[Pp]ersons?|[Ii]ndividual)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Downstream User",
        r"(?:[\s[:punct:]])[Dd]ownstream [Uu]sers?(?:[\s[:punct:]])"
    ),
    actor!("Ind: User", r"(?:[\s[:punct:]])[Uu]sers?(?:[\s[:punct:]])"),
    actor!(
        "Spc: Inspector",
        r"(?:[\s[:punct:]])(?:[Uu]ser inspectorate|[Ii]nspectors?|[Vv]erifier|[Ww]ell examiner)(?:[\s[:punct:]])"
    ),
    actor!(
        "Spc: Employees' Representative",
        r"(?:[\s[:punct:]])(?:[Ee]mployees' representative|[Ss]afety representatives?|[Tt]rade [Uu]nions? representatives?)(?:[\s[:punct:]])"
    ),
    actor!(
        "Spc: Trade Union",
        r"(?:[\s[:punct:]])[Tt]rade [Uu]nions?(?:[\s[:punct:]])"
    ),
    actor!(
        "Spc: Assessor",
        r"(?:[\s[:punct:]])[Aa]ssessors?(?:[\s[:punct:]])"
    ),
    actor!(
        "Spc: Engineer",
        r"(?:[\s[:punct:]])[Ee]ngineer(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Manufacturer",
        r"(?:[\s[:punct:]])[Mm]anufacturer(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: C: Principal Designer",
        r"(?:[\s[:punct:]])[Pp]rincipal [Dd]esigner(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: C: Designer",
        r"(?:[\s[:punct:]])(?:[Dd]esigner|designs for another)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: C: Principal Contractor",
        r"(?:[\s[:punct:]])[Pp]rincipal [Cc]ontractor(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: C: Contractor",
        r"(?:[\s[:punct:]])(?:[Cc]ontractors?|[Dd]iving contractor|[Cc]ompressed air contractor)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Supplier",
        r"(?:[\s[:punct:]])(?:[Ss]upplier|[Pp]erson who supplies)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Importer",
        r"(?:[\s[:punct:]])(?:[Ii]mporter|person who.*?imports*?)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Distributor",
        r"(?:[\s[:punct:]])[Dd]istributors?(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Registrant",
        r"(?:[\s[:punct:]])[Rr]egistrants?(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Applicant",
        r"(?:[\s[:punct:]])[Aa]pplicants?(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Authorised Representative",
        r"(?:[\s[:punct:]])[Aa]uthoris?ed [Rr]epresentatives?(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Notified Body",
        r"(?:[\s[:punct:]])[Nn]otified [Bb]od(?:y|ies)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: Client",
        r"(?:[\s[:punct:]])[Cc]lients?(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: T&L: Carrier",
        r"(?:[\s[:punct:]])(?:[Tt]ransporter|[Cc]arriers?)(?:[\s[:punct:]])"
    ),
    actor!(
        "SC: T&L: Driver",
        r"(?:[\s[:punct:]])[Dd]river(?:[\s[:punct:]])"
    ),
    actor!(
        "Svc: Installer",
        r"(?:[\s[:punct:]])[Ii]nstaller(?:[\s[:punct:]])"
    ),
    actor!(
        "Org: Landlord",
        r"(?:[\s[:punct:]])[Ll]andlord(?:[\s[:punct:]])"
    ),
    actor!(
        "Public",
        r"(?:[\s[:punct:]])(?:[Pp]ublic|[Ee]veryone|[Cc]itizens?)(?:[\s[:punct:]])"
    ),
];

// ── Specialist governed actor definitions ────────────────────────────
//
// Domain-specific actors that only run when the law's family matches.
// Mirrors the specialist dictionary pattern in `fitness.rs`.

/// Offshore petroleum licensing actors.
/// Applied when `family.starts_with("OH&S: Offshore")`.
const OFFSHORE_GOVERNED_DEFS: &[(&str, &str)] = &[actor!(
    "Offshore: Licensee",
    r"(?:[\s[:punct:]])[Ll]icen[cs]ees?(?:[\s[:punct:]])"
)];

/// Public safety actors (online safety, firearms, dangerous dogs).
/// Applied when `family == "PUBLIC"`.
const PUBLIC_GOVERNED_DEFS: &[(&str, &str)] = &[
    actor!(
        "Public: Provider",
        r"(?:[\s[:punct:]])[Pp]roviders?(?:[\s[:punct:]])"
    ),
    actor!(
        "Public: Keeper",
        r"(?:[\s[:punct:]])[Kk]eepers?(?:[\s[:punct:]])"
    ),
    actor!(
        "Public: Dealer",
        r"(?:[\s[:punct:]])(?:[Dd]ealers?|registered (?:firearms )?dealer)(?:[\s[:punct:]])"
    ),
];

// ── Compiled pattern caches ──────────────────────────────────────────

static GOVERNMENT_COMPILED: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    GOVERNMENT_DEFS
        .iter()
        .map(|(label, pat)| (*label, Regex::new(pat).unwrap()))
        .collect()
});

static GOVERNED_COMPILED: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    GOVERNED_DEFS
        .iter()
        .map(|(label, pat)| (*label, Regex::new(pat).unwrap()))
        .collect()
});

static OFFSHORE_GOVERNED_COMPILED: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    OFFSHORE_GOVERNED_DEFS
        .iter()
        .map(|(label, pat)| (*label, Regex::new(pat).unwrap()))
        .collect()
});

static PUBLIC_GOVERNED_COMPILED: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    PUBLIC_GOVERNED_DEFS
        .iter()
        .map(|(label, pat)| (*label, Regex::new(pat).unwrap()))
        .collect()
});

/// Return specialist governed actor patterns for a given law family.
///
/// Returns an empty slice for unknown families — only core patterns run.
fn specialist_governed_for(family: &str) -> &'static [(&'static str, Regex)] {
    if family.starts_with("OH&S: Offshore") {
        &OFFSHORE_GOVERNED_COMPILED
    } else if family == "PUBLIC" {
        &PUBLIC_GOVERNED_COMPILED
    } else {
        &[]
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// All valid actor labels from every pattern library (core + specialist).
///
/// Used by Tier 3 LLM to validate that returned labels match the dictionary.
pub fn all_actor_labels() -> std::collections::HashSet<&'static str> {
    let mut labels = std::collections::HashSet::new();
    for (label, _) in GOVERNMENT_DEFS {
        labels.insert(*label);
    }
    for (label, _) in GOVERNED_DEFS {
        labels.insert(*label);
    }
    for (label, _) in OFFSHORE_GOVERNED_DEFS {
        labels.insert(*label);
    }
    for (label, _) in PUBLIC_GOVERNED_DEFS {
        labels.insert(*label);
    }
    labels
}

/// Extract all actors (governed + government) from text.
///
/// Applies the blacklist first, then runs each pattern library.
/// Matched text is progressively removed to avoid duplicate matches.
pub fn extract_actors(text: &str) -> ExtractedActors {
    let cleaned = apply_blacklist(text);
    ExtractedActors {
        governed: run_patterns(&cleaned, &GOVERNED_COMPILED),
        government: run_patterns(&cleaned, &GOVERNMENT_COMPILED),
    }
}

/// Extract all actors with family-gated specialist patterns.
///
/// Runs core patterns (same as `extract_actors`) plus any specialist
/// governed actor patterns that match the law family prefix.
pub fn extract_actors_for_family(text: &str, family: Option<&str>) -> ExtractedActors {
    let cleaned = apply_blacklist(text);
    let mut governed = run_patterns(&cleaned, &GOVERNED_COMPILED);

    if let Some(fam) = family {
        let specialists = specialist_governed_for(fam);
        if !specialists.is_empty() {
            let mut extra = run_patterns(&cleaned, specialists);
            governed.append(&mut extra);
            governed.sort_by(|a, b| a.label.cmp(&b.label));
            governed.dedup_by(|a, b| a.label == b.label);
        }
    }

    ExtractedActors {
        governed,
        government: run_patterns(&cleaned, &GOVERNMENT_COMPILED),
    }
}

/// Extract only governed actors from text.
pub fn extract_governed(text: &str) -> Vec<ActorMatch> {
    let cleaned = apply_blacklist(text);
    run_patterns(&cleaned, &GOVERNED_COMPILED)
}

/// Extract only government actors from text.
pub fn extract_government(text: &str) -> Vec<ActorMatch> {
    let cleaned = apply_blacklist(text);
    run_patterns(&cleaned, &GOVERNMENT_COMPILED)
}

// ── Internals ────────────────────────────────────────────────────────

fn run_patterns(text: &str, patterns: &[(&str, Regex)]) -> Vec<ActorMatch> {
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
}
