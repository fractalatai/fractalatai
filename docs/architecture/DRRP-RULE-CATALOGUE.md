# DRRP Rule Catalogue v2: principles

**Adopted 2026-10-07 (phase B).** The 123 rules of v1 (`DRRP-RULE-CATALOGUE-V1.md`, kept as history) are now principles, precedents, retired rules, dictionary decisions or merges.
- **Unit:** every principle applies to the **legal sentence** (stem, items and closing words), labelled once.
- **Principles** are general and recur: about 3 provisions from 2 or more laws, or a schema invariant every row depends on (TYPE-03, ACT-01).
- **Precedents** are reviewed gold rows, retrieved by pattern tag and shown as examples. They are never cited as rules.
- **Promotion:** a new pattern becomes a principle only after about 3 provisions from 2 or more laws, **and Jason approves**. Labellers report candidate patterns; they don't make rules.
- **Context principles** (HOLD-05, INF-03) need provisions beyond the sentence. They are decided by the holder-linking step and scored apart from the sentence tiers.
- The **crosswalk** at the end gives every v1 ID's fate, so old citations in gold still resolve.

Sources: the phase B session (`.claude/sessions/parsing/2026-10-06-phaseB-principles-precedents.md`), `data/gold/v4/rule_classification.csv`, and the Gemini reviews in `data/code-review/drrp-catalogue-v2-principles.md`.

## Relation (yes / no / continues)

| v2 ID | Principle | Example (real text) | Covers |
|---|---|---|---|
| REL-01 | Relation `yes` only if the sentence itself creates an Obligation or Liberty; otherwise `no`, and be conservative. *Reworded* (sentence, not "provision with its stem"). | "It shall be the duty of every employer to…" | REL-01, REL-25, DEF-01 |
| REL-28 | A sentence that only sets the content, form, manner, timing, discharge, condition or limit of a duty or power created in **another** sentence → `continues`: no holder, no raw_type. This includes a passive or agentless manner sentence ("may be served by post") and a content sentence that names no holder ("The scheme must include—"). The holder of the continued duty, if named here, is `mentioned`; other actors keep their roles in the continued relation. | "Regulations under subsection (1) may—" → `continues` (Jason, CC(S)A 2019 s.31(2): "from 31(1)") | REL-28, REL-11, REL-12, REL-14, HOLD-04, HOLD-12 |
| REL-33 | A sentence whose "shall/must/may" has a **named party as its subject** puts its own duty or power on that party → `yes`, even when it is procedural, qualified ("so far as is reasonably practicable"), deadline-bound or a permitted way of discharging another duty. No named subject (passive, agentless) → REL-28 decides. | "the hirer may inform the agency worker by a general announcement" → Hirer active Liberty (Jason, AWR 2010 reg.13(4)) | REL-33, REL-34, REL-43, REL-46 |
| REL-42 | Passive and thing-subject "shall/must" sentences are `yes`, Obligation, even with no doer named — unless REL-06 applies ("shall be treated as", "shall be taken to", "deemed"). There is no `Rule` type. | "every workplace shall be ventilated" | REL-42, TYPE-05 |
| REL-41 | Any duty or power of the sentence's own beats its machinery class. This covers laying duties, review clauses, powers to exempt and time-limited transitional powers or duties. *Reworded.* | "until 1 January 2020 a Member State may continue to authorise…" | REL-41, REL-35, REL-36, REL-37, REL-38, REL-39 |
| REL-07 | Machinery (sentences that make the law work as a law rather than tell anyone to do anything) creates no relation → `no`. This covers application, extent, scope, non-prejudice, savings, citation, commencement (including "on such day as the Secretary of State may by order appoint"), parliamentary procedure ("may not be made unless a draft has been laid before") and money provided by Parliament. Amending text is not labelled: it is excluded from gold. | "nothing in this section shall affect…"; "This Act comes into force on such day as the Secretary of State may by order appoint" → `no` | REL-07, REL-03, REL-17, REL-18, REL-20, REL-24 |
| REL-27 | Designating an enforcing authority or regulator ("The HSE shall be the enforcing authority for these Regulations") is an Obligation on the designated body: `yes`, that body `active`, purpose Requirements. Later provisions expand what it must do. *Reverses v1 REL-27 (designations → `no`); Jason 2026-10-07: "the HSE is literally required to be this thing and not e.g. a local authority."* | "The HSE shall be the enforcing authority for these Regulations" → Gvt: Agency: HSE active Obligation, Requirements | REL-27 |
| REL-06 | Definitions, deeming and legal fictions → `no`, even with "shall"; this beats REL-42 ("shall be treated as", "shall be taken to", "deemed"). A deeming sentence that changes who holds duties elsewhere ("shall be treated as the employer") is tagged `deemed-holder` for the holder-linking step. | "a notice shall be treated as served if…" | REL-06, REL-21, REL-22 |
| REL-08 | An exemption is scope, not a Liberty → `no`, and the exempted party is `mentioned`. | "These Regulations shall not apply to an employee" → Employee mentioned | REL-08, POS-10 |
| REL-04 | Offences, penalties, statutory defences, procedural time limits and immunities → `no`, and every actor is `mentioned`. *Reworded* (sanctions family). | "It is a defence for an accused … to prove that…" | REL-04, REL-05, REL-09, REL-19, DEF-08 |

