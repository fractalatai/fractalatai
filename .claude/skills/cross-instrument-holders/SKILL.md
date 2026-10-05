---
description: Periodic Claude-agent pass (#77) that finds holder-unknown duties in secondary legislation whose holder is named by a duty in another Act (usually the parent Act, via DuckDB enacted_by), decides the holder with a rationale, and writes Jason-approved holders to the adjudicated tier. Run every ~3 months, when the reminder hook or lat-sync says it's due.
---

# Cross-instrument duty holders (#77)

## When This Applies

- The SessionStart reminder or the lat-sync closing check says the pass is due (last run > 90 days).
- After a large LAT/LRT sync that brought in new regulations or orders.
- **Trigger:** "cross-instrument holders", "parent Act holders", "run the #77 pass".

**Why a periodic pass, not the prompt:** the definitive prompt (`drrp-v1.1`, #60) resolves holders within the same law only. A regulation that says "a no-smoking sign must be displayed … in accordance with the duty at section 6(1) of the Act" has its holder in the parent Act (Health Act 2006 s.6(1): "any person who occupies or is concerned in the management of smoke-free premises"). These provisions are few (6 in 4 laws on 2026-10-05) and easy to pinpoint, so a Claude agent reviews them occasionally instead (Jason, 2026-10-05). Spec: `docs/architecture/DRRP-CLASSIFICATION.md`, special case "Holder named in another instrument".

## Rules

- **Nothing is written without Jason's approval.** The agent writes `decisions.jsonl` only. `apply_decisions.py --apply` runs after Jason approves.
- **Adjudicated tier only:** `adj_drrp` / `adj_position = 'active'` / `adj_note`. It survives re-parse (`delete_inferred` keeps `adj_position` rows) and wins reconcile. Never overwrite an existing adjudicated value; never touch other tiers.
- **Benchmark laws are excluded** (`gold_benchmarks`, DuckDB `is_benchmark`).
- **Use dictionary labels only** (`crates/fractalaw-core/data/actor-dictionary.yaml`). If none fits, record the gap in the report (actor-drift) and leave that holder out.
- **Use only a holder the cited provision actually names.** If the cited provision is a power, a definition, or names no one, the decision is "no holder".

## Steps

1. **Fill `enacted_by` gaps** (DuckDB `legislation.enacted_by` is how "the Act" resolves; legal holds the missing ones):
   ```bash
   /usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/cross-instrument-holders/scripts/find_candidates.py --gaps
   ```
   Pull LRT for the laws in `enacted_by_gaps.txt` with the lrt-sync skill. That's optional for small gaps: candidates also resolve by the instrument's own definition ("the 1990 Act" means …) or a named Act.
2. **Find candidates** (read-only):
   ```bash
   /usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/cross-instrument-holders/scripts/find_candidates.py
   ```
   This writes `data/audit/cross_instrument/<date>/candidates.jsonl`: each holder-unknown duty (live, substantive, Obligation, no active or adjudicated actor) in a hub secondary law that cites "the duty/requirement under section N of the [YYYY] Act" or a named Act. Each row carries:
   - its stem;
   - the resolved parent provision with its stem, or its subsections when the section has no text of its own;
   - how the parent Act was resolved (`named`, `defined`, `enacted_by`).
   Unresolved cites are listed; resolve them by hand or skip them.
3. **Agent review.** Spawn one general-purpose agent, or do it in-session if there are only a handful. Give it the candidates file, the Rules above, and this contract. For each candidate:
   - Does THIS provision create (or complete) a duty whose holder is the party the cited provision puts under that duty? E.g. "sign must be displayed in accordance with the duty at s.6(1)": the holder is s.6(1)'s occupier/manager.
   - Or is the cited duty only a reference point? E.g. "information demonstrating compliance with duties under section 89 is a statement by …" defines content, and "for the purposes of their duty under section 91" is a purpose clause. Those are "no holder".
   - Output one line per candidate to `data/audit/cross_instrument/<date>/decisions.jsonl`:
     `{"section_id", "parent_section_id", "holders": [{"label", "drrp": "Obligation"}], "rationale": "<one sentence>", "approved": false}`
4. **Jason approves.** Present the decisions as a short table (provision, holder(s), parent provision, rationale). Set `"approved": true` on the ones he accepts.
5. **Write** (dry run first):
   ```bash
   /usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/cross-instrument-holders/scripts/apply_decisions.py data/audit/cross_instrument/<date>/decisions.jsonl
   /usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/cross-instrument-holders/scripts/apply_decisions.py data/audit/cross_instrument/<date>/decisions.jsonl --apply
   ```
   `--apply` stamps `last_run` (commit it: "cross-instrument holders: pass <date>"). If nothing was approved, stamp anyway, so the reminder resets: `date +%F > .claude/skills/cross-instrument-holders/last_run`.
6. **Downstream:** the written laws need reconcile → `taxa backfill` → publish with the next batch (customer-batch-parse steps 8b–9). Don't publish on its own (build then run once).

## Reminder

- `scripts/check_due.py` prints a reminder when `last_run` is more than 90 days old, and stays silent otherwise. It runs from a SessionStart hook (`.claude/settings.json`) and from the lat-sync skill's closing step.
- `last_run` is committed, so the reminder is shared across machines.

## Notes

- **Size on 2026-10-05:** 6 candidates in 4 laws.
  - Building Safety (Higher-Risk Buildings) Regs 2023 → BSA 2022 s.89, s.91(1)(c);
  - Building Regs 2012 (reg.32) → Building Act 1984 s.91;
  - Welsh separate collection regs 2023 → EPA 1990 s.89(1)(c);
  - smoke-free signs 2012 → Health Act 2006 s.6(1).
- **enacted_by vs "the Act":** `enacted_by` lists the enabling Act. An instrument can cite a different Act ("the 1990 Act" in a regulation made under a 2008 Act), so the instrument's own definitions win.
- **Same-law holders are not this skill:** the labelling prompt handles them (APPLYING PROVISIONS, #60).
