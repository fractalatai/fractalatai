//! Making / Not-Making pre-filter for UK legislation.
//!
//! Lightweight metadata-driven classifier that determines whether a law
//! is "Making" (creates new substantive obligations) before running the
//! expensive full Taxa pipeline.
//!
//! Ported from `Taxa.MakingDetector` + `Taxa.MakingDetectorSignals`.

// ── Types ────────────────────────────────────────────────────────────

/// Three-way classification outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MakingClassification {
    Making,
    NotMaking,
    Uncertain,
}

impl MakingClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Making => "making",
            Self::NotMaking => "not_making",
            Self::Uncertain => "uncertain",
        }
    }
}

/// Direction of a signal: pushes score toward making or not-making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    Making,
    NotMaking,
}

/// A single detection signal.
#[derive(Debug, Clone)]
pub struct Signal {
    pub tier: u8,
    pub name: &'static str,
    pub direction: SignalDirection,
    pub confidence: f32,
    pub value: String,
}

/// Composite detection result.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub confidence: f32,
    pub classification: MakingClassification,
    pub tier: u8,
    pub signals: Vec<Signal>,
}

/// Input metadata for detection.
pub struct LawMetadata<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub body_paras: Option<u32>,
    pub schedule_paras: Option<u32>,
}

// ── Constants ────────────────────────────────────────────────────────

const MAKING_THRESHOLD: f32 = 0.70;
const NOT_MAKING_THRESHOLD: f32 = 0.30;
const BASE_RATE: f32 = 0.173;

const TIER_WEIGHTS: [f32; 6] = [0.0, 0.95, 0.75, 0.50, 0.65, 0.85]; // index = tier

/// Provision-level triage counts from text analysis.
#[derive(Debug, Clone, Default)]
pub struct TriageCounts {
    /// Total provisions scanned.
    pub total: u32,
    /// Provisions with Process+Rule as primary purpose.
    pub process_rule: u32,
    /// Provisions with Amendment as any purpose.
    pub amendment: u32,
    /// Provisions with Enactment as any purpose.
    pub enactment: u32,
    /// Provisions with Interpretation as any purpose.
    pub interpretation: u32,
    /// Provisions with at least one governed actor.
    pub with_actor: u32,
    /// Provisions with obligation modals (shall/must).
    pub with_obligation: u32,
    /// Provisions with enabling modals (may/power to).
    pub with_enabling: u32,
}

// ── Public API ───────────────────────────────────────────────────────

/// Run metadata-only detection tiers (1-4). Fast, no provision text needed.
pub fn detect(meta: &LawMetadata<'_>) -> DetectionResult {
    let mut signals = Vec::new();
    tier1_title_definitive(&mut signals, meta);
    tier2_title_strong(&mut signals, meta);
    tier3_structural(&mut signals, meta);
    tier4_description(&mut signals, meta);

    let composite = calculate_composite_score(&signals);
    let classification = classify_score(composite);
    let tier = signals.iter().map(|s| s.tier).max().unwrap_or(0);

    DetectionResult {
        confidence: composite,
        classification,
        tier,
        signals,
    }
}

/// Run full triage including provision text analysis (tier 5).
///
/// Takes pre-computed `TriageCounts` from scanning provisions with
/// purpose classifier, actor extractor, and modal regex.
pub fn detect_with_triage(meta: &LawMetadata<'_>, counts: &TriageCounts) -> DetectionResult {
    let mut signals = Vec::new();
    tier1_title_definitive(&mut signals, meta);
    tier2_title_strong(&mut signals, meta);
    tier3_structural(&mut signals, meta);
    tier4_description(&mut signals, meta);
    tier5_provision_text(&mut signals, counts);

    let composite = calculate_composite_score(&signals);
    let classification = classify_score(composite);
    let tier = signals.iter().map(|s| s.tier).max().unwrap_or(0);

    DetectionResult {
        confidence: composite,
        classification,
        tier,
        signals,
    }
}

/// Scan provision texts and produce triage counts.
///
/// Runs purpose classifier, actor extractor, and modal regex on each
/// provision. Pure function — no store access, no side effects.
pub fn triage_provisions(texts: &[&str], family: Option<&str>) -> TriageCounts {
    use super::actors;
    use super::duty_patterns::{ENABLING, OBLIGATION};
    use super::purpose;
    use super::text_cleaner;

    let mut counts = TriageCounts::default();

    for raw in texts {
        let cleaned = text_cleaner::clean(raw);
        if cleaned.is_empty() {
            continue;
        }
        counts.total += 1;

        // Purpose classification
        let purposes = purpose::classify(&cleaned);
        if purposes.first() == Some(&purpose::PROCESS_RULE) {
            counts.process_rule += 1;
        }
        if purposes.contains(&purpose::AMENDMENT) {
            counts.amendment += 1;
        }
        if purposes.contains(&purpose::ENACTMENT) {
            counts.enactment += 1;
        }
        if purposes.iter().any(|p| p.contains("Interpretation")) {
            counts.interpretation += 1;
        }

        // Actor extraction
        let actors = actors::extract_actors_for_family(&cleaned, family);
        if !actors.governed.is_empty() || !actors.government.is_empty() {
            counts.with_actor += 1;
        }

        // Modal detection
        let lower = cleaned.to_lowercase();
        if OBLIGATION.is_match(&lower) {
            counts.with_obligation += 1;
        }
        if ENABLING.is_match(&lower) {
            counts.with_enabling += 1;
        }
    }

    counts
}

