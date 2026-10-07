# Justifier brief: sentence units (meta-plan phase A, first load)

> **Superseded 2026-10-07** by `JUSTIFY_V2.md` (sentence units on catalogue v2). Kept as the phase 0a/A record.

You label **whole legal sentences**, not the source's rows. Read `JUSTIFY_GOLD.md` first for the fields, the
output format, difficulty grades and the binding documents. This brief **overrides** it where they differ.

## The unit

Each input row is one sentence (`unit_id` = its first row). `text` is the sentence as the law reads it: the stem,
its items indented, then the stem's closing words. `members` lists the source rows it was assembled from.

- Label the sentence **once**: one `relation`, one `raw_type`, one `purpose`, and **every actor named anywhere in
  the sentence** (stem, items or closing words), each with one role.
- Items are never labelled separately. The stem-and-item rules no longer apply: REL-02, REL-13, REL-15, REL-16,
  REL-30, REL-45, HOLD-02, TYPE-06, POS-17, POS-18, the item half of REL-28, and the 10-06 refinements about items
  (items picking the holder, item laying duties, stem beneficiaries on items, "continues" on items).
- Two holders in one sentence ("the operator must …, and the regulator may …") are two `active` actors, each with
  its own `holds`.
- **One entry per label per role** (POS-15, revised 2026-10-07 for #78):
  - one party with a duty **and** a power ("X may prescribe …; X must publish …") → one entry, `"holds": "both"`;
  - one label standing for parties in **different** roles ("a person who carries out a search of a relevant
    person") → one entry per role: list the label twice, each with its own `position`;
  - parties sharing a label **and** a role (two authorities, both beneficiaries) → one entry;
  - a `mentioned` role next to a substantive role for the same label is dropped (the strongest wins);
  - `act` is a **list** (`[]` when none): one party can have several acts ("workers are provided with … ; workers
    do not eat or drink …").
- If you think a generic label (`Ind: Person`, `Gvt: Authority`) hides two recurring parties that deserve their own
  labels, say so in a policy note as a **dictionary candidate**, quoting the law's own term.
- **Relation:**
  - `yes`: the sentence creates an obligation or liberty;
  - `continues`: the **whole sentence** only sets the detail, condition or timing of a duty or power created in
    **another** sentence;
  - `no`: it touches no duty or power at all.
- **Purpose:** the sentence's own class when its words carry one; `inherit` (from its section) only when the
  whole sentence has no purpose of its own.

## Provisional defaults (Jason, 2026-10-06: the answers will emerge from the data)

These are defaults, not settled rules. Apply them, and record a **policy note** whenever one feels wrong for this
sentence, or when a case isn't covered:
1. A proviso ("Provided that…", "But…") joins its sentence.
2. An item that is a full sentence with its own holder and modal stays in the unit.
3. A whole sentence that only details a duty or power in another sentence is `continues`.
4. Purpose: the sentence's own class, or `inherit` when it has none.

## Precedent

`row_decisions` are Jason's earlier decisions on rows of this sentence (row level, before sentence units). For
`kind: stem_reviewed`, treat the stem row's decisions as strong precedent and keep them unless the items change
the answer (say why). For `kind: items_only`, `row_decisions` are withheld so you label blind.

Don't make rules. If you see a pattern you think recurs, name it in a policy note as a **candidate pattern**.

## Output

One JSON line per unit:

```json
{"unit_id": "...", "fields": [ ...as JUSTIFY_GOLD.md, rule IDs from the catalogue (not the retired ones)... ],
 "policy_notes": [{"default": 1, "note": "the proviso creates its own Liberty for …"},
                  {"default": null, "note": "candidate pattern: …"}]}
```

`policy_notes` may be empty. Keep each note to one sentence.
