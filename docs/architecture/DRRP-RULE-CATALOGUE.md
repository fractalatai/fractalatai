# DRRP Rule Catalogue

Stable rule IDs for the gold set (phase 0a). Every gold label (relation, raw_type, and per actor: label, position, holds, inferred, act) cites one or more IDs from this page plus a one-line reason.

- **Status:** draft 2026-10-06, built from `DRRP-CLASSIFICATION.md` (spec), `SYSTEM_PROMPT` in `scripts/drrp_prompt.py` (`drrp-v1.4-2026-10-06`) and `scripts/ml/REFEREE_TRAINING_LABELS.md` (referee brief). It restates them; it does not change them. When a ruling changes a rule, the spec changes first, then the prompt, then this page.
- **Scope:** the core gold fields only. Purpose rules wait for the purpose refactor (phase 0b); where a rule decides relation/actors *and* purpose, only the relation/actor half is catalogued here and the purpose half is listed in Appendix A. Derived schemes (DRRP type, holder class, correlatives, verdict) are not labelled, so they have no rules here.
- **IDs are stable.** Never renumber. A retired rule keeps its row, struck through, with the ruling that retired it. New rules take the next free number in their group.

**Source key:** `S:L1`…`S:L5` = spec layer; `S:SC <row>` = spec special-cases row; `S:WE` = spec worked examples; `P:<section>/<bullet>` = prompt; `R` = referee brief "decisive points"; `D` = actor dictionary (`crates/fractalaw-core/data/actor-dictionary.yaml`). Dates are rulings (Jason unless stated). **P-only** / **S-only** mark rules found in only one of spec and prompt.

## Decision path

