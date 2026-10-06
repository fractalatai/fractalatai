# Purpose Classification (AGREED, 2026-10-01)

**Status:** agreed. Jason signed it off on 2026-10-01, single label included, after reviews by Gemini and sertantai-legal (adopted changes below). The law-level profile is agreed on sertantai-legal#172 and built.
**Fixes:** #40 (Process+Rule too broad), #47 (Offence/Enforcement coverage). **Supersedes:** #69 (provision function).

**Revision in progress (2026-10-06, phase 0b):** a layered scheme replaces the single 18-value label (section below). The 18 values stay the published field until the migration with legal. Legal reports that #172's profile isn't built yet, so nothing depends on the 18 values.

## Layered purpose (decided 2026-10-06)

Jason (2026-10-06): use more than one method: a coarse, simple layer that feeds a finer, more complex one only when needed. The class names are the terms legislation itself uses in its headings, not invented categories.

**Coarse layer: 11 classes.** Class names avoid the DRRP vocabulary (Obligation, Liberty, Duty, Right, Responsibility, Power): `Duties` → **Requirements** and `Powers` → **Permissions** (Jason, 2026-10-06), and the fine split is **Substantive** vs Procedure/Detail.

| Class | Replaces (of the 18) |
|---|---|
| Citation and commencement | Enactment+Citation+Commencement, Extent |
| Interpretation | Interpretation+Definition |
| Application, exemption and transition | Application+Scope, Exemption, Transitional Arrangement |
| Requirements | Requirement, Procedure+Detail |
| Permissions | Power Conferred |
| Enforcement | Enforcement+Prosecution (its own class, not split under Permissions: compliance users look for it directly) |
| Offences and penalties | Offence, Liability |
| Appeals and defences | Defence+Appeal |
| Fees and charges | Charge+Fee |
| Amendment and revocation | Amendment, Repeal+Revocation |
| Constitution | Establishment+Constitution ("there shall be a body called X" is neither a duty nor a power) |

**Class rules from the gold review** (Jason, 2026-10-06):
- **Requirements** is for provisions about obligations. A condition on an exemption, or on how a power applies ("shall not grant any such exemption unless satisfied", "application of the exercise of a power"), is **Application and exemption**.
- **Transitional is merged into Application** (Jason, 2026-10-06): transition is application in time, as scope is application to persons, things and places. So the coarse class is **Application, exemption and transition**, with a detail level **Scope / Exemption / Transitional and saving**. The detail is filled only when a consumer needs it, and isn't labelled in the gold set.
- **Interpretation is for meanings only** (Jason, 2026-10-06): a provision on "the application of" a power or rule (how, when or to whom it applies) is **Application**, even under an interpretation heading (Companies Act 1989 s.112(2)(b)).
- **Who is the regulator from a date** ("S is the regulator of B from 1st January 2026") is **Application** (transition in time), not Constitution (GHG ETS 2020 reg.13(2)). Constitution is for setting up a body and its make-up.
- **Parliamentary procedure** (annulment and affirmative clauses) is **Requirements**: the procedure is the "how" (Jason, batch 1). Application says when, who, where and what.
- **A permitted way of discharging a duty** ("the hirer may inform … by a general announcement") is **Permissions**.
- Class-definition and criterion items **stay Application** (confirmed in batch 1, GHG ETS reg.70(2)(db)); conditions and limits on a power stay Application (reg.44A(8)(a)).
- **A condition on an exemption power** is Application (gas regs 1998 reg.40(2)).
- Purpose stays **single-select**: one class per provision, the dominant one. That's more useful as a data model than multi-select, even where a provision mixes application and transition.

**How the coarse layer decides:**
- high-precision text cues;
- a list item inherits its stem's class;
- section titles once legal serves them, then cross-headings;
- otherwise `undetermined`, which escalates to the fine layer.

**Fine layer:** classifier or LLM, only where a consumer needs the split. The first is **Requirements → Substantive vs Procedure/Detail**, which feeds the DRRP gate.

**Evidence:** first cue pass (phase 0b session, 2026-10-06), on 6,959 labelled provisions. Precision when a cue fires:

| Class | Precision |
|---|---|
| Offences | 97% |
| Interpretation | 93% |
| Appeals | 100% |
| Fees | 86% |
| Amendment | 82% |
| Requirements | 81% |
| Citation | 72% |