## Holder and position

| v2 ID | Principle | Example (real text) | Covers |
|---|---|---|---|
| HOLD-01 | Each party the sentence's own text puts under a duty or gives a power is `active`. There can be several, and a court given a power on application holds it. | "Any other person … shall send" → Ind: Person active, beside Gvt: Planning Inspector and Gvt: Minister (Jason, SI 2002/2686 reg.8(10): "two duties") | HOLD-01, HOLD-13, HOLD-16, POS-01 |
| HOLD-05 | *Context principle: decided by the holder-linking step, scored apart from the sentence tiers.* Some duties are passive or thing-subject and name no holder. If another same-law provision puts a **named** party under a duty to comply, or to ensure compliance, with a scope that truly covers the duty, that party is `active`. Every such provision counts. The applying provision's own sentence is a pointer (`no`), and a sentence that names its own holder ignores it. | reg.6(1) "every enclosed workplace is ventilated", applied by reg.4(1) "Every employer shall ensure that every workplace … complies with any requirement of these Regulations" | HOLD-05, HOLD-03, HOLD-06, HOLD-07, HOLD-08, HOLD-09, HOLD-10, HOLD-11, REL-26 |
| HOLD-15 | Never guess a holder: not the law's dominant actor, and not "obviously the employer". If none is named or supplied, the holder is unknown: no actor is `active` and `raw_type` is set. | "The register shall be kept at the principal office" → no active actor | HOLD-15, DEF-02 |
| POS-02 | The counterparty of a duty is the **recipient of its act**: notified, supplied, consulted, paid, served, given access or charged, or the target of a prohibited act. A party that is both recipient and protected is a counterparty. | "OFCOM must send a copy to the applicant" → Applicant counterparty | POS-02, POS-03, POS-04, POS-08 |
| POS-05 | The counterparty of a power or right is the party subject to it. | Water Act 1989 s.82(2)(c): the Minister may … → Company counterparty | POS-05 |
| POS-12 | Parties whose conduct the duty permits or withholds are `counterparty`, act `other`. | "ensure that workers do not eat…" | POS-12 |
| POS-13 | A party the duty ensures **has** something, without the text saying it is provided to them, is not a recipient → `mentioned`. *Reworded.* | "ensure the nominees have adequate time and means" | POS-13 |
| POS-19 | A party is a `beneficiary` when the words name it as what is protected ("health and safety of X", "protect X", "X are not exposed to risks"). This holds in a duty, a power or a condition on one. Words that only set when or whether the rule applies ("where X…", "having regard to X", "on the application of X") make the party `mentioned`. | "unless satisfied that the health and safety of persons likely to be affected … will not be prejudiced" → Person beneficiary (Jason, gas regs 1998 reg.40(2)) | POS-19, POS-06, POS-07, POS-11 |
| POS-09 | An actor with no role in a relation created here is `mentioned`. | "shall be treated as employment by SOCA" → SOCA mentioned | POS-09, DEF-03 |
| POS-14 | Relation `no` → every actor is `mentioned` and holds `none`. | "A person guilty of an offence under this section is liable…" → Person mentioned | POS-14 |
| POS-15 | One entry per label **per role**: one party with a duty and a power is one entry with `holds: both`. A label standing for parties in different roles gets one entry per role. A `mentioned` role next to a substantive one is dropped. `act` is a list. | "a person who carries out a search of a relevant person" → Ind: Person active Obligation **and** Ind: Person beneficiary | POS-15 |

