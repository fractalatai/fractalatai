#!/usr/bin/env /usr/bin/python3
"""Actor name matcher — resolves LLM natural language to canonical dictionary labels.

Usage as module:
    from actor_match import ActorMatcher
    matcher = ActorMatcher("crates/fractalaw-core/data/actor-dictionary.yaml")
    label, conf = matcher.match("employer")  # → ("Org: Employer", 1.0)

Usage as CLI:
    /usr/bin/python3 .claude/skills/actor-match/scripts/actor_match.py "enforcing authority"
    /usr/bin/python3 .claude/skills/actor-match/scripts/actor_match.py --test
"""

import sys
from pathlib import Path

import yaml


class ActorMatcher:
    """Match LLM actor names to canonical dictionary labels."""

    def __init__(self, dictionary_path: str = "crates/fractalaw-core/data/actor-dictionary.yaml"):
        with open(dictionary_path) as f:
            self.entries = yaml.safe_load(f)

        # Build category lookup
        self._categories = {}
        for entry in self.entries:
            self._categories[entry["canonical"]] = entry.get("category", "other")

        # Pre-sort triggers by length (longest first) for Pass 2
        self._all_triggers = []
        for entry in self.entries:
            for trigger in entry["triggers"]:
                self._all_triggers.append((trigger, entry["canonical"]))
        self._all_triggers.sort(key=lambda x: -len(x[0]))

    def match(self, name: str) -> tuple:
        """Match an LLM actor name to a canonical label.

        Returns (canonical_label, confidence) or (None, 0.0) for discoveries.
        """
        n = name.strip().lower()
        if not n:
            return None, 0.0

        # Pass 1: exact trigger match (order-sensitive)
        for entry in self.entries:
            for trigger in entry["triggers"]:
                if n == trigger:
                    return entry["canonical"], 1.0

        # Pass 2: substring containment (longest trigger first)
        for trigger, canonical in self._all_triggers:
            if trigger in n or n in trigger:
                return canonical, 0.85

        # No match — discovery
        return None, 0.0

    def category(self, canonical_label: str) -> str:
        """Get the category prefix for a canonical label."""
        return self._categories.get(canonical_label, "other")

    def is_government(self, canonical_label: str) -> bool:
        """Check if a canonical label is a government/EU actor."""
        cat = self.category(canonical_label)
        return cat in ("Gvt", "EU")


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Actor name matcher")
    parser.add_argument("name", nargs="?", help="Actor name to match")
    parser.add_argument(
        "--test", action="store_true", help="Run test suite"
    )
    parser.add_argument(
        "--dictionary",
        default="crates/fractalaw-core/data/actor-dictionary.yaml",
        help="Path to dictionary YAML",
    )
    args = parser.parse_args()

    matcher = ActorMatcher(args.dictionary)

    if args.test:
        tests = [
            ("employer", "Org: Employer", 1.0),
            ("competent person", "Ind: Competent Person", 1.0),
            ("enforcing authority", "Gvt: Authority: Enforcement", 1.0),
            ("inspector", "Spc: Inspector", 1.0),
            ("importer", "SC: Importer", 1.0),
            ("manufacturer", "SC: Manufacturer", 1.0),
            ("self-employed person", "Ind: Self-employed Worker", 1.0),
            ("Secretary of State", "Gvt: Minister", 1.0),
            ("secretary of state for defence", "Gvt: Minister: Secretary of State for Defence", 1.0),
            ("HSE", "Gvt: Agency: Health and Safety Executive", 1.0),
            ("Health and Safety Executive", "Gvt: Agency: Health and Safety Executive", 1.0),
            ("local authority", "Gvt: Authority: Local", 1.0),
            ("market surveillance authority", "Gvt: Authority: Market", 1.0),
            ("water undertaker", "Svc: Water Undertaker", 1.0),
            ("liquidator", "Spc: Liquidator", 1.0),
            ("young people", "Ind: Young Person", 1.0),
            ("special negotiating body", "EU: Special Negotiating Body", 1.0),
            ("competent national authorities", "Gvt: Authority", 1.0),
        ]

        passed = 0
        failed = 0
        for name, expected_label, expected_conf in tests:
            label, conf = matcher.match(name)
            ok = label == expected_label
            if ok:
                passed += 1
            else:
                failed += 1
                print(f"  FAIL: '{name}' → {label} (expected {expected_label})")

        print(f"Tests: {passed}/{passed + failed} passed")
        sys.exit(0 if failed == 0 else 1)

    if args.name:
        label, conf = matcher.match(args.name)
        if label:
            cat = matcher.category(label)
            gvt = matcher.is_government(label)
            print(f"{label} ({conf:.0%}) [category={cat}, government={gvt}]")
        else:
            print(f"DISCOVERY: '{args.name}' not in dictionary")
    else:
        print(f"Dictionary: {len(matcher.entries)} entries")
        parser.print_help()


if __name__ == "__main__":
    main()
