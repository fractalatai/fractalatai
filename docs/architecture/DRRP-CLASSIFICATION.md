# DRRP Classification Schema

The authoritative definition of how fractalaw classifies legal relations in UK/EU legislation, and how that data becomes DRRP (Duties, Rights, Responsibilities, Powers) and a law-level verdict in sertantai-legal.

- **Status:** agreed 2026-09-29 (Jason; reviewed by sertantai-legal). Tracks fractalatai #68.
- **Owners:** fractalaw (layers 1–5, payload); sertantai-legal (expansion and `is_making` resolution) links here.
- **Rule of use:** any change to classification behaviour updates this document first. Code and tests follow it.
- **Unit and rules (2026-10-07):** labelling is per **legal sentence** (stem, items and closing words, labelled once). The labelling rules are catalogue v2 (`DRRP-RULE-CATALOGUE.md`, 31 principles), which is authoritative: where this spec and the catalogue differ, the catalogue wins. This spec keeps the data model, layers, schema and payload; special cases are pointers to principles.

## Labelling unit and gold schema (#78 option c, 2026-10-07)

- **Per sentence:** `relation` (`yes` | `no` | `continues`, REL-01, REL-28), `raw_type` (TYPE-04), `purpose` (`PURPOSE-CLASSIFICATION.md`: the sentence's own class, or `inherit`).
- **Per actor entry:** `position`, `holds` (`Obligation` | `Liberty` | `both` | `none`), `inferred`, `act`.
  - `act` is a **list**: `[]` unless the actor is an Obligation's counterparty (ACT-01, ACT-12).
  - `holds: both`: one party with a duty and a power is one entry (POS-15).
- **One entry per label per role** (POS-15). The gold key is (gold_version, section_id, field, actor_label, `actor_position`): `actor_position` is `''` for a label's entry, and the role for a **second entry** of the same label in another role. Parties sharing a label and a role are one entry; a `mentioned` role next to a substantive one is dropped.
- **Excluded from gold:** amending text (REL-07).

## The five layers

```
provision text
   │  per actor
   ▼
1. Hohfeldian type ── what THIS actor holds: Obligation | Liberty | both | none
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
| `Liberty` | this actor holds a power, permission or entitlement ("may", "is entitled to"), including **implied rights** (precedent `implied-access-right`) |
| `both` | this actor holds an Obligation and a Liberty in the sentence: one entry (POS-15, #78) |
| `none` | this actor holds nothing here (e.g. it is only referred to) |

There is **no `Rule` type** (removed 2026-09-30; REL-42). Every "shall"/"must" that requires someone to act or to bring about a state of affairs is an `Obligation`, even when the text names no one:
- **Passive and thing-subject duties** ("records shall be kept", "equipment must be provided", "traffic routes must be suitable") are `Obligation`. Their holder comes from an applying provision or the parent Act (HOLD-05, INF-03); otherwise it's unknown (HOLD-15). A thing can't hold a duty, so these are never a separate type or a sub-type that blocks holder resolution.
- **Definitions, deeming, application and extent** ("'premises' includes…", "a notice shall be treated as served…", "this Part applies to…") create no relation, so they get no type (REL-06, REL-07). Deeming beats passive "shall" ("shall be treated as", "shall be taken to", "deemed"). A positive provision-level label for them (constitutive provisions) is proposed separately as a provision-function field outside DRRP (#69).

**Exemptions are scope, not Liberty or Immunity (Jason, 2026-10-01; REL-08, confirmed 2026-10-07).** Three things are easy to confuse:
- **Liberty** (Hohfeld's privilege) is an affirmative freedom the actor holds: "the employee *may* refuse…". The actor is `active` and holds a Right.
- **Immunity** is protection against someone else's *power*. The other party lacks the power over this actor (Hohfeld's correlative: disability). Examples: "the Crown *shall not be liable* to prosecution", "no improvement notice *may be served* on…".
- **An exemption** ("these Regulations shall not apply to an employee") is neither. It gives the employee nothing and blocks no power; it takes the employee out of the duty's **scope**, so the duty never reaches them. It's the negative form of "this Part applies to…": a scope (constitutive) provision, relation `none`, and the exempted party is `mentioned`.

**Immunity is deferred (#70, Jason 2026-10-01).** It isn't in this model version. Genuine Immunity provisions ("shall not be liable", protection from enforcement or prosecution, possibly statutory defences) are rare in ESH law and findable by their wording. Adding Immunity later means a targeted relabel of those provisions only, not a re-run. Until then they're `none`.

**Non-active actors hold nothing (TYPE-03).** An actor whose position isn't `active` has type `none`, whatever the model predicted. `provision_actors.drrp` keeps the tier's reading (the provision's type from that actor's side), which is used only as the raw type when the holder is unknown. The payload and the roll-up expose `none`.

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
  - Item shape: `{type, to, act}`; `act` is a list (#78); absent means unknown.
  - **Only an Obligation's counterparty carries an act** (ACT-01). A right doesn't: it mirrors the duty's act. Under the #67 access rule the governed party stays active with the inferred Liberty and its `claim_right`. **Open (Jason):** #78 now allows a second entry per role (POS-15); whether that party also takes a counterparty entry with `give_access` isn't decided (precedent `implied-access-right`, no gold case yet).
  - **Synonym mapping** (ACT-12): the text's verb maps onto the fixed classes; new verbs never create new classes. The duty's main verb decides ("serve a notice on X informing it" → `notify`).

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
  - **A passive duty's counterparty also gets an act** (ACT-01): "notice shall be given to the operator" → operator counterparty, act `notify`, even though the holder is unknown. The recipient receives the act either way.
  - **Labelled now:** gold and the definitive prompt label `act` (a list) on each Obligation counterparty. Regex derives it for new laws from the clause verb, measured against those labels. It isn't an SLM target.
  - **Published later:** once compliance and legal agree it on #75. It never feeds DRRP or the verdict.
- **Never feeds layers 4–5.** DRRP types, the holder lists, the verdict and `is_making` stay based only on held types.
- **`claim_right` is the duty's direct correlative, in a broad public-law sense** (Gemini, #72): it doesn't by itself imply a private right of civil action. Many regulatory duties aren't actionable (e.g. HSWA s.47 for ss.2–8). User-facing text should present it as "owed to you", not "you can sue".
- **`protected` is a compliance category, not Hohfeldian.** A beneficiary isn't the direct correlative party. Many "owed to" duties give no enforceable claim either: HSWA s.47 excludes civil action for breach of ss.2–8. `protected` answers "what protects me".
- **Implied rights (#67) stay** (precedent `implied-access-right`). "Available for inspection by the public" gives the Public an inferred **Liberty** (active → Right: it may inspect) *and* a `claim_right` (the authority owes it an available register). These are two relations. Only the Liberty counts in DRRP (Jason, 2026-09-30).
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
- **`counterparty`** = the **recipient** of the duty's act, including a prohibited act aimed at the party (POS-02) → layer 1b `claim_right`. Of a power or right: the party subject to it (POS-05). Parties whose conduct the duty permits or withholds: act `other` (POS-12).
- **`beneficiary`** = the party the words name as what is protected (POS-19) → layer 1b `protected`. HSWA s.2(1) (employees) and s.3(1) (persons not in his employment) are both beneficiaries.
- **Both at once:** recipient and protected → `counterparty` (POS-02).
- **`mentioned`:** no role in the relation (POS-09); a party the duty ensures *has* something without the text saying it's provided (POS-13); a party in a condition or trigger only (POS-19); an exempted party (REL-08); every actor of a `no` sentence (POS-14); the holder of the duty a `continues` sentence continues, when named there (REL-28, Jason 2026-10-07).

**Why:** the SLM split HSWA s.2(1) (Employee = counterparty) and s.3(1) (Person = beneficiary), the same kind of duty. On Obligation provisions, protective-wording duties were split 177 counterparty vs 244 beneficiary, and 683 beneficiaries carried recipient wording.

**Reviews:** Gemini and legal agree (`data/code-review/drrp-counterparty-beneficiary-gemini.md`); legal's consistency check maps position → correlative type, so it isn't affected.

**Fix path:** the definitions go into the SLM/LLM position prompts now. Positions are corrected when the re-enrichment backlog re-runs those tiers. Correlatives are derived from positions, so they follow.

### Layer 3: Holder class

Defined by the **actor dictionary** (`crates/fractalaw-core/data/actor-dictionary.yaml`, field `type`), never by label prefix.

| Class | Members |
|---|---|
| **government** | every entry with `type: government`: all `Gvt*`, all `EU:*`, `Crown`, `HM Forces`, `Spc: Notifying Authority` (2026-09-30). `Spc: Authorised Person` moved to governed on 2026-10-05: a person authorised by the duty holder (LBL-06). Government-authorised officers are `Gvt: Officer`; an authorised person acting for an enforcing authority is `Gvt: Authorised Person` (LBL-05, approved 2026-10-07, dictionary task pending). |
| **governed** | every entry with `type: governed`: `Ind:*`, `Org:*`, `SC:*`, `Spc:*` (except Notifying Authority), `Svc:*`, `Public*`, `Offshore*`, … |

`HM Forces` is government, as the Crown's forces (Jason, 2026-09-29), which matches sertantai-legal's `ActorDefinitions.government_label?/1`. A label-prefix check misses `Crown`, `HM Forces` and `Spc: Notifying Authority`, so code must read the dictionary `type`.

### Layer 4: DRRP (per provision, active actors only)

| Type held (layer 1) | Governed holder | Government holder |
|---|---|---|
| Obligation | **Duty** | **Responsibility** |
| Liberty | **Right** | **Power** |
| `both` | **Duty** + **Right** | **Responsibility** + **Power** |

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

Labelling rules are catalogue v2's principles; the rows here are pipeline mechanics, pointers to principles, or retired. Precedent patterns are in the appendix.

### Pipeline and scope

| Case | Rule |
|---|---|
| **Amendment (inserted) text** | Belongs to the amended instrument; excluded from every layer (`scope = amendment`, #57) and from gold (REL-07). |
| **Holder unknown** | No holder named or supplied → no active actor, raw type, no DRRP (HOLD-15, TYPE-04). Never a guessed default such as the law's dominant actor (Jason, 2026-10-05). |
| **Holder named in an applying provision** (#60) | Context principle HOLD-05, with INF-03: the party another same-law provision puts under a duty to comply, or ensure compliance, with requirements covering this duty is `active`, holds the Obligation, `inferred: true` (`extraction_method = inferred`). Every applying provision whose scope covers the duty contributes its holder. The applying provision's own sentence is a pointer (`no`, X `mentioned`); supervisory and prohibition forms are duties in their own right. Decided by the holder-linking step and scored apart from the sentence tiers. HOLD-07 to HOLD-09 (supervisory, essential-requirements and prohibition forms) are forms of HOLD-05 kept as its examples; none is a principle of its own until the phase C top-up shows about 3 provisions from 2 laws (decision Q4). |
| **Holder named in another instrument** (#77) | Context principle INF-03: the holder comes from the parent Act whenever it can be resolved (`active`, `inferred: true`), holder unknown only when it can't. The skill `cross-instrument-holders` resolves "the Act" via DuckDB `enacted_by` and proposes; Jason approves into the **adjudicated** tier (`adj_note` names the parent provision). Runs on the gold laws before they are justified. (Jason, 2026-10-05/06) |
| **Scoped LAT** | `enabling_extent`: never classified. `relevance`: classified within the scope; the verdict is scope-relative, and provenance carries `lat_coverage` (#66). |
| **Revoked laws** | Status never changes classification: revocation doesn't affect what the law did while it was live, so a revoked Making law stays Making and is never re-marked non-Making because it is revoked (Jason, 2026-09-28). This is **not** a reason to re-process dead laws: their existing verdict stands, and no earlier text is fetched for them (Jason, 2026-09-30). |
| **Human adjudication** | The `adjudicated` tier (`provision_actors.adj_drrp`/`adj_position`/`adj_note`) is the top source tier: reconcile never overrides it and `taxa infer` never deletes its rows. First used for benchmark gold labels carried across a LAT sync (`scripts/benchmarks/carry_forward_gold.py`). |

### Covered by a v2 principle

Purpose for any of these is in `PURPOSE-CLASSIFICATION.md`.

| Case | Principle |
|---|---|
| Offences and penalties; statutory defences; procedural time limits | REL-04: `no`, every actor `mentioned`. Immunity deferred (#70). |
| Exemptions, incl. time-limited disapplications | REL-08: `no`, exempted party `mentioned`. Exemptions and defences stay `no` (Jason, 2026-10-07). |
| Definitions, deeming, electronic delivery, designation inside a definition | REL-06: `no`; deeming beats passive "shall" (REL-42). Deeming that changes who holds duties elsewhere: precedent `deemed-holder`. |
| Machinery: application, extent, savings, citation, commencement (lists, Gazette notification, "on such day as X may by order appoint"), parliamentary procedure (annulment/affirmation, "may not be made unless a draft has been laid"), money provided by Parliament | REL-07: `no` (Jason, 2026-10-07: commencement and parliamentary procedure are machinery). REL-41 no longer lists powers to commence (decision B, 2026-10-07). |
| Laying before Parliament, review clauses, powers to exempt, transitional powers and duties, mixed provisions | REL-41: the sentence's own duty or power beats its machinery class → `yes`. |
| Enforcing-authority designations | REL-27: `yes`, the designated body `active`, Obligation, purpose Requirements (Jason, 2026-10-07). A dated transfer of the regulator ("S is the regulator of B from <date>") is `no`, purpose Application (Jason, 2026-10-07, GHG ETS reg.13(2)). |
| Detail of a duty (content, form, manner, timing, discharge); conditions and limits on a power created elsewhere; content sentences of schemes, regulations and notices that name no holder ("The scheme must include—", "Regulations under subsection (1) may—") | REL-28: `continues`, no holder, no raw_type; a named holder of the continued duty is `mentioned`, other actors keep their roles. |
| A named party as the modal's subject: permitted ways of discharging a duty, procedural or deadline duties of a government actor | REL-33: own relation, `yes`. With no named subject (passive, agentless: "may be served by post") REL-28 decides (Jason, 2026-10-07). |
| Appeal and inquiry procedure | REL-01: `yes` only where the sentence creates its own duty or power. |
| Applications to a court or tribunal | HOLD-01 (the court holds the Liberty); the applicant is `mentioned` (POS-19). |
| Member State: actor or place; instruments are never actors | LBL-03. |
| Beneficiary test; trigger-condition actors | POS-19. |
| Withheld conduct | POS-12. |
| "Means at their disposal" | POS-13. |
| One actor, two roles; two persons, one label | POS-15 (#78 option c): see the gold schema above. |

### Retired with sentence units

- **Stems and list items: counted once**, **headless stems**, stem holder, items take the stem's type (REL-45, REL-30, HOLD-02, TYPE-06): the sentence is labelled once.
- Row-level **`continues` on items** and actors on a `continues` item (REL-28 item half, POS-17); the access-duty `continues` row (POS-18).
- **Class-definition items**, their purpose, and **details inside a content list** (REL-15); **content list vs detail** (REL-16); **timing items** (REL-13).
- **Cross-references, commencement lists, conditions** (REL-10): its `no` outcome was overturned; now REL-28, REL-06, REL-08.
- **Holder named in a referenced provision** (HOLD-04) and **instrument maker as holder** (HOLD-12): now REL-28.
- **One entry per label, strongest role** (old POS-15) and **label every role** (POS-16): now POS-15 revised.

## Payload contract (fractalaw → sertantai-legal)

**Provision payload** (`taxa/provisions/{law}`, from `legislation_text`):
- `drrp_types`: the union of the active actors' types, or raw types when the holder is unknown (layer 4).
- `actors[]`: `{label, position, drrp, reason (= extraction_method), label_source, relates_to}`.
  - `drrp` is the actor's own layer-1 type;
  - non-active actors carry `none`;
  - legal types DRRP from active actors only.
  - **Open (phase E/F):** whether the published payload carries the #78 schema (`drrp = both`, a second entry per role, `act` as a list) is decided with legal before the single run (meta-plan legal-side list).
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

## Appendix: Precedents (examples, not rules)

Reviewed gold sentences retrieved by pattern tag (catalogue v2, "Precedent patterns"). Never cited as rules.

- `functions-list` (REL-44): "The functions of X shall be—": a government body's functions are an Obligation, a governed party's a Liberty.
- `implied-access-right` (INF-01, INF-02): a government duty to make registers or copies available to a named governed party gives that party an inferred Liberty (#67); enforcement and notice service never do.
- `no-person-shall-be-engaged` (HOLD-17): "No person shall be engaged/employed/permitted to…": every applying holder who can engage someone holds it; the person engaged is `mentioned`.
- `participation-right` (TYPE-07): "shall take part in … or shall be consulted" gives the participants a Liberty despite "shall".
- `deemed-holder` (REL-06): "A person who … shall be treated as the employer…" is `no`, but tells the holder-linking step who holds duties elsewhere.