Recall is low until stem inheritance is added. Permissions vs Enforcement needs enforcement cues.

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
- Machinery and `Procedure+Detail` ⇒ relation `none`, **except** `Transitional Arrangement` where the provision itself confers a time-limited power or imposes a time-limited duty, `Enactment+Citation+Commencement` where it confers a power to commence, and `Exemption` where it confers a power to exempt: relation exists (Jason, 2026-10-05; DRRP-CLASSIFICATION.md special cases).
- `Enforcement+Prosecution`, `Charge+Fee` and sanctions may go either way: an inspector's power is a Power, while an offence creates no relation.
- A mismatch is a labelling error in one of the two fields.

**Structural scope** (`provision_scope`): machinery (1–9) and `Procedure+Detail` are structural, so actors default to `mentioned`. The existing override stays: a structural purpose with a DRRP modal is promoted to substantive.

## Sources and precedence

- **Labels:** the definitive prompt (#72 model completion) labels purpose per provision beside the actors. The same prompt feeds gold v2 (two models + Claude referee, used as the benchmark) and the SLM training set (one model).
- **SLM:** trained on both tasks: actor position/type and provision purpose.
- **Regex (`purpose.rs`)** stays as pass 1 at parse time. **Built (2026-10-01, `9aa7f57`):** `purpose::primary` with the stem rule writes the published purpose.
  - The original multi-match patterns stay as **internal signals** for DRRP gating (`should_skip_drrp`) and scope (`provision_scope`), unchanged, so DRRP parsing doesn't drift. Moving gating and scope onto the published purpose (machinery and `Procedure+Detail` structural, as above) is a later change, measured against gold v2.
  - **Survey of 244,624 hub rows:** the catch-all (98,783) and multi-label rows (18,771) are gone, and the stem rule resolves 74,452 items. Unclassified falls from 96K to 54.7K. Of those, 52.5K are list items under condition or definition stems the regex can't classify; the LLM/SLM tiers handle them.
- **Reconcile:** adjudicated > LLM > SLM > regex, as for DRRP. Scope is re-evaluated after reconcile.

## Contract and migration

- **sertantai-legal, provision level** (`legal_articles.purposes`): stored as received. Nothing in legal or compliance reads it (legal, 2026-10-01), so the value changes and single values are safe, and legal needs no code change.
  - Laws not republished in the single run keep the old values, so the store mixes the two vocabularies until every law is re-enriched. The final dry run counts how many laws still carry old labels.
  - Secondary sources (`secondary_source_provisions.purposes`, from the JSP pipeline) aren't in the single run. They keep the old labels until they're next enriched.
- **Law level: a purpose profile, published by fractalaw** (Jason, 2026-10-01; sertantai-legal#172).
  - What kind of law it is (making, amending, commencing…) is legal's **Function**; purpose doesn't repeat it.
  - Legal's `legal_register.purpose` today runs the regex over the whole law's text, so most laws get nearly every label.
  - Instead, fractalaw publishes `purpose_profile`: each purpose's count and share of the law's own provisions, e.g. `[{purpose: "Requirement", count: 132, share: 0.55}, …]`. Amendment instructions count; inserted text is excluded.
  - Legal stores it, derives its multi-select from it, and retires its law-level classifier. **Agreed on #172 (Jason and legal, 2026-10-01); built:**
  - Shape: `purpose_profile: [{purpose, count, share}]`, sorted by count. `[]` when empty; NULL or absent = not in this payload.
  - Base: the **whole law**, the same view as the other law fields, so revoked laws keep a profile. A `current_purpose_profile` can come later if needed.
  - Legal stores it in a new column and derives `purpose.values` from it (share ≥ 0.05, never `Unclassified`). The column keeps its `{values}` shape, so compliance needs no change.
  - Fallback: unprofiled laws keep legal's current values, and legal retires its TaxaParser classifier. Fractalaw leaves the profile NULL for laws still on the old vocabulary.
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
- **Primary + secondary purpose.** Its counter-examples resolve with a single label: HSWA s.2(3) is a qualified duty, and a body "which shall exercise the functions conferred" is `Establishment` (the functions are created elsewhere). A secondary label brings back the ambiguity this proposal removes, and the operative detail is already in DRRP. **Jason: single label (2026-10-01).**

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
