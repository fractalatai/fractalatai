//! Actor definitions and extraction for UK ESH legal text.
//!
//! Identifies WHO is mentioned in legislative text, split into two groups:
//! - **Government actors**: Crown, authorities, agencies, ministers, devolved admins
//! - **Governed actors**: Businesses, individuals, specialists, supply-chain actors
//!
//! Ported from `Taxa.ActorDefinitions`, `Taxa.ActorLib`, and `Taxa.DutyActor`.

use std::sync::LazyLock;

use regex::Regex;

// ── Types ────────────────────────────────────────────────────────────

/// Extraction result: governed + government actor labels found in text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedActors {
    pub governed: Vec<String>,
    pub government: Vec<String>,
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
    actor!(
        "HM Forces",
        r"(?:[\s[:punct:]])(?:(?:His|Her) Majesty[''\u{2019}]s forces|armed forces)(?:[\s[:punct:]])"
    ),
];

/// Governed actor definitions (businesses, individuals, supply chain, etc.)
const GOVERNED_DEFS: &[(&str, &str)] = &[
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

// ── Public API ───────────────────────────────────────────────────────

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

/// Extract only governed actors from text.
pub fn extract_governed(text: &str) -> Vec<String> {
    let cleaned = apply_blacklist(text);
    run_patterns(&cleaned, &GOVERNED_COMPILED)
}

/// Extract only government actors from text.
pub fn extract_government(text: &str) -> Vec<String> {
    let cleaned = apply_blacklist(text);
    run_patterns(&cleaned, &GOVERNMENT_COMPILED)
}

// ── Internals ────────────────────────────────────────────────────────

fn run_patterns(text: &str, patterns: &[(&str, Regex)]) -> Vec<String> {
    let mut remaining = text.to_string();
    let mut found = Vec::new();
    for (label, regex) in patterns {
        if regex.is_match(&remaining) {
            found.push(label.to_string());
            // Remove first match to prevent duplicate detection
            remaining = regex.replace(&remaining, "").to_string();
        }
    }
    found.sort();
    found.dedup();
    found
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_employer() {
        let actors = extract_actors(" The employer shall ensure safety. ");
        assert!(actors.governed.contains(&"Org: Employer".to_string()));
    }

    #[test]
    fn extract_secretary_of_state() {
        let actors = extract_actors(" The Secretary of State may make regulations. ");
        assert!(actors.government.contains(&"Gvt: Minister".to_string()));
    }

    #[test]
    fn extract_hse() {
        let actors = extract_actors(" The Health and Safety Executive shall. ");
        assert!(
            actors
                .government
                .iter()
                .any(|a| a.contains("Health and Safety Executive"))
        );
    }

    #[test]
    fn extract_multiple_actors() {
        let text = " The employer shall consult the inspector and the employee. ";
        let actors = extract_actors(text);
        assert!(actors.governed.contains(&"Org: Employer".to_string()));
        assert!(actors.governed.contains(&"Ind: Employee".to_string()));
    }

    #[test]
    fn blacklist_removes_false_positives() {
        // "public interest" should be blacklisted
        let actors = extract_actors(" This is in the public interest. ");
        assert!(!actors.governed.contains(&"Public".to_string()));
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
        assert!(actors.governed.iter().any(|a| a.contains("Contractor")));
    }

    #[test]
    fn extract_local_authority() {
        let actors = extract_actors(" The local authority may issue a notice. ");
        assert!(actors.government.iter().any(|a| a.contains("Local")));
    }

    #[test]
    fn agency_worker_not_government_agency() {
        // "agency worker" / "temporary work agency" are employment terms,
        // not government agencies — blacklist should prevent false positive
        let actors = extract_actors(
            " Where, in the case of an individual agency worker, the taking \
              of any other action the hirer is required to take. ",
        );
        assert!(
            !actors.government.iter().any(|a| a == "Gvt: Agency"),
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
            !actors.government.iter().any(|a| a == "Gvt: Agency"),
            "temporary work agency should not be classified as Gvt: Agency, got: {:?}",
            actors.government
        );
    }
}
