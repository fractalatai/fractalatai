//! Compiled applicability expression trees for fitness rule evaluation.
//!
//! Each law's fitness mentions are compiled into a boolean expression tree
//! that sertantai evaluates against customer profiles at query time.
//!
//! The tree is serialised as JSON and published via Zenoh as part of the
//! LRT payload. Sertantai deserialises and walks the tree recursively.

use serde::{Deserialize, Serialize};

/// A node in the applicability expression tree.
///
/// Evaluation: walk the tree recursively against a customer profile.
/// Each `Match` leaf checks whether the customer's attributes intersect
/// the required codes for that scope dimension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op")]
pub enum ApplicabilityNode {
    /// Leaf: does the customer match any/all of these codes in this dimension?
    Match {
        dimension: String,
        codes: Vec<String>,
        #[serde(default = "default_match_op")]
        match_op: MatchOp,
    },

    /// All children must match.
    And { children: Vec<ApplicabilityNode> },

    /// Any child must match.
    Or { children: Vec<ApplicabilityNode> },

    /// Child must NOT match.
    Not {
        child: Box<ApplicabilityNode>,
    },

    /// Match `then` only if `condition` matches first.
    Conditional {
        condition: Box<ApplicabilityNode>,
        then: Box<ApplicabilityNode>,
    },

