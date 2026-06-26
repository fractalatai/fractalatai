# Session: Correlative Actor Inference (PENDING)

## Problem

~200 gold benchmark actors are implied, not stated in text. The LLM infers them from Hohfeldian correlatives — deterministic legal relationships between actors. These patterns are consistent enough to codify as rules.

## Patterns found

| When regex finds | Infer | Position | Coverage |
|-----------------|-------|----------|----------|
| Employee active (Obligation) | Employer | counterparty | 19/28 (68%) |
| Member State active (EU reg) | Responsible Undertaking | counterparty | 16/22 (73%) |
| Enforcement Authority active | Public | beneficiary | 7/11 (64%) |

## Design questions

1. Where does this fit in the pipeline? New stage after parse, extension of inheritance, or separate command?
2. Should inferred actors write to `provision_actors` with a separate tier (e.g. `"inferred"`) to keep signals distinct?
3. How to handle false positives — the coverage is 64-73%, not 100%. Should low-coverage rules flag `pending_llm` instead?
4. Are there more correlative pairs beyond these three?

## Key principle

This is a deterministic inference layer — not regex (text patterns) and not classifier (ML features). It reasons about relationships between actors already found, not about the text itself.

## Dependencies

- ✅ provision_actors table with regex signals
- Regex actor gaps session addresses the 1,600 trigger-present-but-unmatched actors first