## Type and holds

| v2 ID | Principle | Example (real text) | Covers |
|---|---|---|---|
| TYPE-01 | `Obligation`: the actor bears a duty or prohibition ("shall", "must", "shall not", "no person shall", "is required to", "it shall be the duty of"). | "It shall be the duty of every employer to…" | TYPE-01 |
| TYPE-02 | `Liberty`: the actor holds a power, permission or entitlement ("may", "is entitled to", "power to"). | "may by certificate exempt…" | TYPE-02 |
| TYPE-03 | Only `active` actors hold Obligation, Liberty or `both`. Everyone else holds `none`. (A schema invariant, decided on every row.) | Employee in HSWA s.2(1) → `none` | TYPE-03, DEF-04 |
| TYPE-04 | `raw_type` is set only when the relation is `yes` and **no** actor is active, including inferred holders. Otherwise it is `null`. | "records shall be kept for five years" → raw_type Obligation | TYPE-04 |

## Act

| v2 ID | Principle | Example (real text) | Covers |
|---|---|---|---|
| ACT-01 | `act` is set only for a counterparty of an Obligation, whether the holder is known or not. Everyone else gets `[]`, including a power's or right's counterparty. (A schema invariant.) | Water Act s.82(2)(c) Company (power counterparty) → no act | ACT-01, ACT-02, ACT-13, DEF-06 |
| ACT-12 | Map the duty's main verb onto the fixed classes. `notify` covers told or served. `supply` needs text saying the party is provided with something. The others are `consult`, `pay`, `give_access`, `charge`, `answer_request` and `other`, which includes withheld conduct. New verbs never add classes. | "serve a notice on X informing it" → `notify`; "send a copy" → `supply` | ACT-12, ACT-03, ACT-04, ACT-05, ACT-06, ACT-07, ACT-09, ACT-10, ACT-11 |

## Inference

