---
description: Claude-mediated human adjudication of LLM validation corrections — step through audit log, present each correction, capture decision, write to LanceDB
---

# Skill: Human Review — Adjudication of LLM Corrections

## When This Applies

After running `taxa validate` on one or more laws. The validate command produces audit log JSON files in `data/audit/` with corrections proposed by the LLM. This skill steps through each correction with the human, captures their accept/reject decision, writes accepted corrections to LanceDB with `extraction_method="adjudicated"`, and logs the full adjudication trail.

**Trigger**: User says "review corrections", "adjudicate", "human review", or references the LLM audit log.

## How It Works

### Step 1: Load the audit log

Ask the user which law to review (or accept a law name argument). Load the audit JSON:

```bash
/usr/bin/python3 .claude/skills/human-review/scripts/load_review.py --law UK_uksi_2002_2788
```

This prints a summary: total corrections, breakdown by delta type, provisions affected.

### Step 2: Present each correction

For each correction in the audit log where `delta == "drrp_override"`:

1. Show the provision section_id and text (first 300 chars)
2. Show the pipeline classification (pre_llm_drrp, extraction_method, confidence)
3. Show the LLM correction (llm_drrp) with its reasoning
4. Ask the human: **Accept** (LLM is right), **Reject** (pipeline was right), or **Skip** (unsure, leave for later)

Format each as a clear comparison:

```
--- reg.4(4) ---
  Text:     An employee shall be treated as having satisfied the condition...
  Pipeline: Liberty (regex, 0.70 confidence)
  LLM says: none — "describes a condition under which another condition is treated as satisfied"
  
  Accept / Reject / Skip?
```

### Step 3: Write accepted corrections

For each accepted correction:
1. Write to LanceDB: update `drrp_types` and set `extraction_method = "adjudicated"`
2. Append to `drrp_history` JSON with tier = "adjudicated"
3. The "adjudicated" tier has source_tier = 7 (highest), protecting it from all lower-tier overwrites

### Step 4: Write adjudication trail

Write a JSON file to `data/audit/{law_name}_adjudicated.json`:

```json
{
  "schema_version": 1,
  "law_name": "UK_uksi_2002_2788",
  "source_audit": "data/audit/UK_uksi_2002_2788.json",
  "adjudicator": "human via Claude",
  "timestamp": "2026-06-23T...",
  "decisions": [
    {
      "section_id": "UK_uksi_2002_2788:reg.4(4)",
      "pre_llm_drrp": "Liberty",
      "llm_drrp": "none",
      "decision": "reject",
      "reason": "Beneficial deeming grants Liberty to the employee"
    }
  ],
  "summary": {
    "accepted": 18,
    "rejected": 8,
    "skipped": 2,
    "total": 28
  }
}
```

### Step 5: Report

Print summary: N accepted (written to LanceDB), N rejected (pipeline kept), N skipped.

## Source Tier Protection

The `extraction_method = "adjudicated"` has `source_tier = 7` in the pipeline hierarchy:

| Tier | Method | source_tier |
|------|--------|-------------|
| Regex | regex | 1 |
| Inherited | inherited | 2 |
| Local LLM | local / local_unvalidated | 3 |
| Classifier | classifier | 4 |
| Agentic (unvalidated) | agentic_unvalidated | 5 |
| Agentic | agentic | 6 |
| **Adjudicated** | **adjudicated** | **7** |

Once a provision is adjudicated, no lower tier can overwrite it — even `--force` re-parse will skip it (unless the user explicitly clears the adjudicated status).

## Usage

```bash
# Step 1: Generate corrections (if not already done)
source ~/.bashrc && GEMINI_API_KEY="$GEMINI_API_KEY" \
  cargo run -p fractalaw-cli -- taxa validate --laws UK_uksi_2002_2788

# Step 2: Review corrections (invoke this skill)
/human-review UK_uksi_2002_2788

# Or review all pending audit logs
/human-review --all
```

## Notes

- Only corrections with `delta == "drrp_override"` need human review — `no_change` entries are informational
- The adjudication trail is append-only — re-running review on the same law adds a new adjudication record
- Audit files are gitignored (in `data/`) but should be synced to NAS for retention
- The skill uses AskUserQuestion to present each correction — the human decides in the Claude interface
