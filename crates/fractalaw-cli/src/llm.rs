use anyhow::Context;

/// Parse a Gemini REST API response body into the inner JSON content.
///
/// Extracts `candidates[0].content.parts[0].text`, strips markdown code
/// fences, and parses the result as JSON.
pub(crate) fn parse_gemini_response(response_body: &str) -> Option<serde_json::Value> {
    let gemini_resp: serde_json::Value = serde_json::from_str(response_body).ok()?;
    let content_text = gemini_resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let json_text = if content_text.contains("```json") {
        content_text
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(content_text)
            .trim()
    } else if content_text.contains("```") {
        content_text
            .split("```")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(content_text)
            .trim()
    } else {
        content_text.trim()
    };
    serde_json::from_str(json_text).ok()
}

/// Actor dictionary entry, deserialized from YAML.
#[derive(serde::Deserialize)]
pub(crate) struct DictEntry {
    label: String,
    #[serde(rename = "type")]
    actor_type: Option<String>,
    category: Option<String>,
    #[serde(default)]
    triggers: Vec<String>,
}

impl DictEntry {
    /// Backward-compatible accessor for canonical label.
    fn canonical(&self) -> String {
        self.label.clone()
    }
}

/// Matches LLM natural-language actor names to canonical dictionary labels.
///
/// Loads `crates/fractalaw-core/data/actor-dictionary.yaml` — the single source of truth.
/// Two-pass matching: exact trigger → substring containment (longest first).
pub(crate) struct ActorMatcher {
    entries: Vec<DictEntry>,
    /// (trigger, canonical) sorted by trigger length descending for Pass 2.
    all_triggers: Vec<(String, String)>,
}

impl ActorMatcher {
    pub(crate) fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading actor dictionary from {path}"))?;
        let entries: Vec<DictEntry> =
            serde_yaml::from_str(&content).context("parsing actor dictionary YAML")?;

        let mut all_triggers: Vec<(String, String)> = Vec::new();
        for entry in &entries {
            for trigger in &entry.triggers {
                all_triggers.push((trigger.clone(), entry.canonical().clone()));
            }
        }
        all_triggers.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        Ok(Self {
            entries,
            all_triggers,
        })
    }

    /// Match an LLM actor name to a canonical label.
    ///
    /// Returns `Some((canonical_label, confidence))` or `None` for discoveries.
    pub(crate) fn match_name(&self, name: &str) -> Option<(String, f64)> {
        let n = name.trim().to_lowercase();
        if n.is_empty() {
            return None;
        }

        // Pass 1: exact trigger match (order-sensitive — specific before generic)
        for entry in &self.entries {
            for trigger in &entry.triggers {
                if n == *trigger {
                    return Some((entry.canonical().clone(), 1.0));
                }
            }
        }

        // Pass 2: substring containment (longest trigger first)
        for (trigger, canonical) in &self.all_triggers {
            if n.contains(trigger.as_str()) || trigger.contains(n.as_str()) {
                return Some((canonical.clone(), 0.85));
            }
        }

        // No match — discovery
        None
    }

    /// Check if a canonical label is a government/EU actor.
    pub(crate) fn is_government(&self, canonical_label: &str) -> bool {
        for entry in &self.entries {
            if entry.canonical() == canonical_label {
                // Prefer the explicit type field from unified YAML
                if let Some(ref t) = entry.actor_type {
                    return t == "government";
                }
                // Fallback to category for backward compatibility
                let cat = entry.category.as_deref().unwrap_or("other");
                return cat == "Gvt" || cat == "EU";
            }
        }
        false
    }
}

/// Parsed actor from Tier 3 LLM response, with label validation.
pub(crate) struct ParsedTier3Actor {
    pub(crate) label: String,
    pub(crate) position: String,
    pub(crate) relates_to: Option<String>,
    pub(crate) label_source: String,
    pub(crate) reason: Option<String>,
}

