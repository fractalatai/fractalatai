# Justifier brief: gold v3 labels with rule citations (phase 0a)

You are proposing the **gold** label for each provision and justifying every field with rule IDs from the rule catalogue. Jason will review your table and approve or correct each row. He is not an expert on hard provisions, so your job is to make each decision checkable: name the rule, quote the deciding words, and grade how hard it was.

## Input

`data/gold/v3/evidence.jsonl` (or the batch file you're given), one JSON object per provision:
- `context`: exactly what the labelling models saw (stem, referenced and applying provisions, the provision itself);
- `headings`: the Part/Chapter titles and cross-heading above it;
- `law_title`;
- `tiers`: the pipeline's own outputs: stored `purposes` (legacy, mostly a catch-all), `drrp_types`, and per actor the regex / cls / slm / llm / final positions;
- `labels`: v1.3 labels from Gemini (`gemini`), GPT-5.4-mini (`mini`), GPT-5.5 (`gpt55`), and the Opus referee's decision (`referee`), where they exist;
- `cue`: the coarse purpose cue's guess.

The evidence is **evidence, not truth**. The models followed an older prompt (`drrp-v1.3`), which differs from the spec on the points listed below.

## The rules (binding, in this order)

1. `docs/architecture/DRRP-RULE-CATALOGUE.md`: **read it first.** Every relation/actor field cites its IDs (e.g. `REL-01`, `HOLD-05`, `POS-06`, `ACT-03`). Retired (struck-through) rules are not cited.
2. `docs/architecture/DRRP-CLASSIFICATION.md` (the spec), including the 2026-10-06 rulings. Where the catalogue and the spec differ, the spec wins; say so in the reason.
3. `docs/architecture/PURPOSE-CLASSIFICATION.md` § "Layered purpose": the **12 coarse classes**.
4. Actor labels only from `crates/fractalaw-core/data/actor-dictionary.yaml`, or `OTHER: <description>` if none fits (that's a `new_edge`).

**Where the spec now differs from the v1.3 prompt the models used** (follow the spec, and expect the evidence to disagree):
- **Functions lists** (REL-44): relation `yes`. A government body's functions are an Obligation; a governed party's are a Liberty. Provisions that only mention functions are `no`.
- **An applying provision's own row** (REL-26): "X shall comply with / ensure compliance with the requirements of SCOPE" is a pointer, so relation `no`, with X `mentioned`. The supervisory form (HOLD-07) and the prohibition form (HOLD-09) are still duties.
- **`serve` is folded into `notify`** (ACT-03, ACT-12). The acts are: `notify`, `supply`, `consult`, `pay`, `give_access`, `charge`, `answer_request`, `other`. Map the verb using the synonym table (spec layer 1b).
- **#67 access rights** (INF-01) also apply when the access duty's holder is unknown ("open to inspection by the public" → the public is `active`, holds `Liberty`, `inferred: true`).
- **One entry per label, strongest role** (POS-15) is still in force. If a second role or a second same-label person is dropped, say what was dropped in the reason. That feeds #78.
- **Cross-instrument holders** (HOLD-14): the holder comes from the parent Act when you can resolve it from the text given. Otherwise it is holder unknown.

## Fields to propose (per provision)

| field | value | rule IDs |
|---|---|---|
| `relation` | `"yes"` / `"no"` | REL-* |
| `raw_type` | `"Obligation"` / `"Liberty"` / `null`; set only when relation is yes and no actor is active | HOLD-15, DEF-02, TYPE-* |
| `purpose` | one of the 12 coarse classes, exactly as written in PURPOSE-CLASSIFICATION.md | `P:<class>` plus the deciding cue, e.g. `P:Interpretation` |
| `actor` (one per label) | `{"position", "holds", "inferred", "act"}` | POS-*, HOLD-*, TYPE-*, INF-*, ACT-*, LBL-* |

Purpose detail (Substantive vs Procedure/Detail under Requirements) is **not** proposed: it is derived from purpose plus relation when the gold set is frozen (Jason, 2026-10-06). Purpose is always proposed, including for relation `no` rows. There are no purpose rule IDs yet (the purpose catalogue comes with phase 2), so cite `P:<class>` and quote the deciding words in the reason.

## Difficulty (per field)

- **`easy`**: one rule clearly applies, and the evidence that exists (models, referee, pipeline) agrees with you, or there is none and the text is plain.
- **`hard`**: the evidence disagrees with you, two rules pull different ways (name both), or you are unsure.
- **`new_edge`**: no rule in the catalogue fits. Set `rule_ids: ["NEW"]` and write the candidate rule in the reason, in one line.

Be honest: a confident wrong `easy` costs Jason more than a `hard`.

## Output

Write one JSON line per provision to the output file you are given:

```json
{"section_id": "...", "text_md5": "...", "fields": [
  {"field": "relation", "proposed": "yes", "rule_ids": ["REL-01", "REL-42"], "reason": "…", "difficulty": "easy"},
  {"field": "raw_type", "proposed": null, "rule_ids": ["HOLD-02"], "reason": "…", "difficulty": "easy"},
  {"field": "purpose", "proposed": "Requirements", "rule_ids": ["P:Requirements"], "reason": "…", "difficulty": "easy"},
  {"field": "actor", "actor_label": "Org: Employer", "proposed": {"position": "active", "holds": "Obligation", "inferred": false, "act": null},
   "rule_ids": ["HOLD-01", "TYPE-01"], "reason": "…", "difficulty": "easy"}
]}
```

- **`reason`:** one plain-English sentence for a non-expert. It must quote the deciding words from the text, e.g. `"shall ensure … are not exposed": a protective duty with no act received by persons not employed, so beneficiary (POS-06)`.
- List **every** actor that should appear, including `mentioned` ones. Don't list things that aren't actors (LBL rules).
- If the text of the provision is empty or a bare heading, emit only `relation: no` and `purpose`, with a reason.

## Process

1. **Pilot:** the batch file you are given. Write the output and then report:
   - counts by difficulty per field;
   - the `new_edge` candidates;
   - where you disagreed with the referee or Gemini, and why;
   - any rule in the catalogue that was hard to apply or seems wrong.
2. **Never write to the database,** and never edit the docs, the dictionary or the scripts. Report rule problems; don't fix them.
3. **No paid APIs.** Read the files only.
