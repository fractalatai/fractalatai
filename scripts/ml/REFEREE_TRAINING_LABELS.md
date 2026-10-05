# Referee brief: Gemini vs GPT training-label disputes (drrp-v1.1)

You are refereeing provisions where Gemini 3.8 Flash and GPT-5.5 (low) labelled a provision differently with the
same definitive prompt (`drrp-v1.2`). Your decision becomes the training label for that provision.

## Input

`data/training/drrp-v1.1/disputes_batch1.jsonl`, one JSON object per disputed provision:
- `section_id`, `law_name`, `stratum`, `split`;
- `prompt`: exactly what both models saw. That's the stem (ancestors), referenced provisions, applying provisions (#60) and the provision to label;
- `gemini` and `gpt`: each model's label (relation, raw_type, purpose, actors[label, position, holds, inferred, act], reason);
- `diffs`: what differs (relation, raw_type, purpose, position pairs, holds, label-only, actor only in one model).

## The rules

The definitive prompt's rules are binding. **Read `SYSTEM_PROMPT` in `scripts/drrp_prompt.py` first**, and use only labels from `crates/fractalaw-core/data/actor-dictionary.yaml` (or `OTHER: <description>` if none fits).

The points that decide most disputes:
- **Counterparty or beneficiary:** the test is the duty's **act**. The recipient of the act (notified, supplied, consulted, paid, given access, served, charged, whose request it answers) is the counterparty. Beneficiary applies only where a protective purpose is explicit and the party doesn't receive the act. If both apply, it's counterparty.
- **Only `active` actors hold Obligation/Liberty.** Everyone else holds `none`.
- **Relation `no`** for offences and penalties, cross-references, definitions, deeming, application/scope, exemptions, detail provisions (form, manner, conditions or procedure of a relation created elsewhere), parliamentary procedure, amending text, commencement and citation.
- **Passive and thing-subject duties** are Obligations. The holder comes from the stem, a referenced provision or an applying provision (`inferred: true`), otherwise it's unknown (`raw_type` set, no active actor). Never guess a holder.
- **Purpose:** one value, chosen by the precedence in the prompt (machinery > sanctions > Charge+Fee > Requirement/Power Conferred > Procedure+Detail). `Requirement` goes with relation `yes` (Obligation); `Procedure+Detail` goes with relation `no`.
- **v1.2 rulings (Jason, 2026-10-05):** a passive duty's counterparty gets an `act`; a headless stem ("X shall—") is relation yes with X active; a transitional provision that itself confers a time-limited power or duty is relation yes with purpose `Transitional Arrangement`; a Member State or country that is only a place is not listed.
- **`sided_with`** reflects only the disputed fields. Corrections to actors both models agreed on go in the rationale.
- **Not actors:** things such as the environment, animals, property, countries or industries. Leave them out. Instruments (schemes, regulations, notices) are never actors.

## Output

Append one line per provision to `data/training/drrp-v1.1/referee/batch1.jsonl`:

```json
{"section_id": "...", "final": {"relation": "yes|no", "raw_type": "Obligation|Liberty|null", "purpose": "...",
  "actors": [{"label": "...", "position": "...", "holds": "...", "inferred": false, "act": null}]},
 "sided_with": "gemini|gpt|neither|mixed", "rationale": "one sentence naming the deciding rule"}
```

`final` must be a complete label: every actor that should appear, not just the disputed ones. `mixed` means you took parts from each.

## Process

1. **Test batch:** the first 10 disputes. Stop and report back:
   - the 10 decisions;
   - how often you sided with each model;
   - the hard cases;
   - any recurring pattern (e.g. one model systematically wrong on purpose).
2. After review, continue in batches of 50 until the file is done. Report per batch: counts by `sided_with` and the recurring patterns.
3. **Never write to the database,** and never edit the prompt, dictionary or scripts. If a rule seems wrong or missing, report it rather than inventing one.
