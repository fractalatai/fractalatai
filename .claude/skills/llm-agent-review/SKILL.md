---
description: Claude-agent review of pending_llm actors (the LLM tier) in batches: same labelling contract and write path as the Gemini llm-batch, with a per-decision rationale log, a test batch before bulk, and promotion + provenance afterwards.
---

# LLM Agent Review — pending_llm actors

## When This Applies

After reconcile, when actors are flagged `extraction_method = 'pending_llm'` (regex, classifier and SLM disagree) and the run wants a Claude agent to adjudicate them, with a written rationale per decision, instead of the Gemini batch (`llm-batch` skill, `scripts/gemini_llm_batch.py`). Both paths fill the same LLM tier columns, so downstream reconcile/backfill don't care which ran.

**Trigger:** "LLM review", "work through the pending_llm actors", "agent LLM actor work".

## Rules

- **Scope to the run's law list.** Never touch benchmark laws (`gold_benchmarks`) or actors outside the queue.
- **The agent writes only `llm_drrp` / `llm_position`.** It never changes `extraction_method`, `drrp`, `position` or other tables, runs no DDL or fractalaw commands, and never publishes. The parent session does the promotion.
- **A test batch of 10 comes first; the agent stops and reports.** The parent reviews it before the agent continues in batches of 50. Every batch is verified (the rows now have `llm_position`) before the next starts.

## Work queue

```sql
SELECT pa.section_id, pa.actor_label, pa.actor_category, pa.regex_drrp, pa.regex_position,
       pa.cls_position, pa.slm_drrp, pa.slm_position, pa.slm_confidence, lt.text
FROM provision_actors pa JOIN legislation_text lt USING (section_id)
WHERE pa.extraction_method = 'pending_llm' AND pa.llm_position IS NULL
  AND lt.law_name = ANY(string_to_array(:laws, ','))
  AND lt.law_name NOT IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)
ORDER BY pa.section_id, pa.actor_label
```

Hub: `PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw fractalaw` (or `/usr/bin/python3` + psycopg2).

## Labelling contract (identical to `gemini_llm_batch.py` `SYSTEM_PROMPT`)

For each (provision, actor):
1. **DRRP of the provision for this actor:**
   - `Obligation`: imposes a duty or prohibition;
   - `Liberty`: grants a power, permission or entitlement;
   - `none`: no obligation or liberty is created for this actor.
2. **Hohfeldian position** (lowercase):
   - `active`: bears the duty or exercises the power/liberty;
   - `counterparty`: to whom the duty is owed, or subject to the power;
   - `beneficiary`: benefits but is neither duty-bearer nor direct correlative;
   - `mentioned`: referenced, with no active legal role.

**Guidance:**
- For sub-paragraphs and list items, read the parent stem and siblings: rows of the same law whose `section_id` is a prefix, e.g. `L:reg.4(2)` for `L:reg.4(2)(a)`. The duty or power often sits in the stem.
- Classify what the provision **itself** creates. A provision that only references, conditions, details, defines or exempts a relation created elsewhere is `none` (the actor is usually `mentioned`).
- Offences and penalties are `none`, not Obligation.
- The regex/classifier/SLM values are hints; they disagree, which is why the actor is here.
- Be decisive. If genuinely unclear, prefer `none`/`mentioned`, with the reason in the rationale.

## Write path (the agent, per batch)

1. In one transaction per batch, check each update touched exactly 1 row:
   ```sql
   UPDATE provision_actors SET llm_drrp = %s, llm_position = %s
   WHERE section_id = %s AND actor_label = %s
     AND extraction_method = 'pending_llm' AND llm_position IS NULL
   ```
2. Log every decision to `data/<run>/llm_review/batch_NN.jsonl`: `section_id, actor_label, drrp, position, rationale` (one sentence), plus the prior `slm_drrp`, `slm_position`, `regex_drrp`.
3. **Report back:**
   - after the test batch: the 10 decisions, agreement with the SLM, hard rows, data oddities;
   - at the end: totals by drrp × position, SLM agreement, and skipped rows with the reason.

## Agent brief

Spawn one general-purpose agent in the background, and continue it (SendMessage) after the test-batch review. Give it:
- the law-list file;
- the output dir;
- this skill's Work queue, Labelling contract, Write path and Rules verbatim;
- the batching instruction: test batch of 10 → stop and report → batches of 50 until the queue is empty.

The same steps can also be run directly in the main session without an agent.

## After the queue is empty (the parent session)

1. **Spot-check.** Read a sample of the JSONL rationales, especially disagreements with the SLM on Obligation rows.
2. **Promote** (same as `llm-batch`), scoped to the run's laws:
   ```sql
   UPDATE provision_actors pa
   SET extraction_method = 'llm', drrp = llm_drrp, position = llm_position, reconcile_confidence = 'HIGHEST'
   FROM legislation_text lt
   WHERE pa.section_id = lt.section_id AND lt.law_name = ANY(string_to_array(:laws, ','))
     AND pa.llm_position IS NOT NULL AND pa.extraction_method = 'pending_llm';
   ```
3. **Record provenance (#63)** for the laws that had reviewed actors. The model is the agent's model id; the prompt version is this skill's content hash:
   ```bash
   cd scripts/ml && /usr/bin/python3 -c "
   import psycopg2, fractalaw_provenance as p
   laws = open('<laws-with-reviewed-actors.txt>').read().split()
   c = psycopg2.connect('host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw')
   p.record(c, laws, 'taxa', 'llm', 'llm_agent', '<agent model id>',
            prompt_version=p.prompt_version(open('../../.claude/skills/llm-agent-review/SKILL.md').read()))"
   ```
4. **Continue the pipeline:** derive_hierarchy → `taxa backfill`, per `customer-batch-parse` steps 8b–9.

## Notes

- **Cost and time:** the agent reads context per actor, so it's slower than Gemini (minutes per 50) but gives an auditable rationale per decision.
- **If agent spawning is unavailable** (e.g. a tool-safety check outage), don't block the pipeline: either run the review directly in-session per this skill, or fall back to `llm-batch` (Gemini).