1. **REL**: does this provision, in its own text read with its stem, create an Obligation or Liberty? (relation yes/no)
2. **HOLD**: if yes, who holds it? Own text → stem → referenced provision → applying provision → unknown.
3. **TYPE**: what does each actor hold (Obligation / Liberty / none), and is `raw_type` needed?
4. **INF**: is a holder inferred (#67 access right, #60 applying provision)?
5. **POS**: what position do the other actors take (counterparty / beneficiary / mentioned)?
6. **ACT**: what is each Obligation counterparty owed?
7. **LBL**: which dictionary label does each actor get, and is it an actor at all?
8. **DEF**: the fallbacks when no rule above fits.

---

## REL: does the provision create a relation?

**Core test, then the "no" cases (REL-02…25), then the "yes" cases (REL-30+; 26–29 left free for new "no" rules).** A "yes" rule (REL-30+) overrides a "no" rule when the provision itself creates a duty or power (REL-41).

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| REL-01 | Relation `yes` only if the provision **itself creates** an Obligation or Liberty (read with its stem); otherwise `no`. | "It shall be the duty of every employer to…" | S:L1; P:What to label/1; R | The master test. |
| REL-02 | A relation that exists only in the stem is not labelled on the item. | stem "X shall—", item that only defines a class | P:Rules/Stems and list items | **P-only** wording. See REL-15. |
| REL-03 | Amending text (substitutions, inserted text) → `no`; actors `mentioned`. | "in section 5, for 'X' substitute 'Y'" | S:SC Amendment (#57); P:Rules/Amending text; R | **Mismatch:** spec excludes amendment text from every layer (scope `amendment`, never labelled); prompt labels it `no`/`mentioned`. Gold should exclude these rows rather than label them. |
| REL-04 | Offences and penalties → `no`; everyone `none`/`mentioned`. The duty is in the provision the offence refers to. | "commits an offence", "is liable on conviction to…" | S:SC Offences; P:Rules/Offences; R | |
| REL-05 | Statutory defences → `no`; the accused `mentioned`. | "It is a defence for an accused … to prove that…" | S:SC Statutory defences (09-30); P:Rules/Statutory defences | Possible Immunity later (#70). |
| REL-06 | Definitions, deeming and legal fictions → `no`, even with "shall". A definition containing "shall" is a definition. | "'premises' includes any place"; "a notice shall be treated as served if…" | S:L1 (09-30); P:Rules/Cross-references, Legal fiction, Purpose/Choosing one | |
| REL-07 | Application, extent, scope and non-prejudice clauses → `no`. | "this Part applies to…"; "nothing in this section shall affect…" | S:L1; P:Rules/Cross-references | "nothing … shall affect" example is **P-only**. |
| REL-08 | Exemptions are scope, not Liberty → `no`; exempted party `mentioned` (POS-10). | "These Regulations shall not apply to an employee"; "nothing in this section requires…" | S:L1 + S:SC Exemptions (10-01); P:Rules/Exemptions | Overlaps REL-39 (power to exempt = yes) and REL-41 (exemption with fallback duty = yes). |
| REL-09 | Immunity (protection from another's power or liability) → `no` until #70. | "the Crown shall not be liable to prosecution"; "no improvement notice may be served on…" | S:L1 (10-01, #70 deferred) | **S-only.** Prompt has no Immunity rule; its purpose list files "shall not be liable" under `Liability` and "nothing … makes the Crown criminally liable" under `Exemption`. Relation agrees (`no`). |
| REL-10 | Cross-references and conditions: a provision that only references, conditions, details, defines or exempts a relation created elsewhere → `no`. | "the right under regulation 13 is a right to…" | S:SC Cross-references; P:Rules/Cross-references | Example is **P-only**. |
| REL-11 | Detail provisions: only sets the form, manner, discharge, conditions or procedure of a relation created in **another** provision → `no`. Test: does it refer to that relation ("the application under section 10", "the information referred to in paragraph (1)")? | "An application under section 10 must be made in the prescribed form…" | S:SC Detail provisions (10-01, gold v2 batch 1); P:Rules/Detail provisions; R | Counted once, where created. Overlaps REL-14, REL-31, HOLD-04 (see Conflicts C3, C4). |
| REL-12 | Discharge details → `no`. | "shall comply with his duty under paragraph (1) by…" | S:SC Discharge details (10-06); P:Rules/Timing items | Special form of REL-11. Conflicts in wording with HOLD-05 ("shall comply with…"); see C4. |
| REL-13 | Timing items → `no`. | "at suitable intervals", "within 8 weeks" | S:SC Timing items (10-06); P:Rules/Timing items | Conflicts with REL-34 (government deadline = yes); see C5. |
| REL-14 | Conditions on another provision's power → `no`. | "shall not grant any such exemption unless satisfied…"; "shall not consent unless—" | S:SC Conditions on another provision's power (10-06); P:Rules/A power to exempt | Looks like a prohibition but qualifies a power created elsewhere. |
| REL-15 | Class-definition and criterion items, under **any** duty or power → `no`; the relation stays on the provision above. | "(i) the person's residence is in Scotland"; "(b) persons who are not employees" | S:SC Details inside a content list (09-30) + Class-definition items (10-05); P:Rules/Class-definition | Fine line with REL-31 (items that complete the stem = yes). |
| REL-16 | Content list vs detail: "The notice/report/register … must— (a)…" is `yes` if it completes a duty created in the **same** provision; `no` if it details something required elsewhere, enforcement notices included. | yes: "The register must— (a)…" (duty created here); no: "the notice referred to in paragraph (1) must contain" | S:SC Content list vs detail (10-06); P:Rules/Content list or detail | Overlaps REL-11, REL-31. |
| REL-17 | Parliamentary procedure (annulment / affirmative clauses) → `no`. | "subject to annulment in pursuance of a resolution of either House"; "may not be made unless a draft has been laid…" | S:SC Detail provisions + Laying (10-05); P:Rules/Parliamentary procedure; R | Contrast REL-35 (duty to lay = yes). |
| REL-18 | Short titles, citation and commencement lists → `no`; so is a Gazette notification inside a commencement provision. | "This Act may be cited as…"; "such date to be notified in the Gazette" | S:SC Commencement and citation (09-30), Notification inside commencement (10-06); P:Rules/Commencement, Designation | Contrast REL-37 (power to commence = yes). |
| REL-19 | Procedural time limits → `no` (a limit on proceedings, not a liberty). | "Proceedings may be commenced within 6 months…" | S:SC Procedural time limits (09-30); P:Rules/Procedural time limits | "may" here is not a Liberty. |
| REL-20 | Savings and continuity → `no`. | "as if this Act had not passed"; "shall continue to have effect"; reliance on old documents as evidence | S:SC Transitional (10-05), Savings (10-06); P:Rules/Transitional, Savings | Contrast REL-38. |
| REL-21 | Electronic-delivery deeming → `no`. | "has effect as a delivery only if…" | S:SC Electronic delivery (10-06); P:Rules/Savings | Special form of REL-06. |
| REL-22 | Designation inside a definition → `no`; only a provision that itself confers the designating power is a Liberty. | "as may be designated by the Secretary of State by order" (within a definition) | S:SC Designation inside a definition (10-06); P:Rules/Designation | |
| REL-23 | Enforcing-authority designations, and functions lists of a **body** that impose no duty in their own text → `no`. | "X shall be (responsible as) the enforcing authority for…" | S:SC Enforcing-authority designations (10-05); P:Rules/Enforcing-authority | Contrast REL-40 (functions of a governed party = yes); see C2. |
| REL-24 | Money provided by Parliament (authority to spend) → `no`. | "There shall be paid out of money provided by Parliament…" | S:SC Money provided by Parliament (10-05); P:Rules/Money | |
| REL-25 | Appeals and reviews of decisions: `yes` only where the provision creates its own duty or power. | an appeal-procedure provision with no own duty → `no` | S:SC Appeal and inquiry procedure (10-06); P:Rules/Appeals | Mainly a purpose rule; relation follows REL-01. |
| REL-30 | Headless stems: "X shall—" / "X may—" with the substance in its items → `yes`, X active. Never `no` just because the object is in the items. | "The employer shall—" | S:SC Headless stems (10-05); P:Rules/Headless stems; R (v1.2) | |
| REL-31 | Content lists of schemes, regulations, notices: each item completes the stem's Obligation/Liberty → `yes`, same type. | "A scheme under this section must— (a)…"; "Regulations may— (a)…" | S:SC Content lists (09-30); P:Rules/Content lists | Holder per HOLD-11. Tension with REL-11's "refers to" test; see C3. |
| REL-32 | Stem-completing items: an item that completes a duty/power sentence begun in the stem → `yes`; the stem's holder is active (HOLD-02). | stem "It shall be the duty of each enforcing authority—", item "(a) to secure that the registers are available…" | P:Rules/Stems and list items; S:L1 (stem as holder source) | |
| REL-33 | A procedural duty created in its **own** text is a relation (not a detail). | "The Executive must consult the Secretary of State before issuing an approved code of practice" | P:Rules/Detail provisions | **P-only.** |
| REL-34 | A deadline that compels a government actor to act → `yes`, Obligation of the regulation maker. | "The first … regulations must come into force no later than 1 April 2018" | S:SC Detail provisions (10-01); P:Rules/Exception | Exception to REL-11/REL-13; see C5. |
| REL-35 | A duty to lay a report, direction or copy before Parliament or an Assembly → `yes`, government Obligation. | "The Secretary of State shall lay a copy of the report before Parliament" | S:SC Laying (10-05); P:Rules/Parliamentary procedure | |
| REL-36 | Review clauses → `yes`, Obligation of the reviewer. | "The Secretary of State must review these Regulations and publish a report" | P:Rules/Review clauses | **P-only.** |
| REL-37 | A power to commence → `yes`, Liberty of that actor. | "on such day as the Secretary of State may by order appoint" | S:SC Commencement and citation (09-30), Commencement powers (10-05); P:Rules/Commencement | Spec wording ambiguous; see C6. |
| REL-38 | A transitional provision that itself confers a time-limited power or imposes a time-limited duty → `yes`. | "until 1 January 2020 a Member State may continue to authorise…" | S:SC Transitional (10-05); P:Rules/Transitional; R (v1.2) | |
| REL-39 | A power to exempt → `yes`, Liberty. | "may by certificate exempt…" | S:SC Powers to exempt (10-06); P:Rules/A power to exempt | Contrast REL-08, REL-14. Spec wording ambiguous; see C6. |
| REL-40 | A **governed** party's listed functions → `yes`, Liberty (Right). | "safety representatives shall have the following functions— (a)…" | S:SC Functions lists of a governed party (10-06); P:Rules/Functions of a governed party | See C2. |
| REL-41 | Mixed provisions: any duty or power of its own → `yes`, whatever else the provision does. | an exemption plus a fallback duty | S:SC Mixed provisions (10-06); P:Rules/Mixed | Tie-breaker over REL-02…REL-25. |
| REL-42 | Passive and thing-subject duties are relations: `yes`, Obligation, even with no doer named. Holder per HOLD. | "records shall be kept for five years"; "every workplace shall be ventilated" | S:L1 (09-30, `Rule` type removed); P:What to label/2, Rules/Passive; R | |
| REL-43 | A duty with a qualifier stays a duty. | "Except in such cases as may be prescribed, it shall be the duty of every employer to prepare a written statement" | P:Purpose/Choosing one | **P-only**, stated as a purpose rule; it decides relation too. |

## HOLD: who holds the relation?

Try in order; stop at the first that names a holder.

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| HOLD-01 | The party the provision's own text puts under the duty or gives the power is `active`. | "The operator must notify the authority" → Operator | S:L2; P:What to label/position | |
| HOLD-02 | Stem holder: when the provision completes a sentence begun in its stem, the stem's holder is `active` here too. | stem "It shall be the duty of each enforcing authority—" → authority active on item (a) | S:L1, S:SC Holder unknown; P:Rules/Stems and list items | |
| HOLD-03 | A provision that names its own holder (text or stem) ignores applying provisions. | — | P:Rules/Holder named in an applying provision | **P-only** (spec implies it: #60 covers passive duties only). |
| HOLD-04 | Referenced provision: when a relation-creating provision names its holder only in a provision it points to, that holder is `active` (`inferred: false`). Only a holder the referenced text actually names. | "Regulations under subsection (2) may prescribe…" where (2) says "The Scottish Ministers may by regulations…"; "A power under this section may be exercised by force" | S:SC Holder named in a referenced provision (09-30); P:Rules/Holder named in a referenced provision | Never turns a detail into a relation (HOLD-10). |
| HOLD-05 | Applying provision (#60): a passive/thing-subject Obligation whose scope is covered by another same-law provision that puts a **named** party under a duty to comply with, or ensure compliance with, it → that party `active`, Obligation, `inferred: true`, raw_type `null`. Form 1: "X shall comply with / ensure [a thing] complies with the requirements of SCOPE". | Workplace Regs reg.6(1) "Effective and suitable provision shall be made to ensure that every enclosed workplace is ventilated", applied by reg.4(1) "Every employer shall ensure that every workplace … complies with any requirement of these Regulations" | S:SC Holder named in an applying provision (10-05); S:WE; P:Rules/Holder named in an applying provision; R | Scope may be explicit ("regulations 5 to 27", "Schedule 2") or instrument-wide ("these Regulations", "this Part"). |
| HOLD-06 | Every applying provision whose scope covers the duty contributes its holder: there can be several. | reg.4(1) Employer, reg.4(2) Person in Control, reg.4(5) Occupier | S:SC Holder named in an applying provision; S:WE | |
| HOLD-07 | Applying form 2, supervisory: both the supervising party and the parties it must make comply are holders. | "the principal contractor must take all reasonable steps to ensure that contractors … comply with the duties under these Regulations" | S:SC (10-05); P:Rules/applying | |
| HOLD-08 | Applying form 3, essential requirements (product regimes): the duty to meet the essential requirements applies every provision that sets them. | "a manufacturer must ensure that it has been designed and manufactured in accordance with the essential health and safety requirements" → applies "The equipment shall meet the essential requirements set out in Annex I" | S:SC (10-05); P:Rules/applying | |
| HOLD-09 | Applying form 4, prohibition: the person barred unless compliant holds the Obligation. | "no person shall keep the material unless he complies with paragraphs (3) to (6)" | S:SC (10-05); P:Rules/applying | |
| HOLD-10 | Referenced and applying provisions never create a relation: they only fill the holder of a provision that already creates one (not a detail, not relation `no`). | — | S:SC Detail provisions, applying row; P:Rules/referenced, applying | |
| HOLD-11 | An applying provision must name the party, and its scope must truly include this provision. A scope clause naming nobody gives no holder. | "these Regulations apply to every workplace" → no holder | S:SC applying (10-05); P:Rules/applying | |
| HOLD-12 | Instrument-maker as holder: "the scheme/regulations may/must…" is the maker's Obligation/Liberty when the stem or a referenced provision names the maker; otherwise holder unknown. | "The scheme may specify…" | S:SC Instruments are never actors, Content lists (09-30); P:Rules/Instruments, Content lists | Instrument itself is not an actor (LBL-12). |
| HOLD-13 | Applications to a court or tribunal: the court holds the Liberty; the applicant is `mentioned`. | "On the application of X, the court may…" → `Gvt: Judiciary` active | S:SC Applications to a court (09-30); P:Rules/Applications | |
| HOLD-14 | Holder in another instrument (#77): the same-law labeller leaves these holder unknown; a periodic agent pass proposes the holder and Jason approves it into the adjudicated tier. | "a no-smoking sign must be displayed … in accordance with the duty at section 6(1) of the Act" | S:SC Holder named in another instrument (10-05) | **S-only.** Gold needs a decision: label at prompt standard (unknown) or at adjudicated standard (parent-Act holder). See C9. |
| HOLD-15 | Never guess a holder (not the law's dominant actor, not "obviously the employer"). No named/stem/referenced/applying holder → holder unknown (DEF-02). | "The register shall be kept at the principal office" → no active actor | S:SC Holder unknown (10-05); P:Rules/Passive; R | |
| HOLD-16 | A provision can have several active holders. | authority's Obligation + public's implied Liberty (INF-01); several applying holders | S:L4; P:Rules (list end) | |

## TYPE: what each actor holds, and raw_type

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| TYPE-01 | `Obligation`: the actor bears a duty or prohibition. | "shall", "must", "shall not", "no person shall", "is required to", "it shall be the duty of" | S:L1; P:What to label/holds | "is required to", "it shall be the duty of" listed in P only. |
| TYPE-02 | `Liberty`: the actor holds a power, permission or entitlement, incl. implied rights (INF-01). | "may", "is entitled to", "power to" | S:L1; P:What to label/holds | Exemptions are not Liberty (REL-08); Immunity not modelled (REL-09). |
| TYPE-03 | Only `active` actors hold Obligation/Liberty. Counterparty, beneficiary and mentioned actors always hold `none`. Never copy the provision's type onto another party. | Employee in HSWA s.2(1) → `none` | S:L1 Non-active actors; P:What to label HARD RULE; R | |
| TYPE-04 | `raw_type` (`Obligation`/`Liberty`) only when relation is `yes` and **no** actor is active; otherwise `null` (including when an inferred holder is active). | "records shall be kept for five years" → raw_type Obligation | S:L4, S:SC Holder unknown; P:What to label/2, Rules/applying | |
| TYPE-05 | There is no `Rule` type: every "shall/must" that requires someone to act or bring about a state of affairs is an Obligation. | "traffic routes must be suitable" | S:L1 (09-30) | **S-only** (prompt never offers Rule, so no conflict). |
| TYPE-06 | Content-list and stem-completing items take the stem's type. | "Regulations may— (a)…" → Liberty | S:SC Content lists; P:Rules/Content lists | Same holder per HOLD-02/HOLD-12. |

## INF: inferred holders

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| INF-01 | Implied access right (#67): where a **government** actor's Obligation is to make something available for inspection/copying by, or to supply copies on request/payment to, a governed party named in the provision, that party is `active`, `Liberty`, `inferred: true`. | EPA 1990 s.20(7) "registers … available … for inspection by the public" → Public active Liberty inferred | S:SC Implied rights (#67), S:L1b; P:Rules/Implied access rights | Conflicts with ACT example; see C7. Edge: passive access duty with no government holder; see G3. |
| INF-02 | Enforcement and notice-service provisions never give an implied right. | "The authority shall serve a notice on the operator…" → Operator counterparty, no Liberty | S:SC Implied rights; P:Rules/Implied access rights | |
| INF-03 | Applying-provision holders are `inferred: true` (HOLD-05…09). | reg.4(1) Employer on reg.6(1) | S:SC applying (#60); P:What to label/3 | |
| INF-04 | `inferred` is `false` in every other case, including holders from the stem (HOLD-02) and referenced provisions (HOLD-04). | — | P:What to label/3 | Spec is silent on HOLD-04's flag; prompt is explicit. |

## POS: positions of the other parties

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| POS-01 | `active` = the holder (from HOLD). | — | S:L2; P | |
| POS-02 | Counterparty (duty): the **recipient of the duty's act**, the party it is done to or withheld from (notified, supplied, consulted, paid, given access, served, charged, whose request it answers). The test is the act, not who gains. | "OFCOM must send a copy to the applicant" → Applicant | S:L2 (10-01, Jason/Gemini/legal); P:Counterparty or beneficiary; R | |
| POS-03 | A prohibited act aimed at a party makes it the counterparty. | "No employer shall levy … any charge on any employee" (HSWA s.9) → Employee, act `charge` | S:L2 (legal); P:Counterparty or beneficiary | |
| POS-04 | "Ensure that X is provided with…" is still a recipient duty → counterparty. "Ensure the health and safety of X" is protective → beneficiary. | "every employer shall ensure that suitable PPE is provided to his employees" → counterparty `supply` | S:L2 (legal); P:Counterparty or beneficiary | Fine line with POS-13. |
| POS-05 | Counterparty (power or right): the party subject to it. | Water Act 1989 s.82(2)(c): Minister may … → Company counterparty | S:L2, S:WE; P:What to label/position | |
| POS-06 | Beneficiary: the party whose interest the duty **explicitly** protects without receiving its act. | HSWA s.2(1) "ensure … the health, safety and welfare at work of all his employees"; s.3(1) "persons not in his employment … are not exposed to risks"; "must not disclose to anyone other than the worker" → worker | S:L2 (10-01); P:Counterparty or beneficiary; R | "Only where explicit" is P/R wording; spec says it via POS-11. Spec worked example contradicts for s.2(1); see C1. |
| POS-07 | The same kind of duty gets the same position. | s.2(1) and s.3(1) both beneficiary | S:L2 | **S-only.** |
| POS-08 | Recipient **and** protected → counterparty (recipient wins). | "provide employees with information, instruction and training" (HSWA s.2(2)(c)) | S:L2; P; R | |
| POS-09 | Mentioned: referred to with no role in a relation created here, e.g. a body merely named in another party's duty. | a regulator named in a landlord's duty | S:L2; P:What to label/position | |
| POS-10 | A party exempted from scope is `mentioned`. | "shall not apply to an employee" → Employee | S:L1, S:SC Exemptions; P:Rules/Exemptions | |
| POS-11 | Trigger-condition actors are `mentioned`, never beneficiary. | "having regard to the risks to end-users"; "where the vessel presents a risk to persons" | S:SC Trigger-condition actors (10-05); P:Rules/Trigger-condition | |
| POS-12 | Withheld conduct: parties whose conduct is permitted or withheld are `counterparty`, act `other`. | "ensure that workers do not eat…"; "shall not be permitted to remain" | S:SC Withheld conduct (10-06); P:Rules/Withheld conduct | Overlaps POS-06 when the withholding is protective; see C8. |
| POS-13 | "Means at their disposal": ensuring a party *has* something without the text saying it is provided to them → `mentioned`. | "ensure the nominees have adequate time and means" | S:SC Means at their disposal (10-06); P:Rules/Means | Fine line with POS-04. |
| POS-14 | Relation `no` → every actor `mentioned`, holds `none` (offences, defences, amending text, court applicants). | "A person guilty of an offence under this section is liable…" → Person mentioned | S:WE; P:Rules/Offences, Amending, Statutory defences, Applications | Generalised from the per-case wording. |
| POS-15 | One entry per label, strongest role: if one actor has two roles, or two persons share a label, list it once at active > counterparty > beneficiary > mentioned. | receives a notification AND may shorten the period → active | S:SC One actor, two roles (10-05); P:Rules/One entry per label | "Two persons, same label" is **P-only**; see C10. |

## ACT: what an Obligation counterparty is owed (#75)

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| ACT-01 | `act` is set only for a `counterparty` of an Obligation; `null` for everyone else (active, beneficiary, mentioned, counterparty to a power or right). | Water Act s.82(2)(c) Company (power counterparty) → `null` | P:What to label/4 | Spec says "each counterparty's correlative can carry what it's owed"; prompt narrows to Obligation. Minor; see M-list. |
| ACT-02 | A passive duty's counterparty gets an act too, holder unknown or not. | "notice shall be given to the operator" → Operator `notify` | S:L1b act (10-05); P:What to label/4; R (v1.2) | |
| ACT-03 | `notify`: told of an event or decision. | RIDDOR reg.4 → enforcing authority | S:L1b; P | |
| ACT-04 | `supply`: given information, a copy or a thing. Needs text saying the party is provided with something (POS-13). | MHSWR reg.10 → employees; PPE Regs reg.4 | S:L1b; P | |
| ACT-05 | `consult`: consulted. | safety representatives | S:L1b; P | |
| ACT-06 | `pay`: paid. | — | S:L1b; P | |
| ACT-07 | `give_access`: given access or inspection. | spec: "EPA s.20(7), the public" | S:L1b; P | Spec example conflicts with INF-01; see C7. |
| ACT-08 | `serve`: served with a notice. | "The authority shall serve a notice on the operator" | S:L1b; P | Overlap with `notify` for notices; see G2. |
| ACT-09 | `charge`: a charge levied on, or withheld from, the party. | HSWA s.9 → employee | S:L1b; P | |
| ACT-10 | `answer_request`: the duty answers the party's request. | — | S:L1b; P | |
| ACT-11 | `other`: an Obligation counterparty whose act fits none of the above, incl. withheld conduct (POS-12). | "ensure that workers do not eat…" | S:L1b, S:SC Withheld conduct; P | |

## LBL: actor label choice

| ID | Rule | Example | Source | Notes |
|---|---|---|---|---|
| LBL-01 | Dictionary labels only, exact spelling; if none fits, `OTHER: <short description>`. | `OTHER: Taker of provisional measure` | P:Actor labels; R | |
| LBL-02 | Use the most specific label that fits. | — | P:Actor labels | **P-only.** |
| LBL-03 | Don't invent actors the provision (with its stem) doesn't refer to. Actors are persons, bodies or classes of persons. | — | P:Actor labels, What to label | **P-only.** |
| LBL-04 | A canonical label stands for the party the text denotes. | "the Regulator" → `Gvt: Authority` | P:Actor labels | **P-only.** |
| LBL-05 | An officer, inspector or "authorised person" authorised by a government body is `Gvt: Officer`, never `Spc: Authorised Person`. | "an inspector appointed by the enforcing authority" | P:Actor labels (D comment 10-05) | Spec layer 3 still lists `Spc: Authorised Person` as government; see C11. |
| LBL-06 | `Spc: Authorised Person` is a specialist authorised by the **duty holder** (governed). | — | D comment (10-05) | In neither spec nor prompt rules. |
| LBL-07 | An officer of a body corporate (director, manager, secretary or similar officer) is `Ind: Company Officer`, not `Gvt: Officer`. | "any director, manager, secretary or other similar officer of the body corporate" | D (10-05); session training-labels-slm | In neither spec nor prompt rules (only the dictionary block in the prompt). |
| LBL-08 | "Economic operator" (manufacturer/importer/distributor umbrella) is `SC: Economic Operator`, not `Operator`. | "economic operators shall…" | D (10-05) | In neither spec nor prompt rules. |
| LBL-09 | "The Scottish Ministers" / "the Welsh Ministers" → `Gvt: Devolved Admin: Scottish Ministers` / `…: Welsh Ministers`, not the Parliament or Assembly. | — | P:Actor labels | **P-only.** |
| LBL-10 | "The person having (the management and) control of…" → `Ind: Person in Control`. | — | P:Actor labels | **P-only.** |
| LBL-11 | A court or tribunal → `Gvt: Judiciary`. | "the court may…" | P:Rules/Applications | **P-only** label (spec names no label). |
| LBL-12 | Instruments are never actors: scheme, regulations, order, notice, licence. | "The scheme may specify…" → no Scheme actor | S:SC Instruments (09-30); P:Rules/Instruments; R | Holder per HOLD-12. |
| LBL-13 | Member State: an actor when it bears the duty/power or receives the act; a place (not listed) when it only names a jurisdiction or location. Same for countries. | actor: "Member States shall ensure…"; place: "placed on the market in a Member State" | S:SC Member State (10-05); P:Rules/Member State; R (v1.2) | |
| LBL-14 | Things are never actors: the environment, animals, property, industries, countries. | — | P:Rules/Member State; R | Spec names only countries; rest **P-only**. |
| LBL-15 | Holder class (government/governed) comes from the dictionary `type`, never the label prefix; the label choice therefore decides Duty vs Responsibility, Right vs Power. `HM Forces` is government. | `Crown`, `Spc: Notifying Authority` → government | S:L3 (09-29/09-30) | Not labelled; explains why label choice matters. |

## DEF: defaults when no rule applies

| ID | Rule | Source | Notes |
|---|---|---|---|
| DEF-01 | Relation: no rule says `yes` and the provision has no own Obligation/Liberty → `no`. Be conservative. | P (opening "precise and conservative"); REL-01 | |
| DEF-02 | Holder: no own, stem, referenced or applying holder → holder unknown: no actor `active`, `raw_type` set. | S:SC Holder unknown (10-05); P; R | |
| DEF-03 | Position: an actor with no role in a relation created here → `mentioned`. No explicit protective purpose → not beneficiary. | S:L2; P | |
| DEF-04 | Holds: not `active` → `none`. | TYPE-03 | |
| DEF-05 | Inferred: `false` unless INF-01 or INF-03. | P:What to label/3 | |
| DEF-06 | Act: `null` unless an Obligation counterparty; `other` if that counterparty's act fits no named value. | P:What to label/4 | Spec: act absent = unknown; prompt: `null` = not applicable. |
| DEF-07 | Label: no dictionary fit → `OTHER: …`; in the gold set that row is a **new edge** (candidate dictionary entry). | P:Actor labels; phase 0a session | |
| DEF-08 | Immunity-shaped text → relation `no` until #70. | S:L1 | |

---

## Conflicts and overlaps

| # | Rules | Problem | Suggested handling |
|---|---|---|---|
| C1 | POS-06/POS-07 vs S:WE | Spec worked example lists HSWA s.2(1) Employee as **counterparty** (`claim_right`); layer 2 (10-01) makes s.2(1) employees **beneficiary**. The layer 1b prerequisite paragraph describes the old split. Worked example is stale. | Gold follows layer 2 (beneficiary). Fix the worked-example row in the spec. |
| C2 | REL-23 vs REL-40 | "Functions lists": a body's → `no`; a governed party's → `yes`. "Body" is undefined; a governed body (e.g. a safety committee, a company) fits both. | Ruling: does the split follow holder class (government body vs governed party)? |
| C3 | REL-11 vs REL-16 vs REL-31 | REL-11's test is "it refers to a relation elsewhere"; REL-31's "A scheme **under this section** must—" refers too but is `yes` because the stem creates the requirement. REL-16 splits "the notice … must—" by whether the duty is created in the same provision. Three overlapping rules for list stems. | Cite all three; the deciding question is "is the duty to produce the scheme/notice created in this provision's own text?" |
| C4 | REL-11/REL-12 vs HOLD-05 | "X shall comply with his duty under paragraph (1) by…" is a detail (`no`); "X shall comply with the requirements of these Regulations" is an applying provision. Neither the spec nor the prompt says whether the **applying provision itself** is relation `yes` (an Obligation of X) or a cross-reference (`no`). | Ruling needed for the applying provision's own row. |
| C5 | REL-13 vs REL-34 | "Within 8 weeks" timing item = `no`; "must come into force no later than 1 April 2018" = `yes`. The exception is tied to a government actor compelled to act. | Cite REL-34 only where a government actor must act by the date. |
| C6 | REL-37, REL-39 (spec wording) | Spec rows "Commencement powers" and "Powers to exempt" end with "An exception to machinery ⇒ relation `none`, like…". This reads as if relation is `none`; it means "an exception to [the rule that] machinery ⇒ none". Prompt is clear (`yes`). | Reword the spec rows. Gold: `yes`. |
| C7 | INF-01 vs ACT-07 | Spec's `give_access` example is "EPA s.20(7), the public", but under #67 the public in s.20(7) is `active` (inferred Liberty), and the prompt gives `act` only to counterparties, so the public gets `act = null`. | Change the spec's `give_access` example to a non-#67 case, or rule that a #67 party also carries the act. |
| C8 | POS-12 vs POS-06/POS-08 | "Ensure that workers do not eat…" is protective of workers, but withheld conduct makes them `counterparty`/`other`. Consistent via POS-08 (recipient wins) only if withholding counts as the "act". | Cite POS-12 (it is the later, specific ruling). |
| C9 | HOLD-14 vs HOLD-15/DEF-02 | Cross-instrument holders (#77) are holder-unknown under the prompt but may hold an adjudicated holder. Gold must choose one standard. | Ruling: gold at prompt standard (unknown) with an adjudicated note, or at adjudicated standard. |
| C10 | POS-15 (prompt extension) | "Two persons, same label → one entry, strongest role" drops a role: "an employer shall notify another employer" keeps Employer `active` only and loses the counterparty and its act. | Accept as known loss, or rule an exception. |
| C11 | LBL-05/LBL-06 vs S:L3 | Spec layer 3 (09-30) lists `Spc: Authorised Person` as **government**; dictionary (10-05) makes it governed, and the prompt routes government-authorised officers to `Gvt: Officer`. | Update spec layer 3 table. Gold follows prompt + dictionary. |
| C12 | REL-08 vs REL-09 (purpose side) | "Nothing in this section makes the Crown criminally liable" is `Exemption` in the prompt's purpose list; "the Crown shall not be liable to prosecution" is Immunity in the spec. Relation is `no` either way. | No relation impact; for the purpose refactor. |
| C13 | Outside sources: `correlative-rules.yaml` rule 3 | ~~Inferred `Ind: Public` beneficiary for every active enforcement authority~~, contradicting POS-06/POS-11. **Resolved 2026-10-05:** rule retired (Jason), 1,148 hub rows deleted (`scripts/migrations/retire_public_correlative_20261005.py`). | None; older evidence may still show these rows, so ignore them. |

**Fine lines (not conflicts, but where reviewers should expect hard rows):** REL-15 vs REL-31 (criterion item vs completing item); POS-04 vs POS-13 ("provided with" vs "have"); REL-17 vs REL-35 (laid-draft procedure vs duty to lay); REL-18 vs REL-37 (commencement list vs power to commence); REL-20 vs REL-38 (savings vs time-limited power); REL-08 vs REL-39 vs REL-14 (exemption vs power to exempt vs conditions on it).

## Gaps (no rule yet; likely "new edge" rows)

- **G1** One actor holding both an Obligation and a Liberty in the same provision: the schema allows one `holds` per actor; no rule says which wins.
- **G2** Act precedence when several fit, e.g. "serve a notice on X informing it…" (`serve` vs `notify`), "send a copy" (`supply` vs `notify`).
- **G3** A passive access duty with no government holder ("the register shall be open to inspection by the public"): INF-01 needs a government actor's Obligation, so is the public a `give_access` counterparty of a holder-unknown duty (ACT-02)?

## Spec vs prompt mismatches

**In the prompt, not the spec:** REL-02 (stem-only relation not labelled), REL-07 example ("nothing … shall affect"), REL-10 example, REL-33 (procedural duty in own text), REL-36 (review clauses), REL-43 (qualified duty), HOLD-03 (own holder beats applying), INF-04 (explicit `inferred: false` for referenced holders), ACT-01 (act for Obligation counterparties only), POS-06 "only where explicit" wording, POS-15 "two persons, same label", TYPE-01 extra triggers, LBL-02, LBL-03, LBL-04, LBL-05, LBL-09, LBL-10, LBL-11, LBL-14 (environment, animals, property, industries).

**In the spec, not the prompt:** REL-09 (Immunity, #70), REL-03 handling (spec excludes amendment text; prompt labels it), HOLD-14 (#77 cross-instrument holders), TYPE-05 (no `Rule` type), POS-07 (same kind of duty, same position), LBL-15 (holder class by dictionary, HM Forces), act "absent = unknown" semantics.

**Spec internally inconsistent:** C1 (s.2(1) worked example), C6 (commencement/exempt-power wording), C7 (`give_access` example), C11 (Authorised Person class).

**In neither (dictionary/session only):** LBL-06, LBL-07, LBL-08.

---

## Appendix A: deferred to the purpose refactor

These rulings decide purpose only, or their purpose half is set aside here. Relation/actor halves are catalogued above (ID in brackets).

| Ruling | Purpose part | Source |
|---|---|---|
| Time-limited disapplications | `Exemption` (Exemption before Transitional Arrangement in machinery order) | S:SC (10-05); P |
| Class-definition / criterion items [REL-15] | relation-`no` items take `Application+Scope` | S:SC (10-06); P |
| Savings of old law or status [REL-20] | `Transitional Arrangement`, ahead of machinery order; pure deeming stays `Interpretation+Definition` | S:SC (10-06); P |
| Electronic delivery [REL-21] | `Interpretation+Definition` | S:SC (10-06); P |
| Designation inside a definition [REL-22] | `Interpretation+Definition` | S:SC (10-06); P |
| Gazette notification in commencement [REL-18] | `Enactment+Citation+Commencement` | S:SC (10-06); P |
| Enforcing-authority designations, body functions [REL-23] | `Establishment+Constitution` | S:SC (10-05); P |
| Money provided by Parliament [REL-24] | `Charge+Fee` | S:SC (10-05); P |
| Appeals [REL-25] | `Defence+Appeal` | S:SC (10-06); P |
| Headless stems [REL-30] | `Requirement` / `Power Conferred`, never `Procedure+Detail` | S:SC (10-05); P |
| Transitional powers [REL-38] | `Transitional Arrangement` | S:SC (10-05); P |
| Commencement powers [REL-37] | `Enactment+Citation+Commencement` | S:SC (10-05); P |
| Powers to exempt [REL-39] | `Exemption` | S:SC (10-06); P |
| Functions of a governed party [REL-40] | `Power Conferred` | S:SC (10-06); P |
| Mixed provisions [REL-41] | purpose follows the operative part | S:SC (10-06); P |
| Content list vs detail, timing, discharge, conditions on a power [REL-12…14, REL-16] | `Procedure+Detail` | S:SC (10-06); P |
| Immunity / "shall not be liable" [REL-09] | `Liability` (prompt) vs Immunity (spec); see C12 | S:L1; P:Purpose |
| Purpose precedence | machinery > sanctions > `Charge+Fee` > `Requirement`/`Power Conferred` > `Procedure+Detail` | P:Purpose; R |
| Requirement vs Procedure+Detail test | same as REL-11 | P:Purpose |
| List items take the stem's purpose | unless the item itself does something else | P:Purpose |
| Purpose ↔ relation consistency | `Requirement`/`Power Conferred` ↔ `yes`; machinery/`Procedure+Detail` ↔ `no`, except transitional, commencement and exempt powers | P:Purpose |

**Purpose coupling to watch in the refactor:** the prompt states REL-43 (qualified duty) and the "a definition containing shall is a definition / an offence provision is an offence" tie-break (REL-04, REL-06) inside the purpose section, and the consistency rule lets purpose imply relation. When purpose is rewritten, those relation halves must move into the relation rules, not be lost.
