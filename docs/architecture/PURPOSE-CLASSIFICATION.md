# Purpose Classification (PROPOSED, 2026-10-01)

**Status:** proposed by fractalaw; reviews pending (sertantai-legal, Gemini), then Jason.
**Fixes:** #40 (Process+Rule too broad), #47 (Offence/Enforcement coverage). **Supersedes:** #69 (provision function).

## What purpose is

`purposes` says **what a provision does**: its function in the law. It is orthogonal to DRRP, which says **who holds what**.
- An inspector's power to enter premises: DRRP is Power; purpose is `Enforcement+Prosecution`.
- A charging power: DRRP is Power; purpose is `Charge+Fee`.
- A definition: DRRP `none`; purpose `Interpretation+Definition`.

Every in-scope provision gets a purpose, including those DRRP leaves `none`. That gives QA the distinction it lacks today: "`none` because it's a definition" vs "`none` but it reads as a duty".

## What's wrong today (`fractalaw-core/src/taxa/purpose.rs`, regex, ported from sertantai's Taxa.PurposeClassifier)

| Problem | Evidence (hub, 360K rows) |
|---|---|
| **The catch-all isn't a purpose.** `Process+Rule+Constraint+Condition` matches any modal or keyword (shall, must, required, conditions, ensure, maintain…). It holds real duties *and* deeming, form/manner details, notice service, scope clauses and establishment (#40). | 115,623 provisions |
| **Labels pile up.** Each regex fires independently, so a provision collects every match, e.g. `Interpretation+Definition` and `Process+Rule` together. | 18,810 provisions with 2–7 labels |
| **Unclassified is mostly real text.** List items whose purpose sits in the stem, conditions, and "may" powers (Power Conferred only matches "functions… exercisable", "power to make regulations"). | 96,409, of which 63,753 are long substantive rows; 11,920 carry Liberty |
| **"Liable" means three things.** A penalty ("liable on conviction"), civil liability ("liable for the damage") and protection ("shall not be liable") all become `Liability`. Offences are under-tagged: HSWA s.33 is `Process+Rule` only (#47). | 4,032 Liability |

Purpose isn't cosmetic. `provision_scope` uses it to decide **structural vs substantive** (structural provisions default their actors to `mentioned`), the compliance controls generator excludes provisions by purpose, and it goes to legal in the provision payload.

## Proposed classification

**One purpose per provision.** The field stays `purposes TEXT[]` for compatibility, but holds one value. Multi-label is the main noise source, and a single choice is what a labeller and the SLM can be trained and measured on.

| # | Purpose | Family | Covers | Change |
|---|---|---|---|---|
| 1 | `Enactment+Citation+Commencement` | machinery | Title, citation, commencement dates, enacting formula | keep |
| 2 | `Interpretation+Definition` | machinery | Definitions, "references to", **deeming and legal fictions** ("shall be treated as", "is deemed") | keep; deeming added (#40) |
| 3 | `Application+Scope` | machinery | What or whom the law applies to; "this Part applies to…"; "shall apply to X as it applies to Y" | keep |
| 4 | `Exemption` | machinery | Takes a party or case out of scope: "shall not apply to…", "nothing in this section requires…", certificates of exemption. A scope clause, not a Liberty or Immunity (spec layer 1) | keep; definition sharpened |
| 5 | `Extent` | machinery | Territorial extent | keep |
| 6 | `Establishment+Constitution` | machinery | Creating and constituting bodies: "There shall be a body corporate…", membership, proceedings, staffing | **new** (#40) |
| 7 | `Amendment` | machinery | Amends another instrument | keep |
| 8 | `Repeal+Revocation` | machinery | Repeals or revokes | keep |
| 9 | `Transitional Arrangement` | machinery | Transitional and saving provisions | keep; savings added |
| 10 | `Requirement` | operative | Creates a duty or prohibition, for anyone (governed or government) | **new**: the real half of the catch-all |
| 11 | `Power Conferred` | operative | Creates a power, permission or entitlement (any "may" relation, including governed rights) | keep; **widened** to every Liberty |
| 12 | `Procedure+Detail` | operative | Form, manner, timing, conditions or procedure of a relation created elsewhere; notice service; parliamentary procedure (the 2026-10-01 detail-provisions ruling) | **new**: the other half of the catch-all |
| 13 | `Charge+Fee` | operative | Fees, charges and payments | keep |
| 14 | `Enforcement+Prosecution` | sanctions | Enforcement bodies' powers and notices, proceedings, prosecution | keep |
| 15 | `Offence` | sanctions | Creates an offence, **including penalties** ("liable on conviction to…", fixed penalties) | keep; absorbs penalty "liable" (#47) |
| 16 | `Defence+Appeal` | sanctions | Statutory defences, appeals, reviews | keep |
| 17 | `Liability` | sanctions | **Civil** liability and compensation, including "shall not be liable" | keep; **narrowed** |
| — | `Unclassified` | — | Only rows with no readable text | target ~0 for substantive rows |
| — | ~~`Process+Rule+Constraint+Condition`~~ | — | Split into `Requirement` and `Procedure+Detail` | **retired** |

**Choosing one purpose.** Ask what the provision *does* in its own text, read with its stem:
- **Precedence** when two fit: machinery (1–9) > sanctions (14–17) > `Charge+Fee` > `Power Conferred` / `Requirement` > `Procedure+Detail`. A definition containing "shall" is a definition; an offence provision is an offence even though it implies a duty elsewhere. Between `Requirement` and `Power Conferred`, the provision's main relation decides.
- **List items and fragments take their stem's purpose** ("A scheme must— (a) …" items are `Requirement`), unless the item does something else itself, e.g. an exemption or definition inside a list. This is the same stem rule as DRRP, and it should clear most of today's Unclassified rows.
- **Government deadline exception** (detail ruling): "The first regulations must come into force no later than…" is `Requirement`.

**Consistency with DRRP** (a QA check, not a derivation):
- `Requirement` ⇒ a relation exists with Obligation.
- `Power Conferred` ⇒ a relation exists with Liberty.
- Machinery and `Procedure+Detail` ⇒ relation `none`.
- `Enforcement+Prosecution`, `Charge+Fee` and sanctions may go either way: an inspector's power is a Power, while an offence creates no relation.
- A mismatch is a labelling error in one of the two fields.

**Structural scope** (`provision_scope`): machinery (1–9) and `Procedure+Detail` are structural, so actors default to `mentioned`. The existing override stays: a structural purpose with a DRRP modal is promoted to substantive.

## Sources and precedence

- **Labels:** the definitive prompt (#72 model completion) labels purpose per provision beside the actors. The same prompt feeds gold v2 (two models + Claude referee, used as the benchmark) and the SLM training set (one model).
- **SLM:** trained on both tasks: actor position/type and provision purpose.
- **Regex (`purpose.rs`)** stays as pass 1 at parse time (scope needs a purpose before actor extraction). It's rewritten to the new labels with the stem rule and is measured against gold v2.
- **Reconcile:** adjudicated > LLM > SLM > regex, as for DRRP. Scope is re-evaluated after reconcile.

## Contract and migration

- **sertantai-legal:** the value set in the provision payload `purposes` changes. `Process+Rule+Constraint+Condition` disappears, three values are added, and arrays hold one value. Legal confirms how it stores, displays or filters purposes before anything ships.
- **Compliance controls** (`scripts/compliance/generate_controls.py`) excludes Offence, Exemption, Enactment and Defence+Appeal. It should select `Requirement` positively instead of excluding. Legal to relay to sertantai-compliance.
- **Fractalaw:** `purpose.rs` labels, `STRUCTURAL_PURPOSES`, `making.rs` counts, and the `classify_title` mapping.
- **Data:** purposes are re-derived in the single run. No separate pass.

## Open questions for reviewers

1. Is single-label right, or do some provisions need two purposes (e.g. "cited as … and comes into force")?
2. Should `Liability` and `Defence+Appeal` merge, now that "shall not be liable" sits with civil liability?
3. Is `Establishment+Constitution` worth its own value, or should it be machinery under `Application+Scope`?
4. Legal: does anything in legal or sertantai (UI, filters, LRT) read the old values, especially `Process+Rule+Constraint+Condition`?