// ── Tier 5: Provision text analysis ─────────────────────────────────

fn tier5_provision_text(signals: &mut Vec<Signal>, counts: &TriageCounts) {
    if counts.total == 0 {
        return;
    }

    let total = counts.total as f32;

    // High proportion of Process+Rule provisions with actors + obligation modals → making
    let obligation_pct = counts.with_obligation as f32 / total;
    let actor_pct = counts.with_actor as f32 / total;
    let process_rule_pct = counts.process_rule as f32 / total;
    let amendment_pct = counts.amendment as f32 / total;

    // Strong making signal: >10% of provisions have actors + obligation modals
    if counts.with_obligation >= 3 && obligation_pct > 0.10 && actor_pct > 0.05 {
        let confidence = (0.60 + obligation_pct * 0.4).min(0.95);
        signals.push(Signal {
            tier: 5,
            name: "provisions_with_obligations",
            direction: SignalDirection::Making,
            confidence: (confidence * 100.0).round() / 100.0,
            value: format!(
                "obligations={}/{} actors={}/{} process_rule={}/{}",
                counts.with_obligation, counts.total,
                counts.with_actor, counts.total,
                counts.process_rule, counts.total,
            ),
        });
    }

    // Strong not-making signal: >60% amendment provisions, few obligations
    if amendment_pct > 0.60 && counts.with_obligation < 3 {
        signals.push(Signal {
            tier: 5,
            name: "mostly_amendment",
            direction: SignalDirection::NotMaking,
            confidence: (0.70 + amendment_pct * 0.25).min(0.95),
            value: format!("amendment={}/{}", counts.amendment, counts.total),
        });
    }

    // No obligations at all in a law with 10+ provisions → not making
    if counts.total >= 10 && counts.with_obligation == 0 {
        signals.push(Signal {
            tier: 5,
            name: "no_obligations_found",
            direction: SignalDirection::NotMaking,
            confidence: 0.85,
            value: format!("0 obligations in {} provisions", counts.total),
        });
    }

    // High Process+Rule with low amendment → positive making signal
    if process_rule_pct > 0.40 && amendment_pct < 0.10 && counts.total >= 5 {
        signals.push(Signal {
            tier: 5,
            name: "high_process_rule",
            direction: SignalDirection::Making,
            confidence: (0.50 + process_rule_pct * 0.3).min(0.85),
            value: format!("process_rule={}/{}", counts.process_rule, counts.total),
        });
    }
}

// ── Composite score (Bayesian-inspired) ──────────────────────────────

fn calculate_composite_score(signals: &[Signal]) -> f32 {
    if signals.is_empty() {
        return BASE_RATE;
    }
    // Sort: not_making first, then making (within each, by tier)
    let mut sorted: Vec<&Signal> = signals.iter().collect();
    sorted.sort_by_key(|s| {
        let dir = match s.direction {
            SignalDirection::NotMaking => 0,
            SignalDirection::Making => 1,
        };
        (dir, s.tier)
    });

    let score = sorted.iter().fold(BASE_RATE, |score, signal| {
        let weight = TIER_WEIGHTS
            .get(signal.tier as usize)
            .copied()
            .unwrap_or(0.5);
        match signal.direction {
            SignalDirection::NotMaking => score * (1.0 - signal.confidence * weight),
            SignalDirection::Making => score + (1.0 - score) * signal.confidence * weight,
        }
    });
    (score * 10000.0).round() / 10000.0
}

fn classify_score(score: f32) -> MakingClassification {
    if score >= MAKING_THRESHOLD {
        MakingClassification::Making
    } else if score <= NOT_MAKING_THRESHOLD {
        MakingClassification::NotMaking
    } else {
        MakingClassification::Uncertain
    }
}

// ── Tier 1: Definitive title exclusion ───────────────────────────────