| v2 ID | Principle | Example (real text) | Covers |
|---|---|---|---|
| INF-03 | *Context principle (as HOLD-05).* A holder not named in the sentence, but supplied by an applying provision (#60) or by the parent Act (#77), is `active` with `inferred: true`. Every holder named in the sentence's own text is `inferred: false`. *Reworded.* | reg.4(1) Employer on Workplace Regs reg.6(1) → active, inferred | INF-03, INF-04, DEF-05, HOLD-14 |

## Purpose (pointer)

Purpose rules live in `docs/architecture/PURPOSE-CLASSIFICATION.md`:
- 13 purpose members as the law's anatomy, in precedence order with Substantive requirements as the default (2026-10-07), single-select;
- a sentence takes its own class, or `inherit` when it has none.

No P: rule is classified here. Two relation halves that sat in the purpose text are now relation principles:
- REL-43 (a qualified duty stays a duty) → REL-33;
- the "definition with shall is a definition" tie-break → REL-06.

**Watch:** Jason's purpose rulings put parliamentary procedure and conditions on a power under **Requirements**. That breaks the purpose doc's consistency check ("Requirements ⇒ relation exists"). See Question 6.

## Labels (pointer)

Three principles stay here. Every label pick goes to the actor dictionary (see Dictionary below).

| v2 ID | Principle | Example (real text) | Covers |
|---|---|---|---|
| LBL-01 | Use dictionary labels only, with exact spelling. If none fits, use `OTHER: <short description>`, which is a new edge and a dictionary candidate. | `OTHER: Taker of provisional measure` | LBL-01, DEF-07 |
| LBL-02 | Label the party the text denotes with the most specific dictionary label that fits. | "the Regulator" → `Gvt: Authority` | LBL-02, LBL-04 |
| LBL-03 | Actors are persons, bodies or classes of persons that the sentence refers to. Instruments, things, hazards and places never are. A Member State or country is an actor only when it holds the duty or power or receives the act. *Reworded.* | "'Biological agent' is not an actor - it's a source of harm/infection" (Jason, Directive 2000/54 Art.18(2)) | LBL-03, LBL-12, LBL-13, LBL-14 |

---

## Precedent patterns

These stay as reviewed examples in `drrp_gold`, retrieved by tag, never cited as rules.

| Pattern | Old IDs | One line |
|---|---|---|
| `functions-list` | REL-44 | "The functions of X shall be—": a government body's functions are an Obligation, a governed party's a Liberty (CAA 1982 s.3, CTSA 2015 s.36(4)). Promotion candidate after the phase C top-up. |
| `implied-access-right` | INF-01, INF-02 | A government duty to make registers or copies available to a named governed party gives that party an inferred Liberty. With #78 the party also takes a counterparty entry, act `give_access`, on the government's duty (Jason 2026-10-07). Enforcement and notice service never do. No gold provision yet (pilot: EPA 1990 s.20(7)). |
| `no-person-shall-be-engaged` | HOLD-17 | "No person shall be engaged…": every applying holder who can engage someone holds it, and the person engaged is mentioned (EAW 1989 reg.14, reg.16). |
| `deemed-holder` | REL-06 | "A person who … shall be treated as the employer …": `no` here, but it decides who holds duties elsewhere; the holder-linking step uses it (Jason 2026-10-07). |
| `participation-right` | TYPE-07 | "shall take part in … or shall be consulted" gives the participants a Liberty despite "shall" (Directive 89/391 Art.11(2)). |

## Retired

| IDs | Why |
|---|---|
| REL-02, REL-13, REL-15, REL-16, REL-30, REL-45, HOLD-02, TYPE-06, POS-17, POS-18 | Stem, item and continuation mechanics. The sentence is labelled once (JUSTIFY_UNITS.md). REL-45's s.31(2) refinement and POS-17's "actors keep their roles" both move into REL-28 at sentence level. |
| INF-04 (stem clause), REL-28 (item half) | Same reason. The remaining halves are merged into INF-03 and kept in REL-28. |
| REL-10 | Its outcome (`no` for details and conditions of a relation elsewhere) was overturned on 10-06. It is superseded by REL-28 (continues), REL-06 (defines) and REL-08 (exempts). |
| POS-16 | Superseded by POS-15 as revised for #78 (option c: per-role key, `both`, act list). |

## Dictionary

| ID | Label decision | Meets the enrichment rule? |
|---|---|---|
| LBL-05 | An officer or inspector of a government body → `Gvt: Officer`; an **authorised person** acting for an enforcing authority → `Gvt: Authorised Person` (approved by Jason 2026-10-07: 598 provisions, 88 laws, 33 Families; **dictionary task pending**: family-gate it against the duty-holder specialist in electrical, mines and rail law, and migrate the 543 hub rows, the LLM prompt rule and the aliases) | Yes |
| LBL-06 | A specialist authorised by the duty holder → `Spc: Authorised Person` (governed: electrical, mines, rail) | Yes (2 Families) |
| LBL-07 | A director, manager or secretary of a body corporate → `Ind: Company Officer` | Not shown by gold (0 provisions after the Companies Act drop). The entry already exists. |
| LBL-08 | The manufacturer/importer/distributor umbrella → `SC: Economic Operator` | Yes (2 Families) |
| LBL-09 | Scottish and Welsh Ministers → `Gvt: Devolved Admin: …` | Yes (8 laws, 7 Families) |
| LBL-10 | "the person having control of…" → `Ind: Person in Control` | Yes (4 laws, 3 Families) |
| LBL-11 | A court or tribunal → `Gvt: Judiciary` | Yes (3 Families). Jason asks for `Gvt: Judiciary: Employment Tribunal`. |
| LBL-15 | Holder class comes from the dictionary `type`, not the label prefix | Not a pick. It documents the dictionary field. |

## Crosswalk: v1 rules that are not v2 principles

| v1 ID | Now | v1 text |
|---|---|---|
| REL-02 | retired (sentence units) | A relation that exists only in the stem is not labelled on the item. |
| REL-03 | merged into REL-07 (amending text excluded from gold) | Amending text (substitutions, inserted text) → `no`; actors `mentioned`. |
| REL-05 | merged into REL-04 | Statutory defences → `no`; the accused `mentioned`. |
| REL-09 | merged into REL-04 | Immunity (protection from another's power or liability) → `no` until #70. |
| REL-10 | retired (REL-28 (detail/condition → continues); REL-06 (defines); REL-08 (exempts)) | Cross-references and conditions: a provision that only references, conditions, details, defines or exempts a relation created elsewhere → `no`. |
| REL-11 | merged into REL-28 | Detail provisions: only sets the form, manner, discharge, conditions or procedure of a relation created in another provision → `no` `continues` (10-06, REL-28). Test: does it refer to that relation ("the application unde |
| REL-12 | merged into REL-28 | Discharge details → `no` `continues` (10-06, REL-28). |
| REL-13 | retired (sentence units) | Timing items → `no` `continues` (10-06, REL-28). |
| REL-14 | merged into REL-28 | Conditions on another provision's power → `no` `continues`. |
| REL-15 | retired (sentence units) | Class-definition and criterion items, under any duty or power → `no`; the relation stays on the provision above. |
| REL-16 | retired (sentence units) | Content list vs detail: "The notice/report/register … must— (a)…" is `yes` if it completes a duty created in the same provision; `no` if it details something required elsewhere, enforcement notices included. |
| REL-17 | merged into REL-07 | Parliamentary procedure (annulment / affirmative clauses) → `no`. |
| REL-18 | merged into REL-07 | Short titles, citation and commencement lists → `no`; so is a Gazette notification inside a commencement provision. |
| REL-19 | merged into REL-04 | Procedural time limits → `no` (a limit on proceedings, not a liberty). |
| REL-21 | merged into REL-06 | Electronic-delivery deeming → `no`. |
| REL-22 | merged into REL-06 | Designation inside a definition → `no`; only a provision that itself confers the designating power is a Liberty. |
| REL-24 | merged into REL-07 | Money provided by Parliament (authority to spend) → `no`. |
| REL-25 | merged into REL-01 | Appeals and reviews of decisions: `yes` only where the provision creates its own duty or power. |
| REL-26 | merged into HOLD-05 | An applying provision's own row is a pointer → `no`; the named party is `mentioned`. Its Obligation goes on each provision it applies (HOLD-05). Forms 2 (supervisory) and 4 (prohibition) are duties in their own right (HO |
| REL-20 | merged into REL-07 | Savings and continuity → `no`. |
| REL-30 | retired (sentence units) | Headless stems: "X shall—" / "X may—" with the substance in its items → `yes`, X active. Never `no` just because the object is in the items. |
| REL-34 | merged into REL-33 | A deadline that compels a government actor to act → `yes`, Obligation of the regulation maker. |
| REL-35 | merged into REL-41 | A duty to lay a report, direction or copy before Parliament or an Assembly → `yes`, government Obligation. |
| REL-36 | merged into REL-41 | Review clauses → `yes`, Obligation of the reviewer. |
| REL-37 | merged into REL-41 | A power to commence → `yes`, Liberty of that actor. |
| REL-38 | merged into REL-41 | A transitional provision that itself confers a time-limited power or imposes a time-limited duty → `yes`. |
| REL-39 | merged into REL-41 | A power to exempt → `yes`, Liberty. |
| REL-43 | merged into REL-33 | A duty with a qualifier stays a duty. |
| REL-44 | precedent `functions-list` | A functions list → `yes`; what is held follows holder class: a government body's functions are an Obligation (its responsibilities), a governed party's are a Liberty (rights). Provisions that only mention functions are n |
| REL-45 | retired (sentence units) | A stem that begins a duty or power ("X shall—", "X may … requiring the person to—", "A scheme … must—") → `yes`, holder active; the relation is counted here once for all its items. The source's closing words on the stem  |
| REL-46 | merged into REL-33 | A permitted way of discharging a duty ("X may inform … by a general announcement") creates its own Liberty: `yes`, X `active` Liberty, purpose Permissions. Detail that only binds (must/shall) still `continues` (REL-28). |
| HOLD-02 | retired (sentence units) | Stem holder: when the provision completes a sentence begun in its stem, the stem's holder is `active` here too. |
| HOLD-03 | merged into HOLD-05 | A provision that names its own holder (text or stem) ignores applying provisions. |
| HOLD-04 | retired (REL-28) | Referenced provision: when a relation-creating provision names its holder only in a provision it points to, that holder is `active` (`inferred: false`). Only a holder the referenced text actually names. |
| HOLD-06 | merged into HOLD-05 | Every applying provision whose scope covers the duty contributes its holder: there can be several. |
| HOLD-07 | merged into HOLD-05 | Applying form 2, supervisory: both the supervising party and the parties it must make comply are holders. |
| HOLD-08 | merged into HOLD-05 | Applying form 3, essential requirements (product regimes): the duty to meet the essential requirements applies every provision that sets them. |
| HOLD-09 | merged into HOLD-05 | Applying form 4, prohibition: the person barred unless compliant holds the Obligation. |
| HOLD-10 | merged into HOLD-05 | Referenced and applying provisions never create a relation: they only fill the holder of a provision that already creates one (not a detail, not relation `no`). |
| HOLD-11 | merged into HOLD-05 | An applying provision must name the party, and its scope must truly include this provision. A scope clause naming nobody gives no holder. |
| HOLD-12 | retired (REL-28) | Instrument-maker as holder: "the scheme/regulations may/must…" is the maker's Obligation/Liberty when the stem or a referenced provision names the maker; otherwise holder unknown. |
| HOLD-13 | merged into HOLD-01 | Applications to a court or tribunal: the court holds the Liberty; the applicant is `mentioned`. |
| HOLD-14 | merged into INF-03 | Holder in another instrument (#77): the same-law labeller leaves these holder unknown; a periodic agent pass proposes the holder and Jason approves it into the adjudicated tier. |
| HOLD-16 | merged into HOLD-01 | A provision can have several active holders. |
| HOLD-17 | precedent `no-person-shall-be-engaged` | "No person shall be engaged/employed/permitted to…": the engager holds the duty, from the applying provision (`active`, Obligation, `inferred: true`); the person engaged is `mentioned`; an applying holder who can't engag |
| TYPE-05 | merged into REL-42 | There is no `Rule` type: every "shall/must" that requires someone to act or bring about a state of affairs is an Obligation. |
| TYPE-06 | retired (sentence units) | Content-list and stem-completing items take the stem's type. |
| TYPE-07 | precedent `participation-right` | Participation: "shall take part in … or shall be consulted" gives the participants a Liberty (a right to participate), despite "shall". |
| INF-01 | precedent `implied-access-right` | Implied access right (#67): where a government actor's Obligation is to make something available for inspection/copying by, or to supply copies on request/payment to, a governed party named in the provision, that party i |
| INF-02 | precedent `implied-access-right` | Enforcement and notice-service provisions never give an implied right. |
| INF-04 | merged into INF-03 | `inferred` is `false` in every other case, including holders from the stem (HOLD-02) and referenced provisions (HOLD-04). |
| POS-01 | merged into HOLD-01 | `active` = the holder (from HOLD). |
| POS-03 | merged into POS-02 | A prohibited act aimed at a party makes it the counterparty. |
| POS-04 | merged into POS-02 | "Ensure that X is provided with…" is still a recipient duty → counterparty. "Ensure the health and safety of X" is protective → beneficiary. |
| POS-06 | merged into POS-19 | Beneficiary: the party whose interest the duty explicitly protects without receiving its act. |
| POS-07 | merged into POS-19 | The same kind of duty gets the same position. |
| POS-08 | merged into POS-02 | Recipient and protected → counterparty (recipient wins). |
| POS-10 | merged into REL-08 | A party exempted from scope is `mentioned`. |
| POS-11 | merged into POS-19 | Trigger-condition actors are `mentioned`, never beneficiary. |
| POS-16 | retired (POS-15 (revised, #78 option c)) | Deferred (#78; Jason 2026-10-06: not this iteration). Label every role, with no winner: one actor with two roles (incl. an Obligation and a Liberty) gets an entry per role; two persons under one label each get an entry. |
| POS-17 | retired (sentence units) | On a `continues` item, its own actors keep their role in the stem's relation: a recipient is `counterparty` with its act, a protected party `beneficiary`, otherwise `mentioned`; holds `none`. Their correlative points `to |
| POS-18 | retired (sentence units) | On a `continues` row of an access duty (#67), the governed party given access is `counterparty`, act `give_access`; its inferred Liberty stays on the row that creates the duty (INF-01). |
| ACT-02 | merged into ACT-01 | A passive duty's counterparty gets an act too, holder unknown or not. |
| ACT-03 | merged into ACT-12 | `notify`: told of an event or decision, incl. served with a notice (ACT-08 folded in 10-06). |
| ACT-04 | merged into ACT-12 | `supply`: given information, a copy or a thing. Needs text saying the party is provided with something (POS-13). |
| ACT-05 | merged into ACT-12 | `consult`: consulted. |
| ACT-06 | merged into ACT-12 | `pay`: paid. |
| ACT-07 | merged into ACT-12 | `give_access`: given access or inspection. |
| ACT-09 | merged into ACT-12 | `charge`: a charge levied on, or withheld from, the party. |
| ACT-10 | merged into ACT-12 | `answer_request`: the duty answers the party's request. |
| ACT-11 | merged into ACT-12 | `other`: an Obligation counterparty whose act fits none of the above, incl. withheld conduct (POS-12). |
| ACT-13 | merged into ACT-01 | Only an Obligation's counterparty carries an act; a right never does. |
| LBL-04 | merged into LBL-02 | A canonical label stands for the party the text denotes. |
| LBL-05 | dictionary (Gvt: Officer; Gvt: Authorised Person) | An officer, inspector or "authorised person" authorised by a government body is `Gvt: Officer`, never `Spc: Authorised Person`. |
| LBL-06 | dictionary (Spc: Authorised Person) | `Spc: Authorised Person` is a specialist authorised by the duty holder (governed). |
| LBL-07 | dictionary (Ind: Company Officer) | An officer of a body corporate (director, manager, secretary or similar officer) is `Ind: Company Officer`, not `Gvt: Officer`. |
| LBL-08 | dictionary (SC: Economic Operator) | "Economic operator" (manufacturer/importer/distributor umbrella) is `SC: Economic Operator`, not `Operator`. |
| LBL-09 | dictionary (Gvt: Devolved Admin: Scottish Ministers / Welsh Ministers) | "The Scottish Ministers" / "the Welsh Ministers" → `Gvt: Devolved Admin: Scottish Ministers` / `…: Welsh Ministers`, not the Parliament or Assembly. |
| LBL-10 | dictionary (Ind: Person in Control) | "The person having (the management and) control of…" → `Ind: Person in Control`. |
| LBL-11 | dictionary (Gvt: Judiciary) | A court or tribunal → `Gvt: Judiciary`. |
| LBL-12 | merged into LBL-03 | Instruments are never actors: scheme, regulations, order, notice, licence. |
| LBL-13 | merged into LBL-03 | Member State: an actor when it bears the duty/power or receives the act; a place (not listed) when it only names a jurisdiction or location. Same for countries. |
| LBL-14 | merged into LBL-03 | Things are never actors: the environment, animals, property, industries, countries. |
| LBL-15 | dictionary (dictionary type field (holder class)) | Holder class (government/governed) comes from the dictionary `type`, never the label prefix; the label choice therefore decides Duty vs Responsibility, Right vs Power. `HM Forces` is government. |
| DEF-01 | merged into REL-01 | Relation: no rule says `yes` and the provision has no own Obligation/Liberty → `no`. Be conservative. |
| DEF-02 | merged into HOLD-15 | Holder: no own, stem, referenced or applying holder → holder unknown: no actor `active`, `raw_type` set. |
| DEF-03 | merged into POS-09 | Position: an actor with no role in a relation created here → `mentioned`. No explicit protective purpose → not beneficiary. |
| DEF-04 | merged into TYPE-03 | Holds: not `active` → `none`. |
| DEF-05 | merged into INF-03 | Inferred: `false` unless INF-01 or INF-03. |
| DEF-06 | merged into ACT-01 | Act: `null` unless an Obligation counterparty; `other` if that counterparty's act fits no named value. |
| DEF-07 | merged into LBL-01 | Label: no dictionary fit → `OTHER: …`; in the gold set that row is a new edge (candidate dictionary entry). |
| DEF-08 | merged into REL-04 | Immunity-shaped text → relation `no` until #70. |
