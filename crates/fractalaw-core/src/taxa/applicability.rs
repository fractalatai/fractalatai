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