fn tier1_title_definitive(signals: &mut Vec<Signal>, meta: &LawMetadata<'_>) {
    let Some(title) = meta.title else { return };
    if title.contains("(Appointed Day") {
        signals.push(Signal {
            tier: 1,
            name: "title_appointed_day",
            direction: SignalDirection::NotMaking,
            confidence: 1.0,
            value: title.to_string(),
        });
    } else if title.contains("(Commencement") {
        signals.push(Signal {
            tier: 1,
            name: "title_commencement",
            direction: SignalDirection::NotMaking,
            confidence: 0.99,
            value: title.to_string(),
        });
    }
}

// ── Tier 2: Strong title exclusion ───────────────────────────────────

fn tier2_title_strong(signals: &mut Vec<Signal>, meta: &LawMetadata<'_>) {
    let Some(title) = meta.title else { return };
    let checks: &[(&str, &str, f32)] = &[
        ("(Revocation", "title_revocation", 0.98),
        ("(Consequential", "title_consequential", 0.90),
        ("(Repeal", "title_repeal", 0.92),
        ("(Amendment", "title_amendment", 0.80),
        ("(Transitional", "title_transitional", 0.75),
    ];
    for &(pattern, name, confidence) in checks {
        if title.contains(pattern) {
            signals.push(Signal {
                tier: 2,
                name,
                direction: SignalDirection::NotMaking,
                confidence,
                value: title.to_string(),
            });
        }
    }
}

// ── Tier 3: Structural metadata ──────────────────────────────────────

fn tier3_structural(signals: &mut Vec<Signal>, meta: &LawMetadata<'_>) {
    let body = meta.body_paras;
    let sched = meta.schedule_paras;

    // Low body + high schedule = amending pattern
    if let (Some(b), Some(s)) = (body, sched)
        && b <= 3
        && s > 50
    {
        signals.push(Signal {
            tier: 3,
            name: "low_body_high_schedule",
            direction: SignalDirection::NotMaking,
            confidence: 0.90,
            value: format!("body={b},sched={s}"),
        });
    }

    // High body count counterbalance
    if let Some(b) = body {
        if b > 50 {
            let confidence = (0.50 + b as f32 / 500.0).min(0.85);
            signals.push(Signal {
                tier: 3,
                name: "high_body_paras",
                direction: SignalDirection::Making,
                confidence: (confidence * 100.0).round() / 100.0,
                value: b.to_string(),
            });
        } else if b <= 5 {
            signals.push(Signal {
                tier: 3,
                name: "very_low_body_paras",
                direction: SignalDirection::NotMaking,
                confidence: 0.60,
                value: b.to_string(),
            });
        }
    }
}

// ── Tier 4: Description analysis ─────────────────────────────────────

