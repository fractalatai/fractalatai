# Justifier brief: gold v3 labels with rule citations (phase 0a)

> **Superseded 2026-10-07** by `JUSTIFY_V2.md` (sentence units on catalogue v2). Kept as the phase 0a/A record.

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
3. `docs/architecture/PURPOSE-CLASSIFICATION.md` § "Layered purpose": the **11 coarse classes**.
4. Actor labels only from `crates/fractalaw-core/data/actor-dictionary.yaml`, or `OTHER: <description>` if none fits (that's a `new_edge`).

**Where the spec now differs from the v1.3 prompt the models used** (follow the spec, and expect the evidence to disagree):
- **Functions lists** (REL-44): relation `yes`. A government body's functions are an Obligation; a governed party's are a Liberty. Provisions that only mention functions are `no`.
- **An applying provision's own row** (REL-26): "X shall comply with / ensure compliance with the requirements of SCOPE" is a pointer, so relation `no`, with X `mentioned`. The supervisory form (HOLD-07) and the prohibition form (HOLD-09) are still duties.
- **`serve` is folded into `notify`** (ACT-03, ACT-12). The acts are: `notify`, `supply`, `consult`, `pay`, `give_access`, `charge`, `answer_request`, `other`. Map the verb using the synonym table (spec layer 1b).
- **#67 access rights** (INF-01) also apply when the access duty's holder is unknown ("open to inspection by the public" → the public is `active`, holds `Liberty`, `inferred: true`).
- **POS-15 revised 2026-10-07 (#78):** one entry per label per role, `holds: both` for one party with a duty and a power, `act` as a list. See `JUSTIFY_UNITS.md`.
- **Stems and list items are counted once** (REL-45, REL-28, POS-17; Jason 2026-10-06). A stem that begins a duty or power ("X shall—", "X may … requiring the person to—") is `yes` with its holder active. An item that only completes it is relation **`continues`**: no holder, no raw_type, and the stem's holder is **not** listed. The item's own actors keep their role in the stem's relation (counterparty with its act, beneficiary, or mentioned). An item with its own duty or power is `yes`; criterion and class-definition items stay `no` (REL-15). The source puts a stem's closing words ("… as it may reasonably require …") on the stem row; they belong to the stem.
- **Detail of a duty continues it** (REL-28, 2026-10-06): a provision that only sets the content, form, manner, timing or discharge of a duty created elsewhere is `continues`, not `no` (it replaces the old `no` for REL-11/12/13 and the detail half of REL-16). `no` means the provision touches no duty or power at all. Conditions on **a power** (REL-14) stay `no`.
- **"No person shall be engaged/employed/permitted"** (HOLD-17): the engager holds it, from the applying provision (`inferred: true`); the person engaged is `mentioned`.
- **Participation** (TYPE-07): "shall take part in … or shall be consulted" is a Liberty for the participants.
- **Purpose:** Requirements is for provisions about obligations. A condition on an exemption, or on how a power applies, is Application and exemption. Choose one class (the dominant one).
- **Conditions on a power continue it** (REL-14 → `continues`, 2026-10-06): "shall not grant … unless satisfied", "a notice may not be given—".
- **Access duties** (POS-18): on a `continues` row of an access duty, the party given access is `counterparty`, act `give_access`.
- **Purpose:** Transitional is merged into **Application, exemption and transition** (11 classes).
- **Beneficiary test** (POS-19): decide beneficiary vs mentioned from the words. If the party is named as what is protected ("health and safety **of** X", "protect X", "the interests of X"), it's a beneficiary, even inside a condition on a power. If the words only set when or whether the rule applies ("where X…", "if X…", "having regard to X"), it's mentioned. A recipient of the act is a counterparty.
- **Engagers** (HOLD-17, refined 10-06): every applying holder who can engage someone holds "No person shall be engaged…": the employer **and** the self-employed person; the employee isn't listed.
- **Item actors** (POS-17, batch 1): a `continues` item lists **only actors named in its own text**. Actors named only in the stem (the person served, the deciding body, the protected party) are not repeated on the item.
- **Stems extending another provision's power** (REL-45, batch 1): "Regulations under subsection (1) may—" `continues` the power conferred in (1); no holder.
- **Permitted discharge** (REL-46, batch 1): "X may inform … by a general announcement" is `yes`, X active Liberty, purpose Permissions.
- **Purpose, batch 1:** parliamentary procedure (annulment clauses) is Requirements. Class-definition/criterion items and conditions on a power stay Application.
- **Deeming clauses** (LBL-03): a body named only there is still listed, `mentioned`.
- **New dictionary labels:** `Gvt: Agency: Civil Aviation Authority`, `Gvt: Registrar General`.
- **Hazards aren't actors** (LBL-14, 10-06): biological agents, substances and micro-organisms are never listed.
- **Purpose, 10-06:** Interpretation is for meanings only; "the application of" a power or rule is Application. "S is the regulator of B from <date>" is Application (transition), not Constitution. Detail of where or how a register is kept `continues` the keeping duty (REL-28).
- **Purpose `inherit`: the escape rule** (Jason, 2026-10-06, batch 2). Purpose was being forced onto snippets; laws give purpose to whole sections. So a provision takes its **own** purpose only when its own words carry one: it creates or confers something, defines a term, applies or disapplies the law, sets an offence, penalty, fee, appeal, amendment or citation. A snippet that only makes sense with the provision above it takes **`inherit`**: its purpose is that of the nearest ancestor (stem, then subsection, then section). Snippets include list items completing a stem, conditions and provisos ("if the person under restraint agrees…"), criterion items, and fragments that set the detail of a duty in the same section. Prefer `inherit` to forcing a class onto a snippet. A provision with no ancestor never inherits. Propose `"inherit"` with rule id `P:inherit`; the reason names the ancestor whose purpose it takes. Grade it easy when the provision is plainly a snippet.
- **Purpose, batch 2:** who/when/what/where is Application; a procedural condition on exercising a power is Requirements (SSI 2018/219 reg.27(6)); a condition on an exemption or on whom a power reaches stays Application. A power to authorise others to act for a body is Permissions. "An offence will not be committed … provided that" is Offences and penalties.
- **Item with its own laying duty** (REL-35, batch 2): "if prepared by the Secretary of State, be laid before Parliament" is `yes`, Secretary of State active Obligation. An item that only selects who holds the stem's duty leaves that party `mentioned`.
- **Duties for existing cases** (batch 2): list the holder as active even when the arrangements are defined in another paragraph (reg.18(5) well-operator).
- **Cross-instrument holders** (HOLD-14): the holder comes from the parent Act when you can resolve it from the text given. Otherwise it is holder unknown.

## Fields to propose (per provision)

| field | value | rule IDs |
|---|---|---|
| `relation` | `"yes"` / `"no"` / `"continues"` (an item completing its stem's relation, REL-28) | REL-* |
| `raw_type` | `"Obligation"` / `"Liberty"` / `null`; set only when relation is yes and no actor is active | HOLD-15, DEF-02, TYPE-* |
| `purpose` | one of the 11 coarse classes, exactly as written in PURPOSE-CLASSIFICATION.md, or `"inherit"` for a snippet (escape rule) | `P:<class>` plus the deciding cue, e.g. `P:Interpretation` |
| `actor` (one per label per role) | `{"position", "holds" (Obligation, Liberty, both, none), "inferred", "act": [..]}` | POS-*, HOLD-*, TYPE-*, INF-*, ACT-*, LBL-* |

Purpose detail (Substantive vs Procedure/Detail under Requirements) is **not** proposed: it is derived from purpose plus relation when the gold set is frozen (Jason, 2026-10-06). Purpose is always proposed, including for relation `no` rows. There are no purpose rule IDs yet (the purpose catalogue comes with phase 2), so cite `P:<class>` and quote the deciding words in the reason.

## Difficulty (per field)

- **`easy`**: one rule clearly applies, and the evidence that exists (models, referee, pipeline) agrees with you, or there is none and the text is plain.
- **`hard`**: the evidence disagrees with you, two rules pull different ways (name both), or you are unsure.
- **`new_edge`**: no rule in the catalogue fits. Set `rule_ids: ["NEW"]` and write the candidate rule in the reason, in one line.

Be honest: a confident wrong `easy` costs Jason more than a `hard`. Evidence that predates a ruling (the models and referee followed `drrp-v1.3`) doesn't make a row hard when the ruling covers the case word for word: grade it easy and name the ruling.

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
