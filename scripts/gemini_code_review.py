#!/usr/bin/env /usr/bin/python3
"""Gemini code review with context caching.

Creates a cached context with the DRRP pipeline source code, then runs
targeted review prompts against it. The cache avoids re-reading ~12K lines
of Rust on every question.

Usage:
    source ~/.bashrc
    GEMINI_API_KEY="$GEMINI_API_KEY" /usr/bin/python3 scripts/gemini_code_review.py
"""

import os
import sys
import json
import google.genai as genai

# ── Extract relevant code sections ──────────────────────────────────

def read_file(path):
    with open(path) as f:
        return f.read()

def read_lines(path, start, end):
    """Read lines start..end (1-indexed, inclusive)."""
    with open(path) as f:
        lines = f.readlines()
    return "".join(lines[start - 1 : end])

def extract_main_rs_sections():
    """Extract only enrichment-relevant sections from the 7300-line main.rs."""
    path = "crates/fractalaw-cli/src/main.rs"
    with open(path) as f:
        lines = f.readlines()

    sections = []
    # Find key function boundaries by scanning for function signatures
    in_section = False
    section_name = ""
    section_start = 0
    depth = 0

    # We want these sections:
    targets = {
        "struct ActorEntry": (None, None),
        "fn enrich_single_law": (None, None),
        "fn parse_tier3_actors": (None, None),
        "fn parse_gemini_response": (None, None),
        "struct ActorMatcher": (None, None),
        "fn cmd_taxa_enrich": (None, None),
    }

    # Simple extraction: find each target, grab ~200 lines after it
    for target in targets:
        for i, line in enumerate(lines):
            if target in line:
                # Find the end of this function/struct (next blank line after closing brace at same indent)
                start = i
                end = min(i + 300, len(lines))  # cap at 300 lines
                # Look for function end
                brace_depth = 0
                for j in range(i, min(i + 500, len(lines))):
                    brace_depth += lines[j].count('{') - lines[j].count('}')
                    if brace_depth <= 0 and j > i + 5:
                        end = j + 1
                        break
                targets[target] = (start + 1, end)
                break

    result = []
    for target, (start, end) in targets.items():
        if start and end:
            result.append(f"// ── {target} (lines {start}-{end}) ──")
            result.append("".join(lines[start - 1 : end]))
            result.append("")

    return "\n".join(result)


# ── Build code context ──────────────────────────────────────────────

def build_code_context():
    """Assemble all code sections into a single context string."""
    sections = []

    # Full files (small enough to include completely)
    for path, label in [
        ("crates/fractalaw-core/src/taxa/mod.rs", "taxa/mod.rs — parse_v2 pipeline + position heuristic"),
        ("crates/fractalaw-core/src/taxa/duty_type.rs", "taxa/duty_type.rs — DRRP classification"),
        ("crates/fractalaw-core/src/taxa/duty_patterns_v2.rs", "taxa/duty_patterns_v2.rs — actor-anchored regex patterns"),
        ("crates/fractalaw-core/src/taxa/purpose.rs", "taxa/purpose.rs — purpose gate"),
        ("crates/fractalaw-ai/src/drrp_classifier.rs", "ai/drrp_classifier.rs — DRRP classifier (Obligation/Liberty/none)"),
        ("crates/fractalaw-ai/src/position_classifier.rs", "ai/position_classifier.rs — position classifier (active/counterparty/other)"),
    ]:
        content = read_file(path)
        sections.append(f"## {label}\n\n```rust\n{content}\n```\n")

    # Extracted sections from main.rs
    main_sections = extract_main_rs_sections()
    sections.append(f"## main.rs — enrichment pipeline (extracted sections)\n\n```rust\n{main_sections}\n```\n")

    # Actor dictionary for reference
    dict_content = read_file("docs/actor-dictionary.yaml")
    sections.append(f"## Actor dictionary (YAML)\n\n```yaml\n{dict_content}\n```\n")

    return "\n\n".join(sections)


# ── Review prompts ──────────────────────────────────────────────────

