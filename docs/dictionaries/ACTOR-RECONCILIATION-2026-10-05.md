# Actor dictionary reconciliation: fractalaw vs sertantai-legal (2026-10-05)

**Why:** the drrp-v1.1 training-label pilot returned 15 `OTHER:` actors. Several of them (consignor, consignee, producer, master of a vessel) already exist in legal's regex library. Jason: the fractalaw actor dictionary is a known gap, so reconcile it before any LLM labelling.

**Sources:**
- fractalaw: `crates/fractalaw-core/data/actor-dictionary.yaml`, 132 labels.
- legal: `backend/lib/sertantai_legal/legal/taxa/actor_definitions.ex`, 129 labels in `@government_patterns_raw` / `@governed_patterns_raw`. Class is decided by label prefix. Legal's `priv/data/actor-dictionary.yaml` is only a snapshot of ours.

**Overlap:** 75 labels shared, with no class disagreements; 54 legal-only; 57 fractalaw-only.

**Proposal (legal suggested, Jason to confirm):**
- fractalaw's YAML becomes **canonical**, and legal's regex library keys its patterns to our labels.
- Additions go in as **trigger-only** entries first. Those are LLM-visible and regex-inert, so the regex tier's output doesn't change until patterns are added with tests. Broad patterns like `[Hh]olders?`, `[Gg]enerators?` or `[Cc]onsumer` would shift regex extraction across the corpus.

## A. Add from legal (no equivalent in fractalaw)

- **Supply chain:** SC: Producer, SC: Exporter, SC: Retailer, SC: Seller, SC: Consumer, SC: Customer, SC: Marketer, SC: Storer, SC: Generator, SC: T&L: Consignor, SC: T&L: Consignee, SC: T&L: Handler, SC: C: Constructor, SC: Domestic Client.
- **Environment:** Env: Disposer, Env: Polluter, Env: Recycler, Env: Reuser, Env: Treater.
- **Individuals:** Ind: Holder, Ind: Appointed Person, Ind: Relevant Person, Ind: Suitable Person, Ind: Chair, Ind: Diver.
- **Organisations:** Org: Investor, Org: Lessee, Org: Partnership.
- **Maritime:** Maritime: master, Maritime: crew. Rename to `Maritime: Master` / `Maritime: Crew` for casing?
- **Specialists and services:** Spc: Advisor, Spc: OH Advisor, Spc: Surveyor, Spc: Technician, Svc: Maintainer, Svc: Repairer.
- **Government:** Gvt: Authority: Energy (NI), Gvt: Ministry: Department of the Environment (NI), HM Forces: Navy.
- **Public:** Public: Parents.

## B. Same concept, different label: pick one name (Jason)

| fractalaw | legal | Note |
|---|---|---|
| SC: Applicant | Ind: Applicant | |
| Spc: Authorised Person (government) | Ind: Authorised Person (governed) | **Class differs.** Legal also lists "Spc: Authorised Person" in its government exact set. |
| Spc: Licence Holder | Ind: Licence Holder | |
| Public: Keeper (PUBLIC-gated) | SC: Keeper ("person who … keeps") | |
| Public: Dealer (PUBLIC-gated) | SC: Dealer (scrap metal dealer) | |
| SC: Authorised Representative | Spc: Representative | |
| Gvt: Ministry, Gvt: Agency, Gvt: Devolved Admin | `Gvt: Ministry:` etc. (trailing colon) | Legal's catch-all form |
| SC: Notified Body, Spc: Approved Body, Spc: Conformity Assessment Body … | Spc: Body (one pattern for all) | Keep ours (more specific) |

## C. Don't add (too broad for a label)

- `: He` (pronoun `[Hh]e`).
- `Organisation` (third party / organisations).
- `Gvt: Official` ("Official").
- `SC: Agent` (`[Aa]gents?`, which matches agency, agent of a company, chemical agent…).
- `Spc: Body` (see B).

## D. New: in neither (from the pilot)

| Actor | Proposed label | Class |
|---|---|---|
| Council of the European Union | EU: Council | government |
| European Parliament | EU: Parliament | government |
| Tenant | Org: Tenant (beside Org: Landlord) | governed |
| Safety committee | Spc: Safety Committee | governed |
| FACTS adviser | covered by Spc: Advisor (A) | governed |
| "taker of the provisional measure" | none: too specific | — |

## E. fractalaw-only (57): legal to add patterns keyed to these labels

These include:
- EU agencies (ECHA, EEA, EFSA), EU: Member State;
- devolved ministers (Scottish/Welsh), Gvt: Mayor, Gvt: Authority: Combined County / Health Body / Fire and Rescue;
- Gvt: Agency: GEMA / NDA / MCA / Oil and Gas Authority / Information Commissioner;
- Ind: Person in Control, Ind: Claimant, Ind: Appellant, Ind: Hirer, Ind: Data Subject;
- Public: Data Controller / Data Processor / Provider;
- the conformity, approval and certification bodies;
- insolvency roles (liquidator, receiver, trustee in bankruptcy), Svc: Water Undertaker, Offshore: Licensee, SC: Downstream User, SC: Registrant.

## Next

Once Jason has decided B and confirmed "canonical + trigger-only first":
1. Add A + D to the YAML (trigger-only), plus the B renames, with tests for any patterns added.
2. Send legal the final label list.
3. Re-run the pilot's `OTHER:` provisions to confirm they map.
4. Start the family-ordered labelling.
