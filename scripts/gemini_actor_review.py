#!/usr/bin/env /usr/bin/python3
"""Send actor dictionary architecture to Gemini for code review.

Reviews the three-source-of-truth problem and proposes a unified design.
"""

import os
import sys

# Load the three source files
with open("crates/fractalaw-core/src/taxa/actors.rs") as f:
    actors_rs = f.read()
with open("docs/actor-dictionary.yaml") as f:
    actor_yaml = f.read()
with open("crates/fractalaw-core/src/taxa/duty_patterns.rs") as f:
    duty_patterns = f.read()

# Extract just the GOVERNMENT_ACTORS list from duty_patterns.rs
import re
gov_actors_match = re.search(
    r'const GOVERNMENT_ACTORS.*?;', duty_patterns, re.DOTALL
)
gov_actors_section = gov_actors_match.group() if gov_actors_match else "NOT FOUND"

prompt = f"""# Actor Dictionary Architecture Review

## Context

This is a UK ESH (environment, safety, health) regulatory data pipeline that classifies legislative provisions into DRRP types (Duties, Rights, Responsibilities, Powers). The critical distinction:

- **Governed actors** (Org, Ind, SC, Spc categories): when they bear an obligation → **Duty**; when they have permission → **Right**
- **Government actors** (Gvt, EU categories): when they bear an obligation → **Responsibility**; when they have discretion → **Power**

An actor CANNOT be both governed and government. The classification determines the DRRP type.

## The Three-Source-of-Truth Problem

### Source 1: `actors.rs` — Rust regex extraction (92 patterns)

Two hardcoded `const` arrays: `GOVERNMENT_DEFS` and `GOVERNED_DEFS`, each containing `(label, regex_pattern)` tuples. Compiled at startup into `LazyLock<Vec<(&str, Regex)>>`. Used by `parse_v2()` to extract actors from provision text.

Issues:
- Adding an actor requires recompiling
- The boundary wrapper `(?:[\\s[:punct:]])` is repeated for every pattern
- Specialist patterns (Offshore, Public) are separate `const` arrays gated by family
- `run_patterns()` does progressive match-and-remove, which means pattern ORDER matters for overlapping keywords

### Source 2: `actor-dictionary.yaml` — LLM post-processing (105 entries)

YAML file with `canonical`, `category`, `triggers`. Used by the `actor-match` skill to map LLM-generated names (e.g., "the employer") to canonical labels (e.g., "Org: Employer").

Issues:
- Manually kept in sync with `actors.rs` but can drift
- Contains entries not in `actors.rs` and vice versa
- `category` field duplicates the prefix of `canonical` (e.g., category="Gvt" for "Gvt: Minister")

### Source 3: `duty_patterns.rs` GOVERNMENT_ACTORS — pattern matching (27 keywords)

A flat `const` list of downcased keywords used by `has_government_actor()` to check if a government actor is present (for government DRRP pattern matching). Must manually stay in sync with `actors.rs` GOVERNMENT_DEFS.

{gov_actors_section}

## Key Questions

1. **Should we unify into a single data-driven dictionary?** A single YAML/TOML file that defines: label, category (governed/government), regex patterns, LLM triggers, and the downcased keywords for pattern matching. The Rust code would parse this at startup.

2. **What format?** YAML is already used for `actor-dictionary.yaml`. Could extend it to include regex patterns. Or use a custom format.

3. **Performance implications?** Currently patterns compile once via `LazyLock`. A data-driven approach would compile on startup. For 92 patterns this is negligible.

4. **The governed/government classification**: for each missing entity below, which category should it be?

Missing entities from benchmark analysis (not in any dictionary):
- NDA (Nuclear Decommissioning Authority)
- Administrator (generic, in energy/climate legislation)
- scheme administrator
- compliance body / compliance bodies
- responsible undertaking
- certification body
- approval body
- authorised person (already matches "Ind: Person" but should it have its own label?)
- manufacturers (plural of SC: Manufacturer — already exists?)

5. **The PERSON_QUALIFIERS compound predicate issue**: `Ind: Person` requires compound predicates ("a person who...", "every person...") to avoid false positives on bare "person". Is there a cleaner way to handle this? Should "authorised person", "responsible person", "competent person" be promoted to separate actor labels rather than relying on compound predicates?

6. **Architecture smell**: `run_patterns()` does `regex.find()` then `regex.replace()` to remove matches — this mutates the text progressively. Is there a more robust approach?

## Source files (abbreviated for review)

### actors.rs (key sections)

```rust
{actors_rs[:8000]}
```

### actor-dictionary.yaml (first 100 lines)

```yaml
{actor_yaml[:4000]}
```

## What I want

1. A clear recommendation: unify or keep separate, with rationale
2. If unify: proposed format, migration path, and which code modules change
3. For each missing entity: governed or government classification with reasoning
4. Whether "authorised person" etc. deserve their own labels
5. Any performance or correctness concerns with the current `run_patterns()` approach
"""

try:
    from google import genai

    client = genai.Client(api_key=os.environ["GEMINI_API_KEY"])
    response = client.models.generate_content(
        model="gemini-2.5-flash",
        contents=prompt,
    )
    print(response.text)

    # Save to file
    with open("data/code-review/gemini-actor-architecture-review.md", "w") as f:
        f.write(f"# Gemini Actor Architecture Review\n\n")
        f.write(f"Date: 2026-06-17\n\n")
        f.write(response.text)
    print("\n\nSaved to data/code-review/gemini-actor-architecture-review.md")

except Exception as e:
    print(f"Error: {e}", file=sys.stderr)
    sys.exit(1)