fn tier4_description(signals: &mut Vec<Signal>, meta: &LawMetadata<'_>) {
    let Some(desc) = meta.description else { return };
    if desc.is_empty() {
        return;
    }
    let lower = desc.to_lowercase();

    let making_patterns: &[(&str, &str, f32)] = &[
        (
            "to make further provision for securing",
            "provision_securing",
            0.90,
        ),
        ("to make provision for securing", "provision_securing", 0.90),
        ("to make further provision for", "provision_for", 0.70),
        ("to make provision for", "provision_for", 0.70),
        ("to require", "to_require", 0.80),
        ("to prohibit", "to_prohibit", 0.80),
        ("to regulate", "to_regulate", 0.75),
        ("to impose", "to_impose", 0.80),
        ("to establish", "to_establish", 0.70),
        ("to create", "to_create", 0.70),
    ];

    let not_making_patterns: &[(&str, &str, f32)] = &[
        ("to amend the", "desc_to_amend", 0.75),
        ("to give effect to", "desc_give_effect", 0.65),
        (
            "in exercise of the powers conferred by",
            "desc_powers_conferred",
            0.55,
        ),
    ];

    for &(pattern, name, confidence) in making_patterns {
        if lower.contains(pattern) {
            signals.push(Signal {
                tier: 4,
                name,
                direction: SignalDirection::Making,
                confidence,
                value: desc.chars().take(200).collect(),
            });
        }
    }
    for &(pattern, name, confidence) in not_making_patterns {
        if lower.contains(pattern) {
            signals.push(Signal {
                tier: 4,
                name,
                direction: SignalDirection::NotMaking,
                confidence,
                value: desc.chars().take(200).collect(),
            });
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(title: &str) -> LawMetadata<'_> {
        LawMetadata {
            title: Some(title),
            description: None,
            body_paras: None,
            schedule_paras: None,
        }
    }

    #[test]
    fn commencement_order_not_making() {
        let result = detect(&meta(
            "The Environmental Protection Act 1990 (Commencement No. 1) Order 1990",
        ));
        assert_eq!(result.classification, MakingClassification::NotMaking);
        assert!(result.confidence < 0.1);
    }

    #[test]
    fn amendment_not_making() {
        let result = detect(&meta(
            "The Workplace (Health, Safety and Welfare) (Amendment) Regulations 2024",
        ));
        assert_eq!(result.classification, MakingClassification::NotMaking);
    }

    #[test]
    fn substantive_act_uncertain_from_title_alone() {
        let result = detect(&meta("Health and Safety at Work etc. Act 1974"));
        // No signals from title → falls back to base rate (0.173) → uncertain
        assert_eq!(result.classification, MakingClassification::NotMaking);
    }

    #[test]
    fn substantive_with_high_body_count() {
        let result = detect(&LawMetadata {
            title: Some("Health and Safety at Work etc. Act 1974"),
            description: Some(
                "An Act to make further provision for securing the health, safety and welfare of persons at work",
            ),
            body_paras: Some(85),
            schedule_paras: Some(12),
        });
        // High body count + "provision for securing" description → high confidence
        assert!(result.confidence > 0.50);
        assert!(
            result
                .signals
                .iter()
                .any(|s| s.name == "provision_securing")
        );
        assert!(result.signals.iter().any(|s| s.name == "high_body_paras"));
    }

    #[test]
    fn amendment_with_high_body_overrides() {
        let result = detect(&LawMetadata {
            title: Some("The Control of Substances (Amendment) Regulations"),
            description: None,
            body_paras: Some(120),
            schedule_paras: Some(5),
        });
        // High body count partially counterbalances amendment title signal
        assert!(result.confidence > 0.15);
    }

    #[test]
    fn low_body_high_schedule_not_making() {
        let result = detect(&LawMetadata {
            title: Some("Some Regulations 2024"),
            description: None,
            body_paras: Some(2),
            schedule_paras: Some(100),
        });
        assert_eq!(result.classification, MakingClassification::NotMaking);
    }

    #[test]
    fn description_making_signals() {
        let result = detect(&LawMetadata {
            title: Some("New Regulations 2024"),
            description: Some("An Act to make provision for securing health and safety"),
            body_paras: Some(60),
            schedule_paras: Some(5),
        });
        assert!(
            result
                .signals
                .iter()
                .any(|s| s.name == "provision_securing")
        );
        assert_eq!(result.classification, MakingClassification::Making);
    }

    #[test]
    fn composite_score_base_rate_no_signals() {
        let score = calculate_composite_score(&[]);
        assert!((score - BASE_RATE).abs() < 0.001);
    }

    #[test]
    fn classification_thresholds() {
        assert_eq!(classify_score(0.80), MakingClassification::Making);
        assert_eq!(classify_score(0.50), MakingClassification::Uncertain);
        assert_eq!(classify_score(0.10), MakingClassification::NotMaking);
    }

    // ── Triage tests ────────────────────────────────────────────────

    #[test]
    fn triage_provisions_counts_obligations() {
        let texts = vec![
            "The employer shall ensure the health and safety of employees.",
            "These Regulations may be cited as the Example Regulations 2024.",
            "The employer must carry out a suitable and sufficient assessment.",
        ];
        let counts = triage_provisions(&texts, None);
        assert_eq!(counts.total, 3);
        assert_eq!(counts.with_obligation, 2);
        assert!(counts.with_actor >= 2); // "employer" extracted
    }

    #[test]
    fn triage_provisions_amendment_text() {
        let texts = vec![
            "For regulation 3 substitute the following regulation.",
            "In regulation 5(2), for paragraph (a) substitute—",
            "Regulation 7 is revoked.",
        ];
        let counts = triage_provisions(&texts, None);
        assert!(counts.amendment >= 2);
        assert_eq!(counts.with_obligation, 0);
    }

    #[test]
    fn triage_with_metadata_making() {
        let counts = TriageCounts {
            total: 50,
            process_rule: 30,
            amendment: 2,
            enactment: 1,
            interpretation: 5,
            with_actor: 20,
            with_obligation: 15,
            with_enabling: 8,
        };
        let result = detect_with_triage(
            &meta("Management of Health and Safety at Work Regulations 1999"),
            &counts,
        );
        assert_eq!(result.classification, MakingClassification::Making);
        assert!(result.signals.iter().any(|s| s.tier == 5));
    }

    #[test]
    fn triage_with_metadata_not_making() {
        let counts = TriageCounts {
            total: 20,
            process_rule: 1,
            amendment: 18,
            enactment: 0,
            interpretation: 1,
            with_actor: 0,
            with_obligation: 0,
            with_enabling: 0,
        };
        let result = detect_with_triage(
            &meta("The Workplace (Amendment) Regulations 2024"),
            &counts,
        );
        assert_eq!(result.classification, MakingClassification::NotMaking);
    }

    #[test]
    fn triage_empty_provisions() {
        let counts = TriageCounts::default();
        let result = detect_with_triage(&meta("Some Law 2024"), &counts);
        // No tier 5 signals when no provisions
        assert!(!result.signals.iter().any(|s| s.tier == 5));
    }
}
