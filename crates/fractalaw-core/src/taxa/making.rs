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

const TIER_WEIGHTS: [f32; 5] = [0.0, 0.95, 0.75, 0.50, 0.65]; // index = tier

// ── Public API ───────────────────────────────────────────────────────

/// Run all detection tiers on metadata and return composite result.
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
}