/// Parse the actors array from an LLM response, resolving labels via the actor matcher.
///
/// Returns `None` if the response doesn't contain a valid actors array.
/// Labels are resolved through the dictionary matcher — unmatched labels
/// get `label_source = "invented"`.
pub(crate) fn parse_tier3_actors(
    result: &serde_json::Value,
    matcher: &ActorMatcher,
) -> Option<Vec<ParsedTier3Actor>> {
    let actors_arr = result.get("actors")?.as_array()?;
    let mut actors = Vec::new();
    for actor_val in actors_arr {
        let raw_label = actor_val
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Resolve through actor dictionary matcher
        let (label, label_source) = if let Some((canonical, _conf)) = matcher.match_name(&raw_label)
        {
            (canonical, "canonical".to_string())
        } else {
            (raw_label, "invented".to_string())
        };

        let position = actor_val
            .get("position")
            .and_then(|v| v.as_str())
            .unwrap_or("mentioned")
            .to_lowercase();
        let position_str = match position.as_str() {
            "active" => "active",
            "counterparty" => "counterparty",
            "beneficiary" => "beneficiary",
            _ => "mentioned",
        };

        // Resolve relates_to through the matcher too
        let relates_to = actor_val
            .get("relates_to")
            .and_then(|v| v.as_str())
            .map(|s| {
                matcher
                    .match_name(s)
                    .map(|(canonical, _)| canonical)
                    .unwrap_or_else(|| s.to_string())
            });

        let reason = actor_val
            .get("reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        actors.push(ParsedTier3Actor {
            label,
            position: position_str.into(),
            relates_to,
            label_source,
            reason,
        });
    }
    Some(actors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_matcher() -> ActorMatcher {
        // CARGO_MANIFEST_DIR points to the crate dir; dictionary is in fractalaw-core/data
        let manifest = env!("CARGO_MANIFEST_DIR");
        let dict_path = format!("{manifest}/../fractalaw-core/data/actor-dictionary.yaml");
        ActorMatcher::load(&dict_path).expect("actor dictionary must exist for tests")
    }

    #[test]
    fn parse_gemini_response_plain_json() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"{\"actors\":[{\"label\":\"Org: Employer\",\"role\":\"HOLDER\"}],\"primary_holder\":\"Org: Employer\"}"}],"role":"model"},"finishReason":"STOP"}]}"#;
        let parsed = parse_gemini_response(body).unwrap();
        assert_eq!(parsed["primary_holder"], "Org: Employer");
    }

    #[test]
    fn parse_gemini_response_code_fence() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"```json\n{\"actors\":[{\"label\":\"Org: Employer\",\"role\":\"HOLDER\"}]}\n```"}],"role":"model"},"finishReason":"STOP"}]}"#;
        let parsed = parse_gemini_response(body).unwrap();
        let actors = parsed["actors"].as_array().unwrap();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0]["label"], "Org: Employer");
    }

    #[test]
    fn parse_gemini_response_truncated() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"{\"actors\":[{\"label\":\"Org: Emp"}],"role":"model"},"finishReason":"MAX_TOKENS"}]}"#;
        assert!(parse_gemini_response(body).is_none());
    }

    #[test]
    fn parse_gemini_response_invalid_json() {
        let body = "not json at all";
        assert!(parse_gemini_response(body).is_none());
    }

    #[test]
    fn parse_tier3_actors_canonical_labels() {
        // LLM outputs natural language — matcher resolves to canonical
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[
                {"label":"employer","position":"ACTIVE","reason":"employer bears the duty"},
                {"label":"employee","position":"COUNTERPARTY","reason":"employee holds the claim"}
            ]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0].label, "Org: Employer");
        assert_eq!(actors[0].position, "active");
        assert_eq!(actors[0].label_source, "canonical");
        assert_eq!(actors[0].reason, Some("employer bears the duty".into()));
        assert_eq!(actors[1].label, "Ind: Employee");
        assert_eq!(actors[1].position, "counterparty");
        assert_eq!(actors[1].label_source, "canonical");
        assert_eq!(actors[1].reason, Some("employee holds the claim".into()));
    }

    #[test]
    fn parse_tier3_actors_natural_language_resolved() {
        // LLM says "responsible person" in natural language → matcher resolves to canonical
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[{"label":"responsible person","position":"ACTIVE"},{"label":"inspector","position":"COUNTERPARTY"}]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].label, "Ind: Responsible Person");
        assert_eq!(actors[0].position, "active");
        assert_eq!(actors[0].label_source, "canonical");
        assert_eq!(actors[1].label, "Spc: Inspector");
        assert_eq!(actors[1].label_source, "canonical");
    }

    #[test]
    fn parse_tier3_actors_invented_label() {
        // "producers of electricity from high-efficiency cogeneration" is not in the dictionary
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[{"label":"producers of electricity from high-efficiency cogeneration","position":"ACTIVE"},{"label":"employer","position":"COUNTERPARTY"}]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(
            actors[0].label,
            "producers of electricity from high-efficiency cogeneration"
        );
        assert_eq!(actors[0].label_source, "invented");
        assert_eq!(actors[1].label, "Org: Employer");
        assert_eq!(actors[1].label_source, "canonical");
    }

    #[test]
    fn parse_tier3_actors_relates_to() {
        // LLM uses natural language for relates_to — matcher resolves it
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[
                {"label":"employer","position":"ACTIVE","relates_to":"employee","reason":"employer must train employees"},
                {"label":"employee","position":"COUNTERPARTY","reason":"receives training"}
            ]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].relates_to, Some("Ind: Employee".into()));
        assert_eq!(actors[1].relates_to, None);
    }

    #[test]
    fn parse_tier3_actors_all_positions() {
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[
                {"label":"employer","position":"ACTIVE"},
                {"label":"employee","position":"COUNTERPARTY"},
                {"label":"any person","position":"BENEFICIARY"},
                {"label":"inspector","position":"MENTIONED"}
            ]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].label, "Org: Employer");
        assert_eq!(actors[0].position, "active");
        assert_eq!(actors[1].label, "Ind: Employee");
        assert_eq!(actors[1].position, "counterparty");
        assert_eq!(actors[2].label, "Ind: Person");
        assert_eq!(actors[2].position, "beneficiary");
        assert_eq!(actors[3].label, "Spc: Inspector");
        assert_eq!(actors[3].position, "mentioned");
    }

    #[test]
    fn parse_tier3_actors_missing_position_defaults_mentioned() {
        let result: serde_json::Value =
            serde_json::from_str(r#"{"actors":[{"label":"employer"}]}"#).unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].label, "Org: Employer");
        assert_eq!(actors[0].position, "mentioned");
        assert_eq!(actors[0].reason, None);
        assert_eq!(actors[0].relates_to, None);
    }

    #[test]
    fn parse_tier3_actors_no_actors_key() {
        let result: serde_json::Value =
            serde_json::from_str(r#"{"primary_holder":"Org: Employer"}"#).unwrap();
        let matcher = test_matcher();
        assert!(parse_tier3_actors(&result, &matcher).is_none());
    }

    #[test]
    fn actor_matcher_exact_triggers() {
        let m = test_matcher();
        // Exact trigger matches
        assert_eq!(
            m.match_name("employer"),
            Some(("Org: Employer".into(), 1.0))
        );
        assert_eq!(
            m.match_name("HSE"),
            Some(("Gvt: Agency: Health and Safety Executive".into(), 1.0))
        );
        assert_eq!(
            m.match_name("inspector"),
            Some(("Spc: Inspector".into(), 1.0))
        );
        assert_eq!(
            m.match_name("local authority"),
            Some(("Gvt: Authority: Local".into(), 1.0))
        );
    }

    #[test]
    fn actor_matcher_substring_containment() {
        let m = test_matcher();
        // Substring matching — longer trigger wins
        let (label, conf) = m.match_name("the enforcing authority").unwrap();
        assert_eq!(label, "Gvt: Authority: Enforcement");
        assert!((conf - 0.85).abs() < 0.01);
    }

    #[test]
    fn actor_matcher_discovery() {
        let m = test_matcher();
        // Genuinely novel actors not in dictionary
        assert!(
            m.match_name("producers of electricity from high-efficiency cogeneration")
                .is_none()
        );
        assert!(m.match_name("committee on toxicity").is_none());
    }

    #[test]
    fn actor_matcher_expanded_dictionary() {
        let m = test_matcher();
        // Insolvency roles added from corpus discoveries
        assert_eq!(
            m.match_name("liquidator"),
            Some(("Spc: Liquidator".into(), 1.0))
        );
        assert_eq!(
            m.match_name("water undertaker"),
            Some(("Svc: Water Undertaker".into(), 1.0))
        );
        assert_eq!(
            m.match_name("young people"),
            Some(("Ind: Young Person".into(), 1.0))
        );
        assert_eq!(
            m.match_name("special negotiating body"),
            Some(("EU: Special Negotiating Body".into(), 1.0))
        );
        assert_eq!(
            m.match_name("competent national authorities"),
            Some(("Gvt: Authority".into(), 1.0))
        );
    }

    #[test]
    fn actor_matcher_specificity() {
        let m = test_matcher();
        // "secretary of state for defence" should match the specific entry, not generic
        let (label, _) = m.match_name("secretary of state for defence").unwrap();
        assert_eq!(label, "Gvt: Minister: Secretary of State for Defence");
        // Plain "secretary of state" should match generic
        let (label, _) = m.match_name("secretary of state").unwrap();
        assert_eq!(label, "Gvt: Minister");
    }

    #[test]
    fn actor_matcher_is_government() {
        let m = test_matcher();
        assert!(m.is_government("Gvt: Authority: Enforcement"));
        assert!(m.is_government("EU: Commission"));
        assert!(!m.is_government("Org: Employer"));
        assert!(!m.is_government("Ind: Employee"));
    }
}
