---
description: Match LLM-generated actor names to canonical dictionary labels. Used in post-processing after Tier 2/3 classification.
---

# Skill: Actor Match

## When This Applies

After an LLM (Gemini or Gemma) names actors in natural language, this matcher resolves them to canonical dictionary labels. Used by the QA write-back and Tier 2/3 enrichment pipelines.

## Usage

```python
from actor_match import ActorMatcher

matcher = ActorMatcher("crates/fractalaw-core/data/actor-dictionary.yaml")

# Match a single name
label, confidence = matcher.match("enforcing authority")
# → ("Gvt: Authority: Enforcement", 1.0)

# Match with discovery detection
label, confidence = matcher.match("water undertaker")
# → (None, 0.0) — discovery, not in dictionary

# Get the category for DRRP decomposition
category = matcher.category("Org: Employer")
# → "Org"
```

## How It Works

1. Loads `crates/fractalaw-core/data/actor-dictionary.yaml` — the single source of truth
2. Pass 1: exact trigger match (order-sensitive, specific before generic)
3. Pass 2: substring containment (longest trigger first for specificity)
4. Unmatched → discovery (confidence 0.0)

## Dictionary location

`crates/fractalaw-core/data/actor-dictionary.yaml` — tracked in git, shared with sertantai.
