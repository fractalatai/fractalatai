# Purpose Classification (PROPOSED, 2026-10-01)

**Status:** proposed by fractalaw. Gemini and sertantai-legal reviewed it (2026-10-01; adopted changes below). Waiting for Jason's sign-off, and his decision on the law-level `purpose` (raised by legal).
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
| 2 | `Interpretation+Definition` | machinery | **Constitutive effect:** definitions, "references to", deeming and legal fictions ("shall be treated as", "is deemed"), evidential effect ("a certificate … shall be conclusive evidence"), and status by operation of law ("an approval remains in force for…", "the licence shall cease to have effect") | keep; widened (#40, Gemini) |
| 3 | `Application+Scope` | machinery | What or whom the law applies to; "this Part applies to…"; "shall apply to X as it applies to Y" | keep |
| 4 | `Exemption` | machinery | Takes a party or case out of scope: "shall not apply to…", "nothing in this section requires…", certificates of exemption. A scope clause, not a Liberty or Immunity (spec layer 1) | keep; definition sharpened |
| 5 | `Extent` | machinery | Territorial extent | keep |
| 6 | `Establishment+Constitution` | machinery | Bodies: creation ("There shall be a body corporate…"), constitution, membership, proceedings, staffing, and **statements of their objectives or general functions** ("the principal objective of the Regulator is…") | **new** (#40, Gemini) |
| 7 | `Amendment` | machinery | Amends another instrument | keep |
| 8 | `Repeal+Revocation` | machinery | Repeals or revokes | keep |
| 9 | `Transitional Arrangement` | machinery | Transitional and time-based saving provisions | keep; time-based savings added |
| 10 | `Requirement` | operative | Creates a duty or prohibition, for anyone (governed or government) | **new**: the real half of the catch-all |
| 11 | `Power Conferred` | operative | Creates a power, permission or entitlement (any "may" relation, including governed rights) | keep; **widened** to every Liberty |
| 12 | `Procedure+Detail` | operative | Form, manner, timing, conditions or procedure of a relation **created in another provision**; notice service; parliamentary procedure (the 2026-10-01 detail-provisions ruling) | **new**: the other half of the catch-all |
| 13 | `Charge+Fee` | operative | Fees, charges and payments | keep |
| 14 | `Enforcement+Prosecution` | sanctions | Enforcement bodies' powers and notices, proceedings, prosecution | keep |
| 15 | `Offence` | sanctions | Creates an offence, **including penalties** ("liable on conviction to…", fixed penalties) | keep; absorbs penalty "liable" (#47) |
| 16 | `Defence+Appeal` | sanctions | Statutory defences, appeals, reviews | keep |
| 17 | `Liability` | sanctions | **Civil** liability and compensation, including "shall not be liable" | keep; **narrowed** |
| — | `Unclassified` | — | Only rows with no readable text | target ~0 for substantive rows |
| — | ~~`Process+Rule+Constraint+Condition`~~ | — | Split into `Requirement` and `Procedure+Detail` | **retired** |

**The Requirement / Procedure+Detail test** (the boundary most likely to make labelling unreliable, Gemini): *does this provision create its own duty, or qualify one created elsewhere?*
- **Qualifies one created elsewhere** (it refers to it: "the application under section 10", "the notice referred to in paragraph (1)", "the record required by regulation 5") → `Procedure+Detail`, relation `none`. Example: "An application under section 10 must be made in the prescribed form and be accompanied by the fee."
- **Creates its own duty**, even a procedural one → `Requirement`. Example: "The Executive must consult the Secretary of State before issuing an approved code of practice" (no other provision imposes the consultation).
- A duty with a qualifier stays `Requirement`. HSWA s.2(3), "Except in such cases as may be prescribed, it shall be the duty of every employer to prepare … a written statement", is a duty, not an `Exemption`.

**Boundary cases** (legal, 2026-10-01):
- **Technical schedules, tables of values, forms and standards** referenced by a duty (exposure limits such as COSHH WELs, specified limits, prescribed forms) are `Procedure+Detail`: they set the content of a duty created elsewhere. For compliance they're the substance, so controls attach them to the duty they qualify (see Contract). Schedules aren't in fractalaw's parse scope today.
- **General non-prejudice savings** ("Nothing in these Regulations shall prejudice any other enactment", "nothing in this section affects any liability…") → `Application+Scope`. They're neither time-based (`Transitional Arrangement`) nor an `Exemption`.
- **Crown application:** "This Act binds the Crown" → `Application+Scope`; "nothing in this section makes the Crown criminally liable" → `Exemption`.
- **Review clauses:** "The Secretary of State must review these Regulations and publish a report" → `Requirement` (a government duty, so a Responsibility). This is intended. It raises Responsibility counts on many post-2012 SIs.

**Choosing one purpose.** Ask what the provision *does* in its own text, read with its stem:
- **Precedence** when two fit: machinery (1–9) > sanctions (14–17) > `Charge+Fee` > `Power Conferred` / `Requirement` > `Procedure+Detail`. **Within each family, table order decides**, so "may be cited as … comes into force … extends to" is `Enactment+Citation+Commencement`. A definition containing "shall" is a definition; an offence provision is an offence even though it implies a duty elsewhere. Between `Requirement` and `Power Conferred`, the provision's main relation decides.
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

- **sertantai-legal, provision level** (`legal_articles.purposes`): stored as received. Nothing in legal or compliance reads it (legal, 2026-10-01), so the value changes and single values are safe, and legal needs no code change.
  - Laws not republished in the single run keep the old values, so the store mixes the two vocabularies until every law is re-enriched. The final dry run counts how many laws still carry old labels.
  - Secondary sources (`secondary_source_provisions.purposes`, from the JSP pipeline) aren't in the single run. They keep the old labels until they're next enriched.
- **Law level: a purpose profile, published by fractalaw** (Jason, 2026-10-01; sertantai-legal#172).
  - What kind of law it is (making, amending, commencing…) is legal's **Function**; purpose doesn't repeat it.
  - Legal's `legal_register.purpose` today runs the regex over the whole law's text, so most laws get nearly every label.
  - Instead, fractalaw publishes `purpose_profile`: each purpose's count and share of the law's own provisions, e.g. `[{purpose: "Requirement", count: 132, share: 0.55}, …]`. Amendment instructions count; inserted text is excluded.
  - Legal stores it, derives its multi-select from it, and retires its law-level classifier. Shape, base (live or whole law) and fallback are agreed on #172.
- **Compliance controls** (`scripts/compliance/generate_controls.py`, fractalaw's, so legal and compliance need no change): select `Requirement` positively instead of excluding purposes, and **attach the `Procedure+Detail` provisions that qualify it** (same law, referring to it) as prompt context. "The record required by regulation 5 must contain…" and a table of limits are what a control needs, so they mustn't be dropped (legal).
- **Fractalaw:** `purpose.rs` labels, `STRUCTURAL_PURPOSES`, `making.rs` counts, and the `classify_title` mapping.
- **Data:** purposes are re-derived in the single run. No separate pass.

## Review decisions (2026-10-01)

**Gemini** (2.5 Pro). Adopted:
- **Gap:** evidential and status provisions → `Interpretation+Definition`, widened to constitutive effect.
- **Gap:** objectives and general functions of bodies → `Establishment+Constitution`.
- **Requirement / Procedure+Detail:** the explicit test above.
- **Open questions 2–4:** keep `Liability` and `Defence+Appeal` separate; keep `Establishment+Constitution`; do a real impact analysis of consumers, not a notice.

Not adopted:
- **"The DRRP consistency check is fundamentally flawed."** That critique reads `Procedure+Detail` as any procedural "must". Under the detail ruling it covers only provisions qualifying a relation created elsewhere, which are relation `none` by definition. A procedural duty created in its own text is `Requirement`. With the test above the check holds.
- **Primary + secondary purpose.** Its counter-examples resolve with a single label: HSWA s.2(3) is a qualified duty, and a body "which shall exercise the functions conferred" is `Establishment` (the functions are created elsewhere). A secondary label brings back the ambiguity this proposal removes, and the operative detail is already in DRRP. **For Jason to confirm.**

**sertantai-legal.** Agrees with the value changes and with one value per provision. Keep Liability and Defence+Appeal separate ("shall not be liable" protects; a defence answers a charge); keep Establishment. Adopted:
- technical schedules and tables → `Procedure+Detail`, attached to their duty in controls;
- non-prejudice savings → `Application+Scope`;
- ties within machinery decided by table order;
- Crown and review-clause examples;
- the mixed-vocabulary count in the final dry run.

## Open questions for reviewers (answered)

1. Is single-label right, or do some provisions need two purposes (e.g. "cited as … and comes into force")?
2. Should `Liability` and `Defence+Appeal` merge, now that "shall not be liable" sits with civil liability?
3. Is `Establishment+Constitution` worth its own value, or should it be machinery under `Application+Scope`?
4. Legal: does anything in legal or sertantai (UI, filters, LRT) read the old values, especially `Process+Rule+Constraint+Condition`?