    /// Temporal applicability: law applies between `from` and `to` dates.
    /// `None` means unbounded in that direction.
    TimeWindow {
        from: Option<String>,
        to: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        inner: Option<Box<ApplicabilityNode>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchOp {
    AnyOf,
    AllOf,
}

fn default_match_op() -> MatchOp {
    MatchOp::AnyOf
}

impl ApplicabilityNode {
    /// Create a Match node.
    pub fn match_any(dimension: &str, codes: Vec<String>) -> Self {
        Self::Match {
            dimension: dimension.to_string(),
            codes,
            match_op: MatchOp::AnyOf,
        }
    }

    /// Wrap in a Not node.
    pub fn negate(self) -> Self {
        Self::Not {
            child: Box::new(self),
        }
    }

    /// Combine multiple nodes with And. Flattens single-element vecs.
    pub fn and(children: Vec<Self>) -> Self {
        match children.len() {
            0 => Self::match_any("any", vec![]),
            1 => children.into_iter().next().unwrap(),
            _ => Self::And { children },
        }
    }

    /// Combine multiple nodes with Or. Flattens single-element vecs.
    pub fn or(children: Vec<Self>) -> Self {
        match children.len() {
            0 => Self::match_any("any", vec![]),
            1 => children.into_iter().next().unwrap(),
            _ => Self::Or { children },
        }
    }

    /// Create a TimeWindow node.
    pub fn time_window(from: Option<&str>, to: Option<&str>) -> Self {
        Self::TimeWindow {
            from: from.map(|s| s.to_string()),
            to: to.map(|s| s.to_string()),
            inner: None,
        }
    }

    /// Serialise to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialise to pretty JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialise from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Canonicalise the tree without changing its meaning:
    /// - Match codes sorted and deduplicated
    /// - nested And/Or of the same op flattened into the parent
    /// - identical siblings removed (lint L1)
    /// - AnyOf Match siblings on the same dimension under an Or merged
    ///   (`Or(Match(d,[a]), Match(d,[b]))` ≡ `Match(d,[a,b])`)
    /// - single-child And/Or collapsed, `Not(Not(x))` → `x`
    pub fn normalize(self) -> Self {
        match self {
            Self::Match {
                dimension,
                mut codes,
                match_op,
            } => {
                codes.sort();
                codes.dedup();
                Self::Match {
                    dimension,
                    codes,
                    match_op,
                }
            }
            Self::And { children } => Self::normalize_group(children, false),
            Self::Or { children } => Self::normalize_group(children, true),
            Self::Not { child } => match child.normalize() {
                Self::Not { child: inner } => *inner,
                other => other.negate(),
            },
            Self::Conditional { condition, then } => Self::Conditional {
                condition: Box::new(condition.normalize()),
                then: Box::new(then.normalize()),
            },
            Self::TimeWindow { from, to, inner } => Self::TimeWindow {
                from,
                to,
                inner: inner.map(|n| Box::new(n.normalize())),
            },
        }
    }

    fn normalize_group(children: Vec<Self>, is_or: bool) -> Self {
        let mut flat: Vec<Self> = Vec::new();
        for child in children.into_iter().map(Self::normalize) {
            match child {
                Self::Or { children } if is_or => flat.extend(children),
                Self::And { children } if !is_or => flat.extend(children),
                other => flat.push(other),
            }
        }

        let mut out: Vec<Self> = Vec::new();
        for child in flat {
            if is_or
                && let Self::Match {
                    dimension,
                    codes,
                    match_op: MatchOp::AnyOf,
                } = &child
            {
                let existing = out.iter_mut().find(|n| {
                    matches!(n, Self::Match { dimension: d, match_op: MatchOp::AnyOf, .. } if d == dimension)
                });
                if let Some(Self::Match { codes: existing_codes, .. }) = existing {
                    existing_codes.extend(codes.iter().cloned());
                    existing_codes.sort();
                    existing_codes.dedup();
                    continue;
                }
            }
            if !out.contains(&child) {
                out.push(child);
            }
        }

        if is_or {
            Self::or(out)
        } else {
            Self::and(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_serialises_correctly() {
        let node = ApplicabilityNode::match_any("personal", vec!["employer".into()]);
        let json = node.to_json().unwrap();
        assert!(json.contains("\"op\":\"Match\""));
        assert!(json.contains("\"dimension\":\"personal\""));
        assert!(json.contains("\"codes\":[\"employer\"]"));
    }

    #[test]
    fn and_with_not_serialises() {
        let tree = ApplicabilityNode::and(vec![
            ApplicabilityNode::match_any("personal", vec!["employer".into()]),
            ApplicabilityNode::match_any("material", vec!["ship".into()]).negate(),
        ]);
        let json = tree.to_json().unwrap();
        assert!(json.contains("\"op\":\"And\""));
        assert!(json.contains("\"op\":\"Not\""));
    }

    #[test]
    fn roundtrip_json() {
        let tree = ApplicabilityNode::and(vec![
            ApplicabilityNode::match_any("personal", vec!["employer".into(), "contractor".into()]),
            ApplicabilityNode::match_any("territorial", vec!["england".into()]),
            ApplicabilityNode::match_any("material", vec!["domestic_premises".into()]).negate(),
            ApplicabilityNode::time_window(Some("2025-10-01"), None),
        ]);
        let json = tree.to_json().unwrap();
        let restored = ApplicabilityNode::from_json(&json).unwrap();
        assert_eq!(tree, restored);
    }

    fn m(dim: &str, codes: &[&str]) -> ApplicabilityNode {
        ApplicabilityNode::match_any(dim, codes.iter().map(|c| c.to_string()).collect())
    }

    #[test]
    fn normalize_removes_duplicate_siblings() {
        let branch = ApplicabilityNode::and(vec![m("material", &["vehicle"]), m("territorial", &["premises"])]);
        let tree = ApplicabilityNode::or(vec![branch.clone(), branch.clone(), branch.clone()]);
        assert_eq!(tree.normalize(), branch);
    }

    #[test]
    fn normalize_merges_or_matches_on_same_dimension() {
        let tree = ApplicabilityNode::or(vec![
            m("personal", &["employer"]),
            m("material", &["waste"]),
            m("personal", &["employee", "employer"]),
        ]);
        assert_eq!(
            tree.normalize(),
            ApplicabilityNode::or(vec![m("personal", &["employee", "employer"]), m("material", &["waste"])])
        );
    }

    #[test]
    fn normalize_does_not_merge_and_matches() {
        // And(AnyOf a, AnyOf b) is not AnyOf(a ∪ b): leave both.
        let tree = ApplicabilityNode::and(vec![m("personal", &["employer"]), m("personal", &["operator"])]);
        assert_eq!(tree.clone().normalize(), tree);
    }

    #[test]
    fn normalize_flattens_nested_same_op_and_single_child() {
        let tree = ApplicabilityNode::And {
            children: vec![
                ApplicabilityNode::And { children: vec![m("personal", &["employer"])] },
                ApplicabilityNode::Or { children: vec![m("material", &["waste"])] },
            ],
        };
        assert_eq!(
            tree.normalize(),
            ApplicabilityNode::and(vec![m("personal", &["employer"]), m("material", &["waste"])])
        );
    }

    #[test]
    fn normalize_double_negation_and_code_order() {
        let tree = m("material", &["waste", "asbestos", "waste"]).negate().negate();
        assert_eq!(tree.normalize(), m("material", &["asbestos", "waste"]));
    }

    #[test]
    fn single_child_and_flattens() {
        let tree = ApplicabilityNode::and(vec![
            ApplicabilityNode::match_any("personal", vec!["employer".into()]),
        ]);
        match tree {
            ApplicabilityNode::Match { .. } => {} // flattened to Match, not And
            _ => panic!("single-child And should flatten"),
        }
    }
}
