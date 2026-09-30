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
- **Passive and thing-subject duties** ("records shall be kept", "equipment must be provided", "traffic routes must be suitable") are `Obligation` with the holder unknown until #60 resolves it (e.g. stem inheritance). A thing can't hold a duty, so these are never a separate type or a sub-type that blocks holder resolution.
- **Definitions, deeming, application and extent** ("'premises' includes…", "a notice shall be treated as served…", "this Part applies to…") create no relation, so they get no type. A positive provision-level label for them (constitutive provisions) is proposed separately as a provision-function field outside DRRP (#69). The missing Hohfeldian positions (Immunity etc.) are proposed in #70.

**Non-active actors hold nothing.** An actor whose position isn't `active` has type `none`, whatever the model predicted.

**Source and precedence** (reconcile, `fractalaw-cli/src/commands/taxa.rs`):
- **Type:** LLM > inferred-active > SLM > regex. An inferred type counts only when the inference also makes the actor active.
- **Position:** LLM > inferred > SLM (≥0.9 conf) > regex/classifier agree > classifier (≥0.7) > `pending_slm` > regex.
- The final values are `provision_actors.drrp`, `provision_actors.position` and `provision_actors.extraction_method`.

### Layer 2: Position (per actor)

| Value | Meaning | Holds a type? |
|---|---|---|
| `active` | bears the duty or exercises the liberty: **the holder** | yes |
| `counterparty` | to whom the duty is owed, or subject to the power | no |
| `beneficiary` | benefits, but is neither the holder nor the direct correlative | no |
| `mentioned` | referred to, with no legal role | no |

### Layer 3: Holder class

Defined by the **actor dictionary** (`crates/fractalaw-core/data/actor-dictionary.yaml`, field `type`), never by label prefix.

| Class | Members |
|---|---|
| **government** | every entry with `type: government`: all `Gvt*`, all `EU:*`, `Crown`, `HM Forces`, `Spc: Notifying Authority` |
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
| **no_obligations** | parsed and reconciled, with no DRRP; **or** zero actors and no duty text in any substantive provision (evidenced, provenance model `fractalaw-law-drrp:no_duty_text`) |
| *(no verdict)* | actors not yet reconciled; **or** zero actors but duty text present (actor gap, #58/#60); **or** the LAT is `enabling_extent` |

Law-level holder fields (`duty_holder`, `rights_holder`, `responsibility_holder`, `power_holder`, `duties`, `rights`, `responsibilities`, `powers`) list **who holds** each DRRP, which means active actors only.

## Special cases

| Case | Rule |
|---|---|
| **Offences and penalties** | "commits an offence", "is liable on conviction to…" → `none`. A penalty is not a duty; the duty is in the provision the offence refers to. |
| **Cross-references, commencement lists, conditions** | A provision that only references, commences, conditions, details, defines or exempts a relation created elsewhere → `none`. |
| **Amendment (inserted) text** | Belongs to the amended instrument; excluded from every layer (`scope = amendment`, #57). |
| **Holder unknown** | Obligation/Liberty text with no actors, or no active actor → raw type, no DRRP. Fractalaw fills holders over time (#60: passive/impersonal duties; stem inheritance for list items). |
| **Implied rights** | Where a government actor's active Obligation grants a governed party access (inspection by the public, facilities for copies, supply on request/payment), the governed party named in the clause gets an **inferred Liberty, active**, so a Right. Depends on the wording: enforcement or notice-service provisions never qualify. Marked `extraction_method = inferred` (#67). |
| **Scoped LAT** | `enabling_extent`: never classified. `relevance`: classified within the scope; the verdict is scope-relative, and provenance carries `lat_coverage` (#66). |
| **Revoked laws** | Classified as while in force. A revoked Making law was Making; status and DRRP are independent (Jason, 2026-09-28). |
| **Human adjudication** | An `adjudicated` actor type (human-review) is the top source tier and survives reconcile. |

## Payload contract (fractalaw → sertantai-legal)

**Provision payload** (`taxa/provisions/{law}`, from `legislation_text`):
- `drrp_types`: the union of the active actors' types, or raw types when the holder is unknown (layer 4).
- `actors[]`: `{label, position, drrp, reason (= extraction_method), label_source, relates_to}`.
  - `drrp` is the actor's own layer-1 type;
  - non-active actors carry `none`;
  - legal types DRRP from active actors only.
- Legal expands to DRRP per active actor using its holder class. With no active actor, it keeps the raw type (holder unknown).

**Law payload** (`taxa/enrichment/{law}`, from DuckDB `legislation`): the layer-5 holder fields, significance, fitness and application fields, plus `provenance` (#63).

**The verdict is one input to legal's `is_making`, not the final word.** sertantai-legal's `Legal.Making` resolves in this order: human review > enrichment (this verdict) > legacy DRRP > triage > legacy flag > detector > default. E.g. UK_uksi_2008_198 and UK_uksi_2014_2868 stay Making by human review despite a `no_obligations` verdict. Legal's "Housekeeping" corresponds to `no_obligations`. Legacy law-level `duty_type = Obligation` (pre-DRRP) counts as holder unknown, not as Making.

## Worked examples

| Provision | Actors (type/position) | DRRP |
|---|---|---|
| EPA 1990 s.20(7): "It shall be the duty of each enforcing authority— (a) to secure that the registers … are available … for inspection by the public" | Enforcement authority Obligation/active; Public Liberty/active (inferred) | **Responsibility** + **Right** |
| Communications Act 2003 s.108(6): "OFCOM must make the register available for public inspection" | OFCOM Obligation/active; Public Liberty/active (inferred) | Responsibility + Right |
| Water Act 1989 s.82(2)(c): Minister may … | Minister Liberty/active; Company none/counterparty | **Power** |
| Medicines Act 1968 s.97E(3) | Responsible Undertaking Obligation/active; Driver none/beneficiary | **Duty** |
| "The authority shall serve a notice on the operator requiring…" | Authority Obligation/active; Operator none/counterparty | Responsibility only (no Right) |
| "The register shall be kept at the principal office" (passive, no holder) | none | raw Obligation, holder unknown |
| "Records shall be kept for five years" (passive, no holder) | none | raw Obligation, holder unknown |
| "Every workplace shall be ventilated…" (thing-subject) | none (holder via #60, e.g. the employer in the stem) | raw Obligation, holder unknown |
| "A notice shall be treated as served if it is sent by post…" (deeming) | none | no type, no DRRP |
| "A person guilty of an offence under this section is liable…" | Person none/mentioned | no DRRP |

## Conformance: fractalaw vs this spec (2026-09-29)

| Area | Status | Action |
|---|---|---|
| Layer 3 holder class | ❌ `law_drrp::is_government_actor` checks the `Gvt*`/`EU:` prefix, so `Crown`, `HM Forces` and `Spc: Notifying Authority` roll up as governed (Duties/Rights instead of Responsibilities/Powers) | Use the dictionary `type`; one shared function for the roll-up and the #67 access rule |
| Layer 1 `Rule` | ⚠️ never emitted (the thing-subject tier already maps to `Obligation`, `duty_type.rs`), but `DutyType::Rule` and its branch in `pipeline.rs` remain | Remove the variant and the dead branch; rename the tier's family (thing-subject) so it isn't read as a type |
| Layer 1 non-active type | ❌ non-active actors carry the provision's type (44,275 of 99,728 OL signals) | Normalise to `none` in reconcile/backfill |
| Layer 4 union | ✅ active-actor union (`bb0d4ff`); raw fallback = holder unknown | none |
| Layer 5 roll-up | ❌ counts every actor's type, whatever the position: holder lists include beneficiaries/counterparties; 9 laws are Making only via non-active Obligations | Count active actors only; dry run and review before publishing |
| Prompts (SLM/LLM) | ⚠️ ask for "the DRRP type of the provision for this actor" | Align to "what this actor holds"; check against the benchmarks (#65) |
| `HM Forces` class | ✅ dictionary now `type: government` (matches legal) | Existing rows take effect on the next roll-up/backfill |
| Legal docs | ⚠️ FUNCTION_VALUES out of date; Housekeeping ↔ no_obligations; legacy OL = holder unknown | Legal aligns and links here |

Tests follow the tables: one table-driven case per row of layers 4–5 and per special case.
