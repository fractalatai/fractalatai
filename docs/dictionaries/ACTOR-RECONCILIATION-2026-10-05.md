# Actor dictionary reconciliation: fractalaw vs sertantai-legal (2026-10-05)

**Why:** the drrp-v1.1 training-label pilot returned 15 `OTHER:` actors. Several of them (consignor, consignee, producer, master of a vessel) already exist in legal's regex library. Jason: the fractalaw actor dictionary is a known gap, so reconcile it before any LLM labelling.

**Sources:**
- fractalaw: `crates/fractalaw-core/data/actor-dictionary.yaml`, 132 labels.
- legal: `backend/lib/sertantai_legal/legal/taxa/actor_definitions.ex`, 129 labels in `@government_patterns_raw` / `@governed_patterns_raw`. Class is decided by label prefix. Legal's `priv/data/actor-dictionary.yaml` is only a snapshot of ours.

**Overlap:** 75 labels shared, with no class disagreements; 54 legal-only; 57 fractalaw-only.

**Label prefixes are groups** (Jason): `Ind:` individual, `Org:` organisation, `SC:` supply chain (`SC: C:` construction, `SC: T&L:` transport & logistics), `Spc:` specialist, `Gvt:` / `EU:` government. `Svc:` (legal's maintainer, repairer) reads as service; to confirm. The prefix says what the actor **is**. Family gating belongs in `families:`, not the prefix. Some labels use a family prefix instead (`Public:`, `Offshore:`, `Maritime:`). New labels follow the group rule. Renaming published labels needs legal's agreement, so it's flagged here rather than done.

**Proposal (legal suggested, Jason to confirm):**
- fractalaw's YAML becomes **canonical**, and legal keys its regex patterns to our labels.
- Additions go in **trigger-only** first: LLM-visible, regex-inert. Patterns follow later with tests, so the regex tier doesn't shift across the corpus in the meantime.

## A. Add (no equivalent in fractalaw), under the group rule

| Group | Labels | Note |
|---|---|---|
| SC | Producer, Exporter, Retailer, Seller, Consumer, Customer, Marketer, Storer, Generator, Agent | Generator covers both waste and electricity, so gate later if the regex needs it |
| SC: T&L | Consignor, Consignee, Handler | |
| SC: C | Constructor, Domestic Client | Legal has `SC: Domestic Client`. Construction actors use `SC: C:`, so use `SC: C: Domestic Client` |
| Ind | Holder, Appointed Person, Relevant Person, Suitable Person, Chair, Diver, **Parent** | Legal's `Public: Parents` → `Ind: Parent` (an individual; Public is a family) |
| Org | Investor, Lessee, Partnership | |
| Spc | Advisor, OH Advisor, Surveyor, Technician | |
| Svc | Maintainer, Repairer | |
| Env | Disposer, Polluter, Recycler, Reuser, Treater | Legal's group. The waste-chain roles could be `SC:`, but Polluter isn't supply chain. Keep `Env:` for convergence |
| Maritime | Master, Crew | Legal's lowercase `Maritime: master/crew`. A domain prefix like `Offshore:`; by the group rule these are `Ind:`. Decide with the family-prefix question (E) |
| Gvt | Authority: Energy (NI), Ministry: Department of the Environment (NI) | |
| HM Forces | Navy | |

**Agent stays, but not with legal's pattern.** `[Aa]gents?` mostly hits substances (chemical 133, biological 96, process, extinguishing, oxidising, physical agent). The actor senses in the corpus are:
- "owner or agent" / "servant or agent" (~87 "or agent");
- "his / the / an agent";
- "authorised agent" (24);
- "agent of the operator / owner / Crown / licence holder";
- "agent … acting on behalf";
- customs, property, travel, handling and diplomatic agents.

When it gets a regex, match these forms (`authorised agent`, `agents? (of|for) (the|a|an|any|his|its)`, `(owner|operator|servant|employer) or agent`, `(his|its|their) agents?`, `(customs|shipping|handling|forwarding|property|travel|letting) agent`), never the bare word. Our Rust regex has no lookbehind, so legal's `(?<![Bb]iological )` exclusion can't be ported anyway. As trigger-only, the LLM sees the label and reads "chemical agent" correctly.

## B. Same concept, different label: decided by the group rule

| Concept | fractalaw now | legal | **Proposed** | Why |
|---|---|---|---|---|
| Applicant | SC: Applicant | Ind: Applicant | **Ind: Applicant** | An applicant isn't a supply-chain role. Legal already uses it |
| Licence / permit holder | Spc: Licence Holder; Ind: Licensee; Offshore: Licensee | Ind: Licence Holder; Ind: Licensee | **Ind: Licensee**, absorbing "licence/permit holder"; retire Spc: Licence Holder | Not a specialist. Both sides already have Ind: Licensee |
| Appellant | Ind: Appellant **and** Spc: Appellant (our duplicate) | — | **Ind: Appellant**; retire Spc: Appellant | Duplicate |
| Authorised person | Spc: Authorised Person, `type: government` but `match_group: governed` | Ind: Authorised Person, governed | **Spc: Authorised Person, type governed** | Our prompt sends government-authorised officers to Gvt: Officer, so this label is the specialist authorised by the duty holder (electrical, mines, rail). **Class change:** existing rows go Responsibility → Duty at the next backfill |
| Authorised representative | SC: Authorised Representative | Spc: Representative | **SC: Authorised Representative** | The EU product-law role, appointed by the manufacturer, is supply chain. Legal's generic one maps here or to Spc: Employees' Representative |
| Keeper | Public: Keeper (PUBLIC-gated) | SC: Keeper ("person who … keeps") | **SC: Keeper**, gated by `families:` where ambiguous | Keeper of waste/animals/vehicles; Public is a family not a group. Published label: rename with legal |
| Dealer | Public: Dealer (PUBLIC-gated) | SC: Dealer (scrap metal) | **SC: Dealer** | Supply chain. Published label: rename with legal |
| Provider | Public: Provider (PUBLIC-gated) | — | **Svc: Provider** | Service provider. Published label: rename with legal |
| Data controller / processor | Public: Data Controller / Processor (added today, unpublished) | — | **Org: Data Controller**, **Svc: Data Processor** (both keep `families: ["PUBLIC: Data"]`) | Rename now, before anything is published |
| Catch-all government | Gvt: Ministry / Agency / Devolved Admin | `Gvt: Ministry:` etc. (trailing colon) | **ours** | Legal maps |
| Conformity/approval bodies | SC: Notified Body, Spc: Approved Body, Spc: Conformity Assessment Body, … | Spc: Body (one pattern) | **ours** (more specific) | Legal maps |

## C. Don't adopt

- `: He` (the pronoun).
- `Organisation` (third party / organisations).
- `Gvt: Official` ("Official").
- `Spc: Body` (see B).

## D. New: in neither list (from the pilot)

| Actor | Proposed label | Class |
|---|---|---|
| Council of the European Union | EU: Council | government |
| European Parliament | EU: Parliament | government |
| Tenant | Org: Tenant (beside Org: Landlord) | governed |
| Safety committee | Spc: Safety Committee (beside Spc: Employees' Representative, Spc: Trade Union) | governed |
| FACTS adviser | covered by Spc: Advisor (A) | governed |
| "taker of the provisional measure" | none: too specific | — |

## E. Later (with legal): family prefixes vs group prefixes

`Public: Keeper / Dealer / Provider`, `Offshore: Licensee` and `Maritime: master / crew` use a family/domain prefix instead of a group. Under the group rule they become `SC: Keeper`, `SC: Dealer`, `Svc: Provider`, `Ind: Licensee` (gated) and `Ind: Ship's Master` / `Ind: Crew`, with family gating in `families:`. These are published labels, so they rename only as a joint change with legal (old → new map applied on both sides, then republish). Part of B proposes doing this for Keeper, Dealer and Provider now; the rest can wait.

Our 57 labels legal lacks, for legal to add patterns keyed to them:
- EU agencies (ECHA, EEA, EFSA), EU: Member State;
- devolved ministers, Gvt: Mayor, Combined County / Health Body / Fire and Rescue authorities;
- GEMA / NDA / MCA / OGA / Information Commissioner;
- Ind: Person in Control, Claimant, Appellant, Hirer, Data Subject;
- the data protection roles;
- the conformity, approval and certification bodies;
- insolvency roles; Svc: Water Undertaker; SC: Downstream User, SC: Registrant.

## Next

Once Jason has confirmed B and "canonical + trigger-only first":
1. Add A + D to the YAML (trigger-only).
2. Apply the B renames and the Authorised Person class fix, with tests.
3. Send legal the final label list and the old → new rename map.
4. Re-run the pilot's `OTHER:` provisions to confirm they map.
5. Start the family-ordered labelling.
