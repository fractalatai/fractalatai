#!/usr/bin/env /usr/bin/python3
"""Migrate actor dictionary to unified format.

Merges:
  1. docs/actor-dictionary.yaml (canonical, category, triggers)
  2. crates/fractalaw-core/src/taxa/actors.rs (regex patterns)
  3. crates/fractalaw-core/src/taxa/duty_patterns.rs (GOVERNMENT_ACTORS keywords)

Produces a unified YAML with: label, type, category, regex_patterns, triggers, drrp_keywords.
"""

import re
import sys

import yaml


def extract_rust_actors(filepath):
    """Extract actor definitions from actors.rs."""
    with open(filepath) as f:
        content = f.read()

    actors = {"government": [], "governed": [], "offshore": [], "public": []}

    # Extract each DEFS block
    patterns = {
        "government": r'const GOVERNMENT_DEFS:.*?=\s*&\[(.*?)\];',
        "governed": r'const GOVERNED_DEFS:.*?=\s*&\[(.*?)\];',
        "offshore": r'const OFFSHORE_GOVERNED_DEFS:.*?=\s*&\[(.*?)\];',
        "public": r'const PUBLIC_GOVERNED_DEFS:.*?=\s*&\[(.*?)\];',
    }

    for group, pat in patterns.items():
        m = re.search(pat, content, re.DOTALL)
        if not m:
            continue
        block = m.group(1)
        # Extract actor!("label", r"pattern") entries
        for am in re.finditer(r'actor!\(\s*"([^"]+)"\s*,\s*r#?"([^"]+)"', block):
            label = am.group(1)
            pattern = am.group(2)
            actors[group].append({"label": label, "pattern": pattern})

    return actors


def extract_drrp_keywords(filepath):
    """Extract GOVERNMENT_ACTORS keywords from duty_patterns.rs."""
    with open(filepath) as f:
        content = f.read()

    m = re.search(r'const GOVERNMENT_ACTORS:.*?=\s*&\[(.*?)\];', content, re.DOTALL)
    if not m:
        return []

    keywords = re.findall(r'"([^"]+)"', m.group(1))
    return keywords


def strip_boundary(pattern):
    """Remove the boundary wrapper from a regex pattern."""
    # Pattern: (?:[\s[:punct:]])CORE(?:[\s[:punct:]])
    p = pattern
    p = re.sub(r'^\(\?:\[\\s\[:punct:\]\]\)', '', p)
    p = re.sub(r'\(\?:\[\\s\[:punct:\]\]\)$', '', p)
    return p.strip()


def classify_type(label, group):
    """Determine governed/government type."""
    if group == "government":
        return "government"
    return "governed"


def main():
    # Load existing YAML
    with open("docs/actor-dictionary.yaml") as f:
        yaml_entries = yaml.safe_load(f)

    # Build lookup by canonical label
    yaml_by_label = {}
    for entry in yaml_entries:
        yaml_by_label[entry["canonical"]] = entry

    # Extract Rust patterns
    rust_actors = extract_rust_actors("crates/fractalaw-core/src/taxa/actors.rs")

    # Extract DRRP keywords
    drrp_keywords = extract_drrp_keywords("crates/fractalaw-core/src/taxa/duty_patterns.rs")

    # Build unified dictionary
    unified = []
    seen_labels = set()

    # Process Rust actors first (they define regex patterns)
    for group in ["government", "governed", "offshore", "public"]:
        for actor in rust_actors[group]:
            label = actor["label"]
            pattern = strip_boundary(actor["pattern"])
            actor_type = classify_type(label, group)

            # Find matching YAML entry
            yaml_entry = yaml_by_label.get(label, {})
            triggers = yaml_entry.get("triggers", [])
            category = yaml_entry.get("category", "")

            # Derive category from label prefix if not in YAML
            if not category:
                if ":" in label:
                    category = label.split(":")[0].strip()
                else:
                    category = "other"

            # Determine drrp_keywords (only for government actors)
            actor_drrp_keywords = []
            if actor_type == "government":
                # Check which GOVERNMENT_ACTORS keywords this actor provides
                for kw in drrp_keywords:
                    if kw in pattern.lower() or kw in label.lower():
                        actor_drrp_keywords.append(kw)
                # Also add from triggers
                for t in triggers:
                    if t in drrp_keywords:
                        actor_drrp_keywords.append(t)
                actor_drrp_keywords = sorted(set(actor_drrp_keywords))

            # Determine families for specialist patterns
            families = None
            if group == "offshore":
                families = ["OH&S: Offshore"]
            elif group == "public":
                families = ["PUBLIC"]

            entry = {
                "label": label,
                "type": actor_type,
                "category": category,
                "regex_patterns": [pattern],
                "triggers": triggers if triggers else [],
            }
            if actor_drrp_keywords:
                entry["drrp_keywords"] = actor_drrp_keywords
            if families:
                entry["families"] = families

            unified.append(entry)
            seen_labels.add(label)

    # Add YAML-only entries (no Rust regex pattern)
    for entry in yaml_entries:
        label = entry["canonical"]
        if label not in seen_labels:
            unified.append({
                "label": label,
                "type": "government" if entry.get("category") in ("Gvt", "EU") else "governed",
                "category": entry.get("category", "other"),
                "regex_patterns": [],  # No regex pattern — LLM-only
                "triggers": entry.get("triggers", []),
            })
            seen_labels.add(label)

    # Output
    print(f"# Unified Actor Dictionary — Single Source of Truth")
    print(f"#")
    print(f"# Generated by scripts/migrate_actor_dictionary.py")
    print(f"# {len(unified)} actors ({sum(1 for a in unified if a['type'] == 'government')} government, {sum(1 for a in unified if a['type'] == 'governed')} governed)")
    print(f"#")
    print(f"# Fields:")
    print(f"#   label:          Canonical label (e.g., 'Org: Employer')")
    print(f"#   type:           'governed' or 'government' — determines DRRP mapping")
    print(f"#   category:       Gvt/EU/Org/Ind/SC/Spc/Svc/Public/Offshore/other")
    print(f"#   regex_patterns: Patterns for text extraction (boundary wrapper added by Rust)")
    print(f"#   triggers:       LLM trigger phrases for actor-match post-processing")
    print(f"#   drrp_keywords:  Downcased keywords for has_government_actor() check (government only)")
    print(f"#   families:       Family gates for specialist patterns (optional)")
    print(f"#")
    print(f"# Ordering: specific entries before generic. Pattern order matters for")
    print(f"# overlapping keywords (e.g., 'Secretary of State for Defence' before")
    print(f"# 'Secretary of State').")
    print()
    print(yaml.dump(unified, default_flow_style=False, allow_unicode=True, sort_keys=False, width=120))


if __name__ == "__main__":
    main()