REVIEW_PROMPTS = [
    {
        "id": "regex_patterns",
        "title": "Regex Pattern Coverage",
        "prompt": """Review the duty_patterns_v2.rs regex patterns for UK/EU legislative text.

Specifically:
1. Are there common UK legislative patterns that the regexes miss? Think about:
   - "It shall be the duty of X to..." (inverted modal-actor order)
   - "X has a right to..." / "X is entitled to..."
   - "X may by regulations..." (power patterns)
   - Passive voice: "A notice shall be served on X by Y"
2. Are any patterns too greedy (matching non-DRRP text)?
3. Is the ordering of specific-before-generic correct?
4. Are there EU-specific patterns missing (directive language: "Member States shall ensure...")?

Be specific — cite line numbers and give example provision text that would be missed."""
    },
    {
        "id": "purpose_gate",
        "title": "Purpose Gate Logic",
        "prompt": """Review the purpose.rs classification and the should_skip_drrp gate in mod.rs.

1. Is the gate too aggressive? Are there provision types that should get DRRP but are being skipped?
2. Is the gate too lenient? Are definitions/interpretations/commencement provisions leaking through?
3. The governed-actor override: if a provision has an actor keyword, it overrides certain purpose gates. Is this correct?
4. The descriptive_summary check: is it catching the right cases?

Think about edge cases:
- "For the purposes of this Part, 'employer' means..." (definition with actor keyword)
- Schedule provisions that create new duties
- Cross-reference provisions that impose obligations"""
    },
    {
        "id": "position_heuristic",
        "title": "Actor Position Heuristic",
        "prompt": """Review the position heuristic in mod.rs (the actor_positions HashMap construction).

The current logic: the actor matched by the DRRP regex pattern = active, all others = counterparty.
The ±3 byte tolerance handles padding offset differences.

1. Is this heuristic sound? When does it fail?
2. Multi-duty-bearer provisions: "duty of every employer and every self-employed person" — only one gets active. How should this be handled?
3. Government patterns where the pattern matches the government actor but the governed actor is the duty-bearer
4. Are there cases where "counterparty" should actually be "beneficiary" or "mentioned"?"""
    },
    {
        "id": "classifier_features",
        "title": "Classifier Feature Engineering",
        "prompt": """Review the DRRP classifier (drrp_classifier.rs) and position classifier (position_classifier.rs).

1. DRRP classifier: 384-dim embedding + 13 modal features. Are the modal features the right ones? What's missing?
2. Position classifier: adds DRRP type (5), actor category (10), and text offset (1). Is text offset a reliable signal?
3. The 3-class hierarchy (Obligation/Liberty/none for DRRP, active/counterparty/other for position) — are these the right groupings?
4. Both use softmax(X @ W + b) from JSON weights. Any numerical concerns (overflow, precision)?
5. The decompose_drrp function maps Obligation → Duty/Responsibility based on actor category. Is this decomposition always correct?"""
    },
    {
        "id": "confidence_protection",
        "title": "Confidence Protection & Data Flow",
        "prompt": """Review the confidence protection logic across the pipeline.

The hierarchy: agentic (0.90) > classifier (0.85) > regex (0.30-0.80).
Higher-confidence data survives re-enrichment.

1. Is the confidence ordering correct? Should classifier ever overwrite agentic?
2. The position classifier writes to the `reason` field on disagreement — does this interact correctly with confidence protection?
3. When --force re-enrichment runs, existing classifier/agentic data is supposed to be protected. Trace the flow — are there gaps?
4. The enrichment queue (enrichment_pending) — can a law get stuck in the queue?
5. merge_insert write patterns: are there race conditions if two processes enrich the same law?"""
    },
]


# ── Main ────────────────────────────────────────────────────────────

def main():
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        print("Error: GEMINI_API_KEY not set")
        sys.exit(1)

    client = genai.Client(api_key=api_key)

    # Build the code context
    print("Extracting code sections...")
    code_context = build_code_context()
    print(f"Code context: {len(code_context):,} chars (~{len(code_context) // 4:,} tokens)")

    # Create cached context
    print("Creating Gemini cache...")
    cache = client.caches.create(
        model="gemini-2.5-flash",
        config={
            "display_name": "fractalaw-drrp-code-review",
            "system_instruction": """You are a senior Rust developer reviewing a legal data classification pipeline.
The codebase classifies UK/EU legislation provisions into DRRP types (Duties, Rights, Responsibilities, Powers)
using a cascade: regex patterns → ML classifiers → LLM (Gemini).

Each provision has actors (who bears the duty) with Hohfeldian positions (active/counterparty/beneficiary/mentioned).

The code is production — serving a legal compliance SaaS. Your review should focus on correctness, edge cases,
and missed patterns. Be specific: cite line numbers, give example provision text, and suggest concrete fixes.

Do not suggest style/formatting changes. Focus on logic, correctness, and coverage.""",
            "contents": [
                {"role": "user", "parts": [{"text": f"Here is the complete DRRP pipeline source code for review:\n\n{code_context}"}]},
                {"role": "model", "parts": [{"text": "I've loaded the DRRP pipeline source code. I can see the regex patterns, purpose gates, position heuristic, classifiers, and enrichment pipeline. Ready for targeted review questions."}]},
            ],
            "ttl": "3600s",  # 1 hour cache
        },
    )

    print(f"Cache created: {cache.name}")
    print(f"Token count: {cache.usage_metadata}")
    print()

    # Save cache name for reuse
    cache_file = "data/gemini_review_cache.json"
    with open(cache_file, "w") as f:
        json.dump({"cache_name": cache.name, "model": "gemini-2.5-flash"}, f)
    print(f"Cache name saved to {cache_file}")
    print()

    # Run review prompts
    results_dir = "data/code-review"
    os.makedirs(results_dir, exist_ok=True)

    for review in REVIEW_PROMPTS:
        print(f"Running review: {review['title']}...")
        try:
            response = client.models.generate_content(
                model="gemini-2.5-flash",
                contents=review["prompt"],
                config={
                    "cached_content": cache.name,
                    "temperature": 0.3,
                    "max_output_tokens": 4096,
                    "thinking_config": {"thinking_budget": 2048},
                },
            )

            result_path = f"{results_dir}/{review['id']}.md"
            with open(result_path, "w") as f:
                f.write(f"# Code Review: {review['title']}\n\n")
                f.write(f"**Model**: Gemini 2.5 Flash (cached context)\n")
                f.write(f"**Date**: 2026-06-11\n\n")
                f.write(response.text or "(no response)")

            print(f"  → {result_path}")
        except Exception as e:
            print(f"  Error: {e}")

    print("\nDone. Review results in data/code-review/")
    print(f"Cache valid for 1 hour: {cache.name}")
    print("To ask follow-up questions, use the cache name in subsequent API calls.")


if __name__ == "__main__":
    main()
