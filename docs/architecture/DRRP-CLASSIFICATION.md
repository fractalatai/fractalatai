# DRRP Classification Schema

The authoritative definition of how fractalaw classifies legal relations in UK/EU legislation, and how that data becomes DRRP (Duties, Rights, Responsibilities, Powers) and a law-level verdict in sertantai-legal.

- **Status:** agreed 2026-09-29 (Jason; reviewed by sertantai-legal). Tracks fractalatai #68.
- **Owners:** fractalaw (layers 1–5, payload); sertantai-legal (expansion and `is_making` resolution) links here.
- **Rule of use:** any change to classification behaviour updates this document first. Code and tests follow it.

## The five layers

```
provision text
   │  per actor
   ▼
1. Hohfeldian type ── what THIS actor holds: Obligation | Liberty | none
2. Position ──────── is this actor the holder? active | counterparty | beneficiary | mentioned
3. Holder class ──── government | governed (actor dictionary `type`)
   │  active actors only
   ▼
4. DRRP ──────────── Duty | Responsibility | Right | Power          (per provision)
   │  over the law's in-scope provisions
   ▼
5. Verdict ───────── making | empowering | no_obligations | (no verdict)   (per law)
```

### Layer 1: Hohfeldian type (per actor)

The type is **what this actor holds** in this provision. It is not the provision's type seen from the actor's side.

| Value | Meaning |
|---|---|
| `Obligation` | this actor bears a duty or prohibition ("must", "shall", "shall not", "no person shall") |
| `Liberty` | this actor holds a power, permission or entitlement ("may", "is entitled to"), including **implied rights** (see special cases) |
| `none` | this actor holds nothing here (e.g. it is only referred to) |

