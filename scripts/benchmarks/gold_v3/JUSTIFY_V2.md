# Justifier brief v2: sentence units on catalogue v2 (meta-plan phase C)

You propose the **gold** labels for legal sentences and justify each field with the **principles** of catalogue v2.
Jason reviews every field on the sentence page and approves, changes or queries it. Make each decision checkable:
name the principle, quote the deciding words, and grade how hard it was honestly.

This brief replaces `JUSTIFY_GOLD.md` and `JUSTIFY_UNITS.md` (phase 0a/A history).

## Binding documents, in this order

1. `docs/architecture/DRRP-RULE-CATALOGUE.md`: **catalogue v2, read it first.** Cite v2 principle IDs only. A v1 ID
   appears there only in the crosswalk; never cite a merged, retired, precedent or dictionary ID.
2. `docs/architecture/PURPOSE-CLASSIFICATION.md`: the 12 coarse purpose classes, plus `inherit`. **Subordinate legislation** (added 2026-10-07) takes every power or duty to make regulations, orders, rules, schemes or byelaws, what they may contain, and how they are made (laying, annulment, consultation); commencement by order stays Citation and commencement.
3. Actor labels only from `crates/fractalaw-core/data/actor-dictionary.yaml`, or `OTHER: <short description>` when
   none fits (a `new_edge`, and a dictionary candidate).

`DRRP-CLASSIFICATION.md` (the spec) is being trimmed to v2. Where it disagrees with the catalogue, the catalogue
wins.

## Input

Each line of the evidence file you're given is one sentence (`unit_id` = its first row):
- `text`: the sentence as the law reads it (lead-in, items indented, closing words); `members`: its source rows;
- `law_title`, `headings`; `context`: referenced and applying provisions and the law's definitions of terms used;
- `flags`: assembler flags. `proviso_row` is a sibling "But …" row qualifying the row before it;
  `section_text` is text the source left on a section row; `definitions_moved` means definitions were taken back
  from the section row;
- `silver`: older model labels on the member rows. These are **evidence, not truth**: they predate sentence units
  and v2;
- `row_decisions`: Jason's row-level decisions on member rows. Strong precedent for `stem_reviewed` sentences;
  withheld for `items_only`;
- `precedents`: the 5 nearest **reviewed sentences**, with their final labels, Jason's comments, and why each was
  retrieved (pattern, same section, similarity).

## The unit

Label each sentence **once**: one `relation`, one `raw_type`, one `purpose`, and every actor named anywhere in it (lead-in, items or closing words). Items are never labelled separately.

## The tests that decide most sentences (catalogue v2 has the rest)

- **Relation** (REL-01): `yes` only if the sentence itself creates an Obligation or a Liberty.
  - A named party as the **subject** of "shall/must/may" puts its own duty or power on that party, so `yes` (REL-33), even when the duty is procedural, qualified or a way of discharging another duty.
  - With no named subject, a sentence that only sets the content, manner, timing or conditions of a duty or power in **another** sentence is `continues` (REL-28). Examples: "may be served by post", "The scheme must include—". A `continues` sentence has no holder and no `raw_type`. If the holder of the continued duty is named, it is `mentioned`.
  - Machinery is `no` (REL-07): application, extent, savings, citation, commencement, parliamentary procedure.
  - Definitions and deeming are `no` (REL-06), and they beat a passive "shall" ("shall be treated as"). Deeming that moves who holds duties gets the `deemed-holder` pattern.
  - Designating an enforcing authority is that body's Obligation, with purpose Requirements (REL-27).
  - Offences, penalties and defences are `no` (REL-04). An exemption is `no` (REL-08).
- **Actors** (POS-15, revised): one entry per label **per role**.
  - One party with a duty and a power is one entry with `"holds": "both"`.
  - One label for parties in **different** roles: list the label twice, each with its own `position`.
  - Parties sharing a label and a role are one entry. A `mentioned` role next to a substantive role is dropped.
  - An implied access right (precedent `implied-access-right`): the party is `active` with an inferred Liberty **and** has a second entry as `counterparty` of the government's duty, with act `["give_access"]`.
  - `act` is a **list**: `[]` when none, and only for the counterparty of an Obligation (ACT-01). Use the ACT-12 classes.
- **Holders from elsewhere** (HOLD-05, INF-03, the context principles): a holder supplied by an applying provision or by the parent Act is `active` with `inferred: true`. Never guess a holder (HOLD-15).
- **Purpose:** the sentence's own class, or `inherit` when its words carry none. A power to make further law is **Subordinate legislation**, not Permissions.
- **Exclusions:** amending text (words inserted into, substituted in or repealed from other laws) and text outside EHS&HR are not labelled. Emit an exclusion instead (see Output).

## Precedents

Precedents are examples, not rules.
- Read them before deciding.
- If a precedent decides your sentence word for word, follow it and say so in the reason: "as precedent `<unit_id>`".
- If you depart from a precedent, say why.
- Never put a precedent in `rule_ids`, except a catalogue precedent **pattern**, cited as `PREC:<pattern>` (e.g. `PREC:deemed-holder`).

## Difficulty (per field)

- **`easy`**: a principle decides it on the words, and nothing in the evidence or the precedents disagrees, or a precedent decides it word for word.
- **`hard`**: a principle applies but needs judgment. Examples: two principles pull different ways (name both); the evidence or a precedent disagrees with you; you are unsure.
- **`new_edge`**: no principle fits. Set `rule_ids: ["NEW"]` and describe the candidate pattern in a note.

A confident wrong `easy` costs Jason more than an honest `hard`.

## Notes: report, don't legislate

Use `notes` (one sentence each) for:
- **`candidate_pattern`**: a recurring shape no principle covers. Name it in kebab-case. A pattern becomes a principle only after about 3 provisions from at least 2 laws, and only when Jason approves.
- **`dictionary_candidate`**: a party a generic label hides, quoting the law's own term. The dictionary adds a label only when it is used across 2 or more Families, or repeatedly within one Family.
- **`source_fault`**: garbled or misplaced text in the sentence.
- **`question`**: something Jason should rule on.

## Output

One JSON line per sentence:

```json
{"unit_id": "...", "fields": [
  {"field": "relation", "proposed": "yes", "rule_ids": ["REL-01", "REL-33"], "reason": "…", "difficulty": "easy"},
  {"field": "raw_type", "proposed": null, "rule_ids": ["TYPE-04"], "reason": "…", "difficulty": "easy"},
  {"field": "purpose", "proposed": "Requirements", "rule_ids": ["P:Requirements"], "reason": "…", "difficulty": "easy"},
  {"field": "actor", "actor_label": "Gvt: Agency: Natural Resources Body for Wales",
   "proposed": {"position": "active", "holds": "both", "inferred": false, "act": []},
   "rule_ids": ["HOLD-01", "POS-15"], "reason": "…", "difficulty": "easy"}
 ],
 "notes": [{"kind": "candidate_pattern", "note": "…"}]}
```

For an excluded sentence:

```json
{"unit_id": "...", "exclude": "amending text"}
```

- **`reason`**: one plain-English sentence that quotes the deciding words.
- List every actor that should appear, including `mentioned` ones. Things, hazards and places are never actors (LBL-03).

## Rules of engagement

Read only:
- never write to the database;
- never edit docs, the dictionary or scripts;
- no paid APIs.

Report rule problems in `notes`. Don't fix them.