There is **no `Rule` type** (removed 2026-09-30). Every "shall"/"must" that requires someone to act or to bring about a state of affairs is an `Obligation`, even when the text names no one:
- **Passive and thing-subject duties** ("records shall be kept", "equipment must be provided", "traffic routes must be suitable") are `Obligation`. Their holder comes from the stem, a referenced provision or an applying provision (special cases); otherwise it's unknown. A thing can't hold a duty, so these are never a separate type or a sub-type that blocks holder resolution.
- **Definitions, deeming, application and extent** ("'premises' includes…", "a notice shall be treated as served…", "this Part applies to…") create no relation, so they get no type. A positive provision-level label for them (constitutive provisions) is proposed separately as a provision-function field outside DRRP (#69).

**Exemptions are scope, not Liberty or Immunity (Jason, 2026-10-01).** Three things are easy to confuse:
- **Liberty** (Hohfeld's privilege) is an affirmative freedom the actor holds: "the employee *may* refuse…". The actor is `active` and holds a Right.
- **Immunity** is protection against someone else's *power*. The other party lacks the power over this actor (Hohfeld's correlative: disability). Examples: "the Crown *shall not be liable* to prosecution", "no improvement notice *may be served* on…".
- **An exemption** ("these Regulations shall not apply to an employee") is neither. It gives the employee nothing and blocks no power; it takes the employee out of the duty's **scope**, so the duty never reaches them. It's the negative form of "this Part applies to…": a scope (constitutive) provision, relation `none`, and the exempted party is `mentioned`.

**Immunity is deferred (#70, Jason 2026-10-01).** It isn't in this model version. Genuine Immunity provisions ("shall not be liable", protection from enforcement or prosecution, possibly statutory defences) are rare in ESH law and findable by their wording. Adding Immunity later means a targeted relabel of those provisions only, not a re-run. Until then they're `none`.

**Non-active actors hold nothing.** An actor whose position isn't `active` has type `none`, whatever the model predicted. `provision_actors.drrp` keeps the tier's reading (the provision's type from that actor's side), which is used only as the raw type when the holder is unknown. The payload and the roll-up expose `none`.

**Source and precedence** (reconcile, `fractalaw-cli/src/commands/taxa.rs`):
- **Type:** adjudicated > LLM > inferred-active > SLM > regex. An inferred type counts only when the inference also makes the actor active.
- **Position:** adjudicated > LLM > inferred > SLM (≥0.9 conf) > regex/classifier agree > classifier (≥0.7) > `pending_slm` > regex.
- The final values are `provision_actors.drrp`, `provision_actors.position` and `provision_actors.extraction_method`.

### Layer 1b: Correlatives (per actor, against each other active holder) (#72, agreed 2026-09-30)

**Correlatives** record what an actor holds *because of* another active holder's Obligation or Liberty in the same provision. They are computed for each (actor, other active holder) pair. Layer 1 records only what an actor itself holds, so without this layer, "what are my rights?" misses the duties owed to that actor.

| This actor's position | Active holder holds | Holder class | Correlative |
|---|---|---|---|
| `counterparty` | Obligation | any | `claim_right`: the duty is owed to this actor |
| `counterparty` | Liberty | government (Power) | `liability`: exposed to the power |
| `counterparty` | Liberty | governed (Right) | `no_right`: cannot prevent it |
| `beneficiary` | Obligation | any | `protected`: the duty protects this actor (Jason, 2026-09-30) |
| `beneficiary` | Liberty | any | none |
| `mentioned` | any | any | none |
| any | none (holder unknown) | — | none |
| `active`, Liberty inferred by the #67 access rule | Obligation (the access duty) | government | `claim_right` → the duty holder |

**Scope.** Non-active actors use the table by their position. An active actor gets a correlative only in the #67 case: the governed party given an inferred access Liberty also holds a `claim_right` against the government actor whose access Obligation it is. No other active actor gets a correlative, e.g. two co-holders of the same Obligation don't get correlatives against each other.

- **Derived, not predicted.** Backfill computes correlatives from the reconciled positions (layer 2), the holder class (layer 3) and `relates_to`. There is no model tier. Legal stores them as received.
- **A list of pairs.** Each item is `actors[].correlatives = [{type, to}]`, where `to` is the active holder's label.
  - When `relates_to` names the holder, there is one pair.
  - Otherwise there is one pair per active holder, so a counterparty to a government Power and a governed Right gets both `liability` and `no_right`.
  - An actor with no correlative sends `[]`, never NULL.
- **The act (#75, Jason 2026-10-01).** Each counterparty's correlative can carry what it's owed, from a fixed list:
  - `notify`: told of an event or decision, including being served with a notice (RIDDOR reg.4, the enforcing authority). `serve` was folded into `notify` (Jason, 2026-10-06);
  - `supply`: given information, a copy or a thing (MHSWR reg.10, employees);
  - `consult`: consulted (safety representatives);
  - `pay`: paid;
  - `give_access`: given access or inspection ("the register shall be available for inspection by the operator");
  - `charge`: a charge levied on, or withheld from, the party (HSWA s.9);
  - `answer_request`: the duty answers its request;
  - `other`.
  - Item shape: `{type, to, act}`; `act` absent means unknown.
  - **Only an Obligation's counterparty carries an act** (Jason, 2026-10-06). A right doesn't: it mirrors the duty's act. Under the #67 access rule the governed party stays active with the inferred Liberty and its `claim_right` (one entry per label), so its `give_access` act isn't recorded until #78.
  - **Synonym mapping** (Jason, 2026-10-06): the text's verb maps onto the fixed classes; new verbs never create new classes. The duty's main verb decides ("serve a notice on X informing it" → `notify`).

    | Act | Verbs in the text |
    |---|---|
    | `notify` | notify, inform, give notice, serve (a notice), report to, warn, advise |
    | `supply` | supply, provide, furnish, send, deliver, issue (a certificate), give a copy |
    | `consult` | consult, seek the views of, invite representations |
    | `pay` | pay, compensate, reimburse, refund |
    | `give_access` | permit to inspect, afford access, make available for inspection, allow copies to be taken |
    | `charge` | charge, levy, impose a fee ("shall not charge" too) |
    | `answer_request` | respond to, reply to, determine an application |
    | `other` | anything else, incl. withheld conduct |

    The table becomes a data file for the regex act tier (#75, phase 2). Stored labels with `serve` (126) are read as `notify`; the raw rows are not rewritten.
  - **A passive duty's counterparty also gets an act** (Jason, 2026-10-05): "notice shall be given to the operator" → operator counterparty, act `notify`, even though the holder is unknown. The recipient receives the act either way.
  - **Labelled now:** the definitive prompt labels `act` on each counterparty. Regex derives it for new laws from the clause verb, measured against those labels. It isn't an SLM target.
  - **Published later:** once compliance and legal agree it on #75. It never feeds DRRP or the verdict.
- **Never feeds layers 4–5.** DRRP types, the holder lists, the verdict and `is_making` stay based only on held types.
- **`claim_right` is the duty's direct correlative, in a broad public-law sense** (Gemini, #72): it doesn't by itself imply a private right of civil action. Many regulatory duties aren't actionable (e.g. HSWA s.47 for ss.2–8). User-facing text should present it as "owed to you", not "you can sue".
- **`protected` is a compliance category, not Hohfeldian.** A beneficiary isn't the direct correlative party. Many "owed to" duties give no enforceable claim either: HSWA s.47 excludes civil action for breach of ss.2–8. `protected` answers "what protects me".
- **Implied rights (#67) stay.** "Available for inspection by the public" gives the Public an inferred **Liberty** (active → Right: it may inspect) *and* a `claim_right` (the authority owes it an available register). These are two relations. Only the Liberty counts in DRRP (Jason, 2026-09-30).
- **Query semantics.**
  - "My obligations" = active Obligation (Duty/Responsibility).
  - "My rights" = active Liberty (Right) ∪ `claim_right`.
  - "My protections" = `claim_right` ∪ `protected`.
  - "What can be imposed on me" = `liability`.
- **Law level.** `claim_holder`, `liability_holder` and `protected_holder` list who holds each correlative, beside `duty_holder` and the others: non-active actors, plus #67-inferred actors for `claim_holder`. They are never counted in the verdict.
  - **Both holder classes appear.** For example, a regulator owed a notification holds a `claim_right`. The never-cross-assign rule (layer 4) applies only to DRRP holder lists, not to these.
  - **Holder lists only.** There are no entry lists with clauses (like `duties`/`rights`) for now.
- **Prerequisite.** The counterparty/beneficiary split must be consistent: HSWA s.2(1) (employees) and s.3(1) (persons not employed) are both beneficiary (layer 2, 2026-10-01). Check reconcile/SLM positions for this pattern before trusting the split.
- **Rollout order.**
  1. Fix the counterparty/beneficiary consistency (prerequisite).
  2. Fractalaw code and tests: backfill derivation and the payload fields.
  3. Legal adds three `legal_register` columns and the actor pass-through, with tests, and confirms.
  4. First publish.

### Layer 2: Position (per actor)

| Value | Meaning | Holds a type? |
|---|---|---|
| `active` | bears the duty or exercises the liberty: **the holder** | yes |
| `counterparty` | to whom the duty is owed, or subject to the power | no |
| `beneficiary` | benefits, but is neither the holder nor the direct correlative | no |
| `mentioned` | referred to, with no legal role | no |

**Counterparty vs beneficiary for a duty (#72, agreed 2026-10-01: Jason, Gemini, sertantai-legal).** The test is the duty's **act**, not who gains from it:
- **`counterparty`** = the **recipient** of the duty's act: the party the act is **done to or withheld from**. That covers notified, informed, sent or supplied something, consulted, paid, given access, served, charged, or whose request the duty answers. The duty runs *to* them → layer 1b `claim_right`.
  - **Prohibited acts aimed at a party** count: "No employer shall levy … any charge on **any employee**" (HSWA s.9) → the employee is the counterparty (legal).
  - **"Ensure that X is provided with …"** is still a recipient duty: "every employer shall ensure that suitable PPE is provided to **his employees**" (PPE Regs reg.4) → counterparty. The word "ensure" doesn't make it a protected-interest duty (legal).
  - "The operator must notify **the authority**"; "OFCOM must send a copy to **the applicant**"; "the employer shall provide information to **the employee**".
- **`beneficiary`** = the party whose **interest the duty protects**, without being the recipient of its act → layer 1b `protected`.
  - "It shall be the duty of every employer to ensure … the health, safety and welfare at work of all his **employees**" (HSWA s.2(1)); "… that **persons not in his employment** … are not exposed to risks" (s.3(1)).
  - Both are beneficiaries: the same kind of duty gets the same position.
- **Both at once:** an actor that is both the recipient and the protected party is a `counterparty`. The recipient test wins: "provide **employees** with information, instruction and training" (HSWA s.2(2)(c)).
- **Bodies with no role in the relation** (e.g. a regulator merely named in a landlord's duty) are `mentioned`, not counterparty.

**Why:** the SLM split HSWA s.2(1) (Employee = counterparty) and s.3(1) (Person = beneficiary), the same kind of duty. On Obligation provisions, protective-wording duties were split 177 counterparty vs 244 beneficiary, and 683 beneficiaries carried recipient wording.

**Reviews:** Gemini agrees with changes (`data/code-review/drrp-counterparty-beneficiary-gemini.md`): it checked duties to the public, to an authority, consultation, duties of care, access, courts, prohibitions ("must not disclose to anyone other than the worker": the worker is a beneficiary) and payment, and agrees the recipient wins the tie-break. Legal agrees (no change to its code; its consistency check maps position → correlative type, so it isn't affected) and added the two edge cases above. Legal also offered an optional extra check: each pair against its `to` holder's type.

**Fix path:** the definitions go into the SLM/LLM position prompts now. Positions are corrected when the re-enrichment backlog re-runs those tiers. Correlatives are derived from positions, so they follow.

### Layer 3: Holder class

Defined by the **actor dictionary** (`crates/fractalaw-core/data/actor-dictionary.yaml`, field `type`), never by label prefix.

| Class | Members |
|---|---|
| **government** | every entry with `type: government`: all `Gvt*`, all `EU:*`, `Crown`, `HM Forces`, `Spc: Notifying Authority` (2026-09-30). `Spc: Authorised Person` moved to governed on 2026-10-05: a person authorised by the duty holder. Government-authorised officers are `Gvt: Officer`. |
| **governed** | every entry with `type: governed`: `Ind:*`, `Org:*`, `SC:*`, `Spc:*` (except Notifying Authority), `Svc:*`, `Public*`, `Offshore*`, … |

`HM Forces` is government, as the Crown's forces (Jason, 2026-09-29), which matches sertantai-legal's `ActorDefinitions.government_label?/1`. A label-prefix check misses `Crown`, `HM Forces` and `Spc: Notifying Authority`, so code must read the dictionary `type`.

### Layer 4: DRRP (per provision, active actors only)

| Type held (layer 1) | Governed holder | Government holder |
|---|---|---|
| Obligation | **Duty** | **Responsibility** |
| Liberty | **Right** | **Power** |

- **Never cross-assign:** a government actor never holds a Duty or Right, and a governed actor never holds a Responsibility or Power.
- Each **active** actor contributes the DRRP its own type and class give. A provision can carry several, e.g. Responsibility (authority) + Right (public).
- **Provision `drrp_types`** = the union of the **active** actors' own types.
- **No active actor = holder unknown.** The provision's types stay raw `Obligation` / `Liberty` (the union of all actors' types) and are **not** expanded to DRRP. That's the same treatment as a provision with no actors at all. Nothing is guessed.

### Layer 5: Law-level verdict

Rolled up over the law's provisions, **excluding** amendment scope (inserted text, #57) and scoped `enabling_extent` LAT (#66). Only active actors' DRRP counts.

| Verdict | Condition |
|---|---|
| **making** | at least one Duty or Responsibility |
| **empowering** | Rights and/or Powers only |
| **no_obligations** | parsed and reconciled, with no DRRP and no holder-unknown Obligation; **or** zero actors and no duty text in any substantive provision (evidenced, provenance model `fractalaw-law-drrp:no_duty_text`) |
| *(no verdict)* | actors not yet reconciled; **or** zero actors but duty text present (actor gap, #58/#60); **or** no Duty/Responsibility but at least one substantive provision has an Obligation with no known holder (it may be a Duty); **or** the LAT is `enabling_extent` |

A holder-unknown Obligation never makes a law Making, and it also stops a law being called empowering or no_obligations. That no-verdict case has an explicit payload shape (see the payload contract), not NULL.

Law-level holder fields (`duty_holder`, `rights_holder`, `responsibility_holder`, `power_holder`, `duties`, `rights`, `responsibilities`, `powers`) list **who holds** each DRRP, which means active actors only.

## Special cases

| Case | Rule |
|---|---|
| **Offences and penalties** | "commits an offence", "is liable on conviction to…" → `none`. A penalty is not a duty; the duty is in the provision the offence refers to. |
| **Cross-references, commencement lists, conditions** | A provision that only references, commences, conditions, details, defines or exempts a relation created elsewhere → `none`. |
| **Amendment (inserted) text** | Belongs to the amended instrument; excluded from every layer (`scope = amendment`, #57). |
| **Holder unknown** | Obligation/Liberty text with no actors, or no active actor → raw type, no DRRP. Holders are filled only from the stem, a referenced provision or an applying provision (rows below). Never from a guessed default such as the law's dominant actor (Jason, 2026-10-05). |
| **Statutory defences** | "It is a defence for an accused … to prove that …" → relation none: it qualifies the offence (offences are none). Possibly Immunity later (#70, deferred). (Jason, 2026-09-30) |
| **Holder named in a referenced provision** | When the provision points to a specific provision that names the holder ("regulations under subsection (2) may prescribe…", "a power under this section may be exercised by force"), that holder is the active holder here. The target is fractalaw's holder resolution (#60); the gold v2 benchmark labels to this standard. (Jason, 2026-09-30) |
| **Holder named in an applying provision** (#60) | A passive or thing-subject duty ("effective and suitable provision shall be made to ensure that every enclosed workplace is ventilated…") whose holder is named by **another provision that applies it**: one that puts a named party under a duty to comply with, or ensure compliance with, the requirements covering this provision. Its scope can be explicit ("regulations 5 to 27", "Schedule 2") or instrument-wide ("any requirement of these Regulations", "this Part"). That party is `active`, holds the Obligation, and is marked `inferred: true` (`extraction_method = inferred`). Each applying provision whose scope covers the duty contributes its holder, so there can be several (e.g. the employer, and the person in control of the workplace). The applying provision must name the party; a scope clause that names none ("these Regulations apply to every workplace") gives no holder. Otherwise holder unknown. Forms: (1) "X shall comply with / ensure that [a thing] complies with the requirements of SCOPE"; (2) **supervisory**: "the principal contractor must take all reasonable steps to ensure that contractors … comply with the duties under these Regulations": **both** the supervising party and the persons it must make comply are holders, because the law requires the supervisor to secure the duty as well; (3) **essential requirements** (product regimes): "a manufacturer must ensure that it has been designed and manufactured in accordance with the essential (health and safety) requirements" applies every provision of the law that sets those requirements ("The equipment shall meet the essential requirements set out in Annex I"); (4) **prohibition**: "no person shall use … unless it complies with SCOPE": the person holds the Obligation (a prohibition). (Jason, 2026-10-05) **The applying provision's own row is a pointer** (Jason, 2026-10-06): "X shall comply with / ensure compliance with the requirements of SCOPE" (forms 1 and 3) is relation `none` on its own row, and X is `mentioned` there. X's Obligation is recorded once, on each provision it applies. Forms 2 and 4 are unchanged: the supervisory duty and the prohibition are duties in their own right. |
| **Holder named in another instrument** (#77) | A duty in secondary legislation whose holder is named by a duty in another Act, usually the parent Act ("a no-smoking sign must be displayed … in accordance with the duty at section 6(1) of the Act"). The labelling prompt is same-law only, so it labels these holder unknown or as a detail provision. A periodic Claude-agent pass (skill `cross-instrument-holders`) resolves "the Act" via DuckDB `enacted_by` or the instrument's own definitions, reads the parent provision, and proposes the holder. Jason approves; approved holders are written to the **adjudicated** tier (`adj_note` names the parent provision). (Jason, 2026-10-05) **Standard:** the holder comes from the parent Act whenever it can be resolved (`active`, `inferred: true`), and is holder unknown only when it can't (Jason, 2026-10-06). Gold labels follow this, so the #77 pass runs on the gold laws before they are justified. |
| **Headless stems** (Jason, 2026-10-05) | "X shall—" or "X may—" with the substance in its items: the stem itself is relation `yes` with X active (Obligation / Liberty) and purpose Requirements / Permissions, like the content-lists row. Its items `continue` it (row "Stems and list items: counted once"). It's never `Procedure+Detail` just because it has no object of its own. |
| **Transitional powers and duties** (Jason, 2026-10-05) | A transitional provision that itself confers a time-limited power or imposes a time-limited duty ("until 1 January 2020 a Member State may continue to authorise…") is relation `yes` with that Liberty / Obligation, purpose `Transitional Arrangement`. Pure savings and continuity ("as if this Act had not passed", "shall continue to have effect") stay relation `none`. |
| **Member State: actor or place** (Jason, 2026-10-05) | `EU: Member State` is an actor when it bears the duty or power ("Member States shall ensure…") or receives the act. Where it only names a jurisdiction or location ("the laws in force in the Member State", "placed on the market in a Member State"), it's a place: not an actor, so it isn't listed. The same goes for countries ("third countries"). |
| **Trigger-condition actors** (Jason, 2026-10-05) | A party named only in a condition or trigger ("having regard to the risks to end-users", "where the vessel presents a risk to persons") is `mentioned`, not a beneficiary. A beneficiary needs the duty's own protective purpose. |
| **Commencement powers** (Jason, 2026-10-05) | "On such day as X may by order appoint": X holds a Liberty (relation `yes`), purpose `Enactment+Citation+Commencement`. Relation `yes` despite being machinery, like transitional powers. |
| **Laying before Parliament or an Assembly** (Jason, 2026-10-05) | A duty to lay a report, direction or copy before Parliament or an Assembly is relation `yes`: a government Obligation. "Parliamentary procedure" (relation `none`) means only annulment/affirmation clauses ("subject to annulment…", "may not be made unless a draft has been laid and approved"). |
| **Enforcing-authority designations** (Jason, 2026-10-05) | "X shall be (responsible as) the enforcing authority" is relation `none`, purpose `Establishment+Constitution`. (Functions lists moved to the row below on 2026-10-06.) |
| **Class-definition items** (Jason, 2026-10-05) | An item that only defines which class of persons or things a duty covers is relation `none`, under any duty, not only in content lists. The relation stays on the provision above. |
| **Time-limited disapplications** (Jason, 2026-10-05) | Purpose `Exemption` (machinery order: Exemption before Transitional Arrangement). |
| **Money provided by Parliament** (Jason, 2026-10-05) | "There shall be paid out of money provided by Parliament…" is an authority to spend: relation `none`, purpose `Charge+Fee`. |
| **One actor, two roles; two persons, one label** (Jason, 2026-10-05; confirmed 2026-10-06) | One entry per label, strongest role: active > counterparty > beneficiary > mentioned. This applies when one actor has two roles in a provision (e.g. receives a notification and may shorten the period), and when two persons share a label ("an employer shall notify another employer": Employer active/Obligation only). **Known loss, not fixed in this iteration (#78):** the second role, its act and its correlative are dropped. That includes an actor holding both an Obligation and a Liberty, and the #67 party's `give_access` counterparty entry. |
| **Functions lists** (Jason, 2026-10-06; replaces the 2026-10-05 "functions lists are `none`" rule) | A list of functions conferred on a party is relation `yes`. **What the party holds follows its holder class (layer 3):** a **government** body's functions are its responsibilities, so an Obligation ("The functions of the CAA shall be—", "OFCOM shall have the following functions—"). A **governed** party's functions are rights, so a Liberty ("Each safety representative shall have the following functions— … but no function … shall be construed as imposing a duty on him"; a safety committee's functions). Provisions that only *mention* functions ("references to the functions of the MMO are to…", "the functions of the Authority are performed on behalf of the Crown") are not functions lists: relation `none`. Purpose waits for the purpose refactor. |
| **Content list vs detail** (Jason, 2026-10-06) | "The notice/report/register … must— (a)…": a content list (relation `yes`) if it completes a duty created in the same provision; detail (relation `none`, Procedure+Detail) if it details something required elsewhere ("the notice referred to in paragraph (1) must contain"), including enforcement notices. **Superseded 2026-10-06 for duties:** relation `continues` (row "Detail of a duty continues it"). |
| **Timing items** (Jason, 2026-10-06) | "At suitable intervals", "within 8 weeks": detail, relation `none`, Procedure+Detail. **Superseded 2026-10-06 for duties:** relation `continues` (row "Detail of a duty continues it"). |
| **Appeal and inquiry procedure** (Jason, 2026-10-06) | Purpose `Defence+Appeal` when the provision concerns an appeal or review of a decision; relation `yes` only where it creates its own duty or power. |
| **Powers to exempt** (Jason, 2026-10-06) | "May by certificate exempt": relation `yes`, Liberty, purpose `Exemption`. Relation `yes` despite being machinery, like commencement powers. |
| **Conditions on another provision's power** (Jason, 2026-10-06) | "Shall not grant … unless satisfied", "shall not consent unless—": detail, relation `none`, Procedure+Detail. |
| **Savings of old law or status** (Jason, 2026-10-06) | "Continues to have effect as if", evidential reliance on old documents: `Transitional Arrangement`, relation `none`, ahead of the listed machinery order. Pure deeming with no time element stays `Interpretation+Definition`. |
| **Discharge details** (Jason, 2026-10-06) | "Shall comply with his duty under paragraph (1) by…": detail, relation `none`, Procedure+Detail. **Superseded 2026-10-06 for duties:** relation `continues` (row "Detail of a duty continues it"). |
| **Mixed provisions** (Jason, 2026-10-06) | A provision that creates any duty or power of its own (e.g. an exemption with a fallback duty) is relation `yes`; its purpose follows that operative part (`Requirement` / `Power Conferred`). |
| **Withheld conduct** (Jason, 2026-10-06) | "Ensure workers do not eat…", "shall not be permitted to remain": the parties whose conduct is permitted or withheld are `counterparty`, act `other`. |
| **Class-definition items: purpose** (Jason, 2026-10-06) | A relation-`none` class-definition or criterion item takes purpose `Application+Scope`. |
| **Notification inside commencement** (Jason, 2026-10-06) | "Such date to be notified in the Gazette" within a citation/commencement provision: machinery, relation `none`, `Enactment+Citation+Commencement`. |
| **Electronic delivery** (Jason, 2026-10-06) | "Has effect as a delivery only if": deeming, relation `none`, `Interpretation+Definition`. |
| **Designation inside a definition** (Jason, 2026-10-06) | "As may be designated by the Secretary of State by order" inside a definition: relation `none`, `Interpretation+Definition`. Only a provision that itself confers the designating power is a Liberty. |
| **"Means at their disposal"** (Jason, 2026-10-06) | "Ensure nominees have adequate means": the nominees are `mentioned`. A counterparty with `supply` needs the text to say the party is provided with something. |
| **Stems and list items: counted once** (Jason, 2026-10-06; replaces item-level repetition, 2026-09-30) | A duty or power begun in a stem ("X shall—", "X may serve a notice … requiring the person to—", "A scheme under this section must—", "regulations may—") is counted **once, on the stem row**: relation `yes`, holder active (or holder unknown, with a raw type). An item that only completes it is relation **`continues`**: it creates nothing new, so it has no holder and no DRRP of its own. **Its own actors keep their roles in the stem's relation:** a party it notifies, supplies or consults is `counterparty` with its act, a protected party is `beneficiary`, others are `mentioned`. The stem's holder isn't repeated on the item. An item that creates its own duty or power ("(c) the employer shall…") is `yes`. Criterion and class-definition items stay `none` (row "Class-definition items"). Where the source puts the stem's lead-in and closing words ("… as it may reasonably require …") on the stem row, both belong to the stem. **Pipeline consequence:** an item's correlatives point `to` the stem's holder (backfill resolves the stem). It's built with the single run. Holder lists and verdicts don't change, because the stem keeps the holder. |
| **Detail of a duty continues it** (Jason, 2026-10-06) | A provision that only sets the **content, form, manner, timing or discharge** of a duty created in another provision is relation **`continues`**: the duty is counted once, where it's created. Examples: "The measures required by paragraph (1) shall consist of—", "The copy … shall be indexed and arranged so that members of the public can readily trace…", "The person's entry must set out—", "within 8 weeks", "shall comply with his duty under paragraph (1) by…". As for list items, it has no holder of its own, the duty's holder isn't repeated, and its own actors keep their role in that duty. **"No" now means the provision touches no duty or power at all:** definitions, offences, application and exemption, citation, criterion items, savings. Conditions and limits on **a power** created elsewhere ("shall not grant … unless satisfied", "a deficit notice may not be given—") stay `none` for now. Closing words the source puts on a stem row (PUWER reg.11(2), "and the provision of such information, instruction, training and supervision as is necessary") are the law's own drafting, not a parse error. |
| **"No person shall be engaged/employed/permitted to…"** (Jason, 2026-10-06) | The duty is on whoever engages, employs or permits: the parties an applying provision makes responsible (e.g. EAW 1989 reg.3: employer, self-employed person; `active`, Obligation, `inferred: true`). The person engaged is `mentioned`. An applying holder who can't do the engaging (EAW: an employee) isn't listed. |
| **Participation** (Jason, 2026-10-06) | "Workers or their representatives shall take part in a balanced way … or shall be consulted" (Directive 89/391 Art.11(2)) gives the workers and representatives a **Liberty** (a right to participate), despite "shall"; the employer holds the Obligation to consult. Under one entry per label, the workers' active Liberty outranks their counterparty role. |
| **Content lists of schemes, regulations, notices** | "A scheme under this section must— (a) …" and "regulations may— (a) …": the stem holds the Obligation or Liberty, and its items `continue` it (row above). The holder is the scheme or regulation maker, resolved as in the previous row, or holder unknown if none is named. (Jason, 2026-09-30; items counted once 2026-10-06) |
| **Details inside a content list** | Below a content-list item, a sub-item that only states an eligibility criterion or defines a class of persons is a detail → none. The Obligation/Liberty stays on the item above. (Jason, 2026-09-30) |
| **Detail provisions** | A provision that only sets the **form, manner, discharge, conditions or procedure** of a relation created in another provision → `none`. The duty or power is counted once, where it's created. Examples: "The information referred to in paragraph (1) must—" (form of a duty in (1)); "where the authority have not so informed the applicant, at any time after …" (condition or timing); "which are required to be kept under these Regulations, or" (list fragment). The *holder named in a referenced provision* rule applies only to provisions that create a new relation. **Parliamentary procedure** ("subject to annulment in pursuance of a resolution of either House", "may not be made unless a draft has been laid before and approved by …") → `none`. **Exception: a deadline that compels a government actor to act** ("The first … regulations must come into force no later than 1 April 2018") is an Obligation of the regulation maker → Responsibility. *Content lists of schemes, regulations and notices* (above) are unchanged: their stem creates the requirement in its own text. (Jason, 2026-10-01; gold v2 batch 1, 89 relation disputes) **Superseded 2026-10-06 for duties:** relation `continues` (row "Detail of a duty continues it"). |
| **Exemptions** | "These Regulations shall not apply to …", "nothing in this Part applies to …" → `none`; the exempted party is `mentioned`. A scope clause, not a Liberty or an Immunity (see layer 1). (Jason, 2026-10-01) |
| **Instruments are never actors** | A scheme, regulations, order, notice or licence is not an actor. "The scheme may specify…" is a Liberty of the scheme maker when resolvable, otherwise holder unknown. (Jason, 2026-09-30) |
| **Applications to a court or tribunal** | "On the application of X, the court may…" → the court holds the Liberty (Power); X is mentioned (its application is a condition). (Jason, 2026-09-30) |
| **Commencement and citation** | A short title ("This Act may be cited as …") and a commencement list are none. A power to commence ("on such day as the Scottish Ministers may by order appoint") is a Liberty of that actor (a Power). (Jason, 2026-09-30) |
| **Procedural time limits** | "Proceedings may be commenced within 6 months …" → none: a limitation on proceedings, not a liberty anyone holds. (Jason, 2026-09-30) |
| **Implied rights** | Where a government actor's active Obligation grants a governed party access (inspection by the public, facilities for copies, supply on request/payment), the governed party named in the clause gets an **inferred Liberty, active**, so a Right. Depends on the wording: enforcement or notice-service provisions never qualify. Marked `extraction_method = inferred` (#67). **Also when the access duty's holder is unknown** (Jason, 2026-10-06): "the register shall be open to inspection by the public" still gives the public its inferred Liberty, even though no duty holder is named. |
| **Scoped LAT** | `enabling_extent`: never classified. `relevance`: classified within the scope; the verdict is scope-relative, and provenance carries `lat_coverage` (#66). |
| **Revoked laws** | Status never changes classification: revocation doesn't affect what the law did while it was live, so a revoked Making law stays Making and is never re-marked non-Making because it is revoked (Jason, 2026-09-28). This is **not** a reason to re-process dead laws: their existing verdict stands, and no earlier text is fetched for them (Jason, 2026-09-30). |
| **Human adjudication** | The `adjudicated` tier (`provision_actors.adj_drrp`/`adj_position`/`adj_note`) is the top source tier: reconcile never overrides it and `taxa infer` never deletes its rows. First used for benchmark gold labels carried across a LAT sync (`scripts/benchmarks/carry_forward_gold.py`). |

## Payload contract (fractalaw → sertantai-legal)

**Provision payload** (`taxa/provisions/{law}`, from `legislation_text`):
- `drrp_types`: the union of the active actors' types, or raw types when the holder is unknown (layer 4).
- `actors[]`: `{label, position, drrp, reason (= extraction_method), label_source, relates_to}`.
  - `drrp` is the actor's own layer-1 type;
  - non-active actors carry `none`;
  - legal types DRRP from active actors only.
- Legal expands to DRRP per active actor using its holder class. With no active actor, it keeps the raw type (holder unknown).
- `actors[]` also carries `correlatives: [{type, to}]` (layer 1b). The field is always present: `[]` when there are none, never NULL or absent.
- **Every row of an enriched law is sent, and `drrp_types`/`actors` are never NULL** (legal reads NULL as "not in this payload" and keeps stale values):
  - a provision with no actors sends `actors = []`;
  - amendment text (#57) sends `drrp_types = []`, `actors = []`;
  - an unclassified row (scope `out`, whatever method an older pipeline left on it, or not yet parsed) sends `drrp_types = []`, `actors = []`, `extraction_method = null`. That means "not classified", not "classified as having no type".

**Law payload** (`taxa/enrichment/{law}`, from DuckDB `legislation`): the layer-5 holder fields, significance, fitness and application fields, plus `provenance` (#63). The DRRP section has three shapes:

| Shape | Meaning to legal |
|---|---|
| holder lists and `duties`/`responsibilities`/`rights`/`powers` populated or empty, `duty_type` = DRRP types | the verdict (making / empowering / no_obligations) |
| `duty_type` keeps the raw `"Obligation"` beside any known Right/Power types (e.g. `["Obligation","Liberty"]`); `duties`, `responsibilities`, `duty_holder`, `responsibility_holder` = `[]`; `rights`, `powers` and their holder lists populated when known, `[]` when none | **holder unknown, no verdict**: legal overwrites stale DRRP, keeps the known Rights/Powers and the unowned Obligation, and sets `making_enrichment_verdict = NULL` (legal e36161b, 03f0154) |
| every DRRP column NULL | no DRRP in this payload: legal keeps what it has (e.g. a fitness-only publish) |

Never send NULL to clear a verdict: legal can't tell it apart from "not in this payload".

**Current view (fractalatai #73, R1a, agreed 2026-10-01).** The law payload also carries the as-amended view. The existing fields above stay the **whole-law** view and feed `is_making`.
- **Whole-law view (Jason, 2026-10-01):** the existing fields mean "everything the law imposed as far as our text shows, including later-repealed provisions": the roll-up over all held provisions. They aren't computed from made text (#73 R5/R6 dropped).
- `is_making` is the law's **character** (it imposes substantive requirements, as against only commencing, amending or revoking). Repeal or revocation never removes it, and amendment should rarely remove it.
- **Smell list:** a live, amended law whose whole-law verdict is `no_obligations` or `empowering` is suspect: it may have been making, or its effects repeal its own provisions. The backfill dry run flags these, and legal puts them to Jason. Made text is fetched only per law, with Jason's approval (legal's exception path).
- `current_verdict`: `making | empowering | no_obligations | revoked | holder_unknown`. `revoked` means legal's LRT `live` says the whole law is revoked.
- `current_duty_type`.
- `current_duty_holder`, `current_rights_holder`, `current_responsibility_holder`, `current_power_holder`: over live provisions only (status not repealed/prospective, and not an in-force unapplied repeal).
- Same shapes as above: `[]` when empty in a DRRP-carrying payload; never NULL to clear.
- Legal's resolver never reads `current_verdict` for `is_making`.

**Correlative holder lists (layer 1b).** The law payload's DRRP section also carries `claim_holder`, `liability_holder` and `protected_holder`, in the same format as `duty_holder` (a list of labels).
- They are `[]` in any payload that carries the DRRP section.
- They are NULL only in the "no DRRP in this payload" shape.
- They never feed the verdict.

**The verdict is one input to legal's `is_making`, not the final word.** sertantai-legal's `Legal.Making` resolves in this order: human review > enrichment (this verdict) > legacy DRRP > triage > legacy flag > detector > default. E.g. UK_uksi_2008_198 and UK_uksi_2014_2868 stay Making by human review despite a `no_obligations` verdict. Legal's "Housekeeping" corresponds to `no_obligations`. Legacy law-level `duty_type = Obligation` (pre-DRRP) counts as holder unknown, not as Making.

## Worked examples

| Provision | Actors (type/position) | DRRP | Correlatives (layer 1b) |
|---|---|---|---|
| EPA 1990 s.20(7): "It shall be the duty of each enforcing authority— (a) to secure that the registers … are available … for inspection by the public" | Enforcement authority Obligation/active; Public Liberty/active (inferred) | **Responsibility** + **Right** | Public: `claim_right` → Enforcement authority (plus its Right) |
| Communications Act 2003 s.108(6): "OFCOM must make the register available for public inspection" | OFCOM Obligation/active; Public Liberty/active (inferred) | Responsibility + Right | Public: `claim_right` → OFCOM (plus its Right) |
| Water Act 1989 s.82(2)(c): Minister may … | Minister Liberty/active; Company none/counterparty | **Power** | Company: `liability` → Minister |
| Medicines Act 1968 s.97E(3) | Responsible Undertaking Obligation/active; Driver none/beneficiary | **Duty** | Driver: `protected` → Responsible Undertaking |
| "The authority shall serve a notice on the operator requiring…" | Authority Obligation/active; Operator none/counterparty | Responsibility only (no Right) | Operator: `claim_right` → Authority |
| "The register shall be kept at the principal office" (passive, no holder) | none | raw Obligation, holder unknown | none |
| "Records shall be kept for five years" (passive, no holder) | none | raw Obligation, holder unknown | none |
| "Every workplace shall be ventilated…" (thing-subject, no applying provision found) | none | raw Obligation, holder unknown | none |
| Workplace Regs 1992 reg.6(1): "Effective and suitable provision shall be made to ensure that every enclosed workplace is ventilated…", applied by reg.4(1) "Every employer shall ensure that every workplace … complies with any requirement of these Regulations", reg.4(2) (every person who has control of a workplace) and reg.4(5) (deemed occupier of a factory) | `Org: Employer`, `Ind: Person in Control`, `Org: Occupier`: each Obligation/active (inferred, from reg.4) | **Duty** | none |
| "A notice shall be treated as served if it is sent by post…" (deeming) | none | no type, no DRRP | none |
| "A person guilty of an offence under this section is liable…" | Person none/mentioned | no DRRP | none |
| HSWA 1974 s.2(1): "It shall be the duty of every employer to ensure … the health, safety and welfare at work of all his employees" | Employer Obligation/active; Employee none/beneficiary (layer 2, 2026-10-01) | **Duty** | Employee: `protected` → Employer |
| HSWA 1974 s.3(1): duty "to ensure … that persons not in his employment … are not thereby exposed to risks" | Employer Obligation/active; Person none/beneficiary | **Duty** | Person: `protected` → Employer (see the layer 1b prerequisite: s.2/s.3 consistency) |

## Conformance: fractalaw vs this spec (2026-09-30)

| Area | Status | Action |
|---|---|---|
| Layer 3 holder class | ✅ `actors::is_government` reads the dictionary `type` (prefix fallback only for labels not in the dictionary); used by the roll-up, the #67 access rule and the LLM tier | none |
| Layer 1 `Rule` | ✅ `DutyType::Rule` removed; the thing-subject tier is `DutyFamily::ThingSubject` and yields `Obligation` | none |
| Layer 1 non-active type | ✅ payload `actors[].drrp` is `none` unless active (`backfill_from_actors`); the roll-up skips non-active actors | Takes effect on the next backfill + republish |
| Layer 4 union | ✅ active-actor union (`bb0d4ff`); raw fallback = holder unknown | none |
| Layer 5 roll-up | ✅ active actors only, holder-unknown guard (`law_drrp::aggregate`); `taxa backfill --dry-run` reports current vs new verdicts | Dry run 2026-09-30 awaiting Jason's review before the backfill + publish |
| Prompts (SLM/LLM) | ⚠️ ask for "the DRRP type of the provision for this actor" | Align to "what this actor holds"; check against the benchmarks (#65) |
| `HM Forces` class | ✅ dictionary now `type: government` (matches legal) | Existing rows take effect on the next roll-up/backfill |
| Legal docs | ⚠️ FUNCTION_VALUES out of date; Housekeeping ↔ no_obligations; legacy OL = holder unknown | Legal aligns and links here |
| Holder from an applying provision (#60) | ✅ labelling prompt `drrp-v1.1`: `applying_index()`/`applying()` pass APPLYING PROVISIONS (`scripts/drrp_prompt.py`). ❌ Rust tiers (regex/SLM) and reconcile don't yet write these inferred holders | The SLM learns it from the v1.1 training labels; reconcile writes `extraction_method = inferred` when the LLM tier moves to per-provision labels |
| Layer 1b correlatives (#72) | ❌ not implemented | Prerequisite: counterparty/beneficiary consistency; then backfill derivation, payload `correlatives` and law-level holder lists; legal migration before first publish |

Tests follow the tables: one table-driven case per row of layers 4–5 and per special case.
