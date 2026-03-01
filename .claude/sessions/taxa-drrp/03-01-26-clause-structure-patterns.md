# Session: Clause Structure Pattern Extraction

**Date**: 2026-03-01
**Objective**: Identify standard structural patterns in extracted `clause_refined` text, then build regex/functions to decompose clauses into structured components automatically.

## Context

The taxa/DRRP parser (`fractalaw-core/src/taxa/`) extracts `clause_refined` text from UK ESH legislation provisions. These clauses are "legal word salad" but contain recurring structural components:

- **Applicability** — who/what the clause applies to, conditions
- **Actor** — the duty-holder (employer, operator, person, etc.)
- **Modal verb** — shall, must, may, shall not, etc.
- **Modifier** — "so far as is reasonably practicable", "where necessary", etc.
- **Action/Obligation** — what must be done

The goal is to decompose `clause_refined` into these structured fields automatically.

## Objective 1: Pattern Discovery

Analyze real clause data to identify how many distinct structural patterns exist and whether a single decomposition covers everything or we need a handful.

---

## Corpus Analysis

### Sample clauses by sub-type (from LanceDB, high-confidence Governed provisions)

#### GeneralDuty
```
"The master of the vessel and the operator of the berth [...] shall ensure that the safety
 precautions in the list referred to in paragraph (1) are carried out."

"Every employer shall ensure that where the presence of more than one risk to health or
 safety makes it necessary for his employee to wear or use simultaneously more than one
 item of personal protective equipment, such equipment is compatible [...]"

"The duty holder shall ensure that, where necessary for the health and safety of persons—
 (a) comprehensible instructions on procedures to be observed on the offshore installation
 are put in writing;"
```

#### SfairpDuty
```
"Each employer shall ensure, so far as is reasonably practicable, the safety of the
 employer's employees in respect of harm caused by fire in the workplace."

"Every person who has to any extent control of [...] shall ensure that, so far as is
 reasonably practicable, nothing in the manner in which that substance is handled is such
 as might create a risk [...]"

"The well-operator shall ensure that a well is so designed and constructed that, so far as
 is reasonably practicable— (a) it can be suspended or abandoned in a safe manner;"
```

#### Prohibitive
```
"No person shall operate an installation or mobile plant after the prescribed date [...]
 except under and to the extent authorised by a permit [...]"

"A person must not accept a consignment of eels unless it is accompanied by—
 (a) a certificate prepared under regulation 5 or 6;"

"A person must not damage, interfere with, obstruct or do anything that impedes the
 passage of eels through an eel pass."
```

#### InformationDuty
```
"Every employer shall provide any person whom he has employed under a fixed-term contract
 of employment with comprehensible information on— (a) any special occupational
 qualifications or skills [...]"

"Any person who — (a) designs for another any pressure system [...]; or (b) supplies [...]
 any pressure system [...], shall provide sufficient written information concerning its
 design, construction, examination, operation and maintenance [...]"
```

#### RiskAssessment
```
"Every employer shall make a suitable and sufficient assessment of— (a) the risks to the
 health and safety of his employees to which they are exposed whilst they are at work; [...]"

"Where a dangerous substance is or is liable to be present at the workplace, the employer
 shall make a suitable and sufficient assessment of the risks to his employees [...]"
```

#### TrainingDuty
```
"Where an employer is required to ensure that personal protective equipment is provided to
 an employee, the employer shall also ensure that the employee is provided with such
 information, instruction and training as is adequate and appropriate [...]"

"Every employer shall ensure that each of his employees receives suitable and sufficient
 instruction and training in the meaning of safety signs [...]"
```

#### Prescriptive (general obligation)
```
"An operator of a landfill must keep records containing the following information [...]"

"Any person who constructs, alters or maintains a dam or structure must first notify
 the Agency."

"A responsible person must immediately notify the Agency of any obstruction [...]"

"Where an operator fails to comply [...], the regulator shall serve a notice on the
 operator specifying the relevant requirement [...]"
```

#### Enabling (rights/powers)
```
"An operator of an installation [...] may apply to the regulator for the variation of
 the conditions of his permit."

"Where this regulation applies, the operator may— (a) if he has ceased or intends to
 cease operating [...], apply to the regulator to surrender the whole permit;"
```

---

## Observed Patterns

### Pattern 1: Direct — `[Actor] [modal] [action]`
The simplest and most common. Actor is the grammatical subject, immediately followed by modal verb.

```
"Every employer shall ensure that [...]"
"A responsible person must immediately notify the Agency [...]"
"An operator of a landfill must keep records [...]"
"A person must not damage, interfere with [...]"
```

**Structure**: `{actor} {modal} {action}`

### Pattern 2: Qualified Direct — `[Actor] [modal] [modifier] [action]`
Same as Pattern 1 but with a qualifier between modal and core action.

```
"Each employer shall ensure, so far as is reasonably practicable, [action]"
"The duty holder shall ensure that, where necessary for [scope], [action]"
"The well-operator shall ensure that [...], so far as is reasonably practicable— [action]"
```

**Structure**: `{actor} {modal} {modifier/qualifier} {action}`

### Pattern 3: Conditional Lead — `[Condition], [Actor] [modal] [action]`
A `Where/When/If/Subject to` clause precedes the actor-modal-action core.

```
"Where a dangerous substance is [...] present at the workplace, the employer shall make [...]"
"Where an employer is required to ensure [...], the employer shall also ensure [...]"
"Where this regulation applies, the operator may— [...]"
"Where an operator fails to comply [...], the regulator shall serve a notice [...]"
```

**Structure**: `{condition}, {actor} {modal} {action}`

### Pattern 4: Reverse Duty — `It shall be the duty of [Actor] to [action]`
Classic HSWA-style formulation where the modal precedes the actor.

```
"It shall be the duty of each licensing authority to establish and maintain a register [...]"
"It shall be the duty of a waste regulation authority to comply with any direction [...]"
```

**Structure**: `It shall be the duty of {actor} to {action}`

### Pattern 5: Compound Actor — `[Actor1] and [Actor2] [modal] [action]`
Multiple actors sharing an obligation.

```
"The master of the vessel and the operator of the berth [...] shall ensure [...]"
"...the operator and the proposed transferee shall jointly make an application [...]"
```

**Structure**: `{actor1} and {actor2} {modal} {action}`

### Pattern 6: Passive/Impersonal — `[thing] shall be [done]` (no clear actor subject)
The obligation is on a thing rather than a named actor; actor may appear in a `by` phrase.

```
"Any notice [...] shall be duly given to, or served on, the secretary or clerk [...]"
"...such equipment is compatible and continues to be effective [...]"
```

**Structure**: `{subject} {modal} be {past-participle} [by {actor}]`

---

## Assessment

Most clauses fit into **3–4 core patterns**:

| # | Pattern | Estimated | Actual (n=3000) | Complexity |
|---|---------|-----------|-----------------|------------|
| 1 | Direct: Actor + modal + action | ~40% | **70.9%** | Low |
| 2 | Qualified: Actor + modal + qualifier + action | ~15% | **12.8%** | Medium |
| 3 | Conditional: Where/When..., Actor + modal + action | ~30% | **10.9%** (8.2% + 2.7% with qualifier) | Medium |
| 4 | Reverse duty: "It shall be the duty of..." | ~5% | **0.7%** | Low |
| 5 | Compound actor | ~5% | (variant of 1/3, not separately counted) | Low |
| 6 | Passive/impersonal | ~5% | **2.6%** | High |
| — | Unmatched | — | **2.0%** (title text, amendment instructions) | — |

Patterns 1–3 cover **95%** of clauses. P1 alone is 71%. P4 is rare (0.7%) but a known special case already handled by the v2 parser's reverse anchor. Passive (6) is the hardest but uncommon at 2.6%. Unmatched are mostly title/preamble text and amendment instructions that shouldn't have `duty_sub_type` set.

---

## Empirical Data: Enumerable Sets

### Modal Verbs (from 2,000 enriched clauses)

| Modal | Count | Notes |
|-------|-------|-------|
| may | 698 | permissive (but beware epistemic "may") |
| shall | 651 | mandatory (traditional drafting) |
| must | 452 | mandatory (modern drafting) |
| shall ensure | 252 | mandatory + ensure (very common in ESH) |
| shall not | 233 | prohibitive |
| is required to | 28 | mandatory (periphrastic) |
| may not | 20 | prohibitive-permissive |
| must not | 20 | prohibitive (modern) |
| are required to | 5 | mandatory (plural) |

**9 distinct values** — clean enum, no ambiguity.

### Governed Actor Labels (from enriched corpus, top 20)

| Actor | Count | Category |
|-------|-------|----------|
| Ind: Person | 767 | Individual |
| Org: Employer | 319 | Organisation |
| Operator | 187 | Specialist role |
| Ind: Employee | 178 | Individual |
| Public | 80 | Collective |
| Ind: Duty Holder | 71 | Individual |
| Org: Owner | 60 | Organisation |
| SC: Supplier | 54 | Supply Chain |
| Spc: Inspector | 39 | Specialist |
| Org: Company | 37 | Organisation |
| Ind: Self-employed Worker | 35 | Individual |
| Ind: Competent Person | 33 | Individual |
| Ind: Responsible Person | 32 | Individual |
| Org: Occupier | 24 | Organisation |
| Ind: Manager | 20 | Individual |
| Ind: Worker | 19 | Individual |
| Ind: User | 19 | Individual |
| Org: Landlord | 16 | Organisation |
| SC: T&L: Carrier | 10 | Supply Chain |
| Ind: Supervisor | 10 | Individual |

**~29 governed labels** — already enumerated in `actors.rs`. Stable set.

### Qualifier/Modifier Phrases (from enriched corpus)

| Qualifier | Count | Category |
|-----------|-------|----------|
| SFAIRP ("so far as is reasonably practicable") | 39 | Standard of care |
| SFAIP ("so far as is practicable") | 2 | Standard of care |
| suitable and sufficient | 27 | Standard of adequacy |
| adequate and appropriate | 1 | Standard of adequacy |
| as soon as reasonably practicable | 30 | Timing |
| as soon as practicable | 17 | Timing |
| immediately | 22 | Timing |
| without delay | 1 | Timing |
| where necessary | 8 | Conditionality |
| where appropriate | 16 | Conditionality |
| as is necessary | 8 | Conditionality |
| as is appropriate | 3 | Conditionality |
| as may be necessary | 3 | Conditionality |
| in writing | 126 | Form |
| from time to time | 15 | Frequency |
| at reasonable intervals | — | Frequency |
| at all [reasonable] times | 11 | Availability |
| at any reasonable time | 6 | Availability |
| in so far as | 15 | Scope limiter |
| so far as is necessary | 4 | Scope limiter |

**~20 distinct phrases**, clustering into **6 semantic categories**. Only ~15% of clauses contain a recognised qualifier — most obligations are unqualified.

---

## Schema Recommendations

### Principle: Enum where membership is fixed, free text where provision-specific

Two fields have **closed, enumerable membership** and should be stored as enum/tag values:
- **Modal** — 8 values, fully known (see Decision 1)
- **Qualifier** — ~12 standard phrases, groupable into 6 categories

Two fields are inherently **provision-specific free text**:
- **Applicability** — the conditional preamble ("Where a dangerous substance is present...")
- **Action** — the core obligation ("make a suitable and sufficient assessment of the risks...")

Actor data is already captured in existing `governed_actors`/`government_actors` columns (see Decision 2).

### Struct & enums: `ClauseStructure`

```rust
/// Modal verb indicating the nature of the obligation.
/// 8 values — captures obligation strength only, not the action verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modal {
    Shall,        // "shall [verb]"
    Must,         // "must [verb]"
    May,          // "may [verb]" (permissive)
    ShallNot,     // "shall not [verb]"
    MustNot,      // "must not [verb]"
    MayNot,       // "may not [verb]"
    IsRequiredTo, // "is/are required to [verb]"
    HasDutyTo,    // "has a duty to" / "it shall be the duty of"
}

/// Standard legal qualifier/modifier on the obligation.
/// Multiple may apply to a single clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Qualifier {
    // Standard of care
    Sfairp,                   // "so far as is reasonably practicable"
    Sfaip,                    // "so far as is practicable"
    SuitableAndSufficient,    // "suitable and sufficient"
    AdequateAndAppropriate,   // "adequate and appropriate"

    // Timing
    Immediately,              // "immediately"
    AsSoonAsReasonablyPracticable, // "as soon as [is] reasonably practicable"
    AsSoonAsPracticable,      // "as soon as [is] practicable"
    WithoutDelay,             // "without delay" / "without undue delay"

    // Conditionality
    WhereNecessary,           // "where necessary"
    WhereAppropriate,         // "where appropriate"

    // Form
    InWriting,                // "in writing"

    // Scope
    InSoFarAs,                // "in so far as" / "so far as is necessary"
}

/// Decomposed structure of a legal clause — 4 fields.
/// Actor data comes from existing governed_actors/government_actors columns.
pub struct ClauseStructure {
    /// Conditional preamble, if any ("Where X applies...", "Subject to Y...")
    /// Free text — provision-specific.
    pub applicability: Option<String>,

    /// The modal verb. Enum — 8 fixed values.
    pub modal: Modal,

    /// Standard legal qualifiers modifying the obligation.
    /// Enum — ~12 fixed values. Vec because multiple can co-occur.
    pub qualifiers: Vec<Qualifier>,

    /// The core obligation/action. Free text — provision-specific.
    /// e.g. "ensure that the health, safety and welfare of all employees..."
    pub action: String,
}
```

### Storage (Arrow schema / LanceDB columns)

| Column | Arrow Type | Enum? | Notes |
|--------|-----------|-------|-------|
| `clause_modal` | `Utf8` (dictionary-encoded) | Yes | "Shall", "Must", "May", etc. |
| `clause_qualifiers` | `List<Utf8>` (dictionary-encoded) | Yes | ["Sfairp", "InWriting"] |
| `clause_applicability` | `Utf8` | No | Free text, nullable |
| `clause_action` | `Utf8` | No | Free text, the core obligation |

### Design rationale

1. **Modal as enum, not string** — There are exactly 8 modals. Storing as enum enables filtering ("show me all 'shall not' clauses") and aggregation ("what % of duties use 'must' vs 'shall'?") without text matching. Action verbs (ensure, make, provide, etc.) belong in the `action` field.

2. **Qualifiers as `Vec<Qualifier>`, not embedded in action text** — Qualifiers are terms of art with specific legal meaning (SFAIRP has decades of case law). Extracting them as tags enables filtering ("show me all SFAIRP duties") and analysis ("what % of employer duties are qualified?"). A clause can have zero, one, or multiple qualifiers (e.g. SFAIRP + InWriting). Only ~15% of clauses have any qualifier, so the field is often empty — that's fine and informative.

3. **Applicability as free text** — Conditions are too varied to enumerate ("Where a dangerous substance is present...", "Subject to paragraph (4)...", "Where the employer employs five or more employees..."). Extracting them as a separate field still adds value: consumers can display them distinctly, and downstream AI can classify them further.

4. **Action as free text** — The core obligation is inherently provision-specific. No enumeration possible. But isolating it from the preamble/qualifier clutter makes it much more readable and analysable.

---

## Next Steps

1. **Validate frequency estimates** — run the structural patterns against the full enriched corpus and count matches per pattern
2. **Create `clause_structure.rs`** — Modal enum, Qualifier enum, ClauseStructure struct, `decompose()` function
3. **Wire into `parse_v2()`** — call `decompose()` after `extract_clause()`, add `ClauseStructure` to `TaxaRecord`
4. **Test on corpus** — measure decomposition coverage and identify edge cases

---

## Decisions

### Decision 1: `ShallEnsure` → merged into `Shall`

**Question**: Is "shall ensure" a distinct modal or `Shall` + action verb "ensure"?

**Data**: "shall" pairs with 30+ different verbs in the corpus:
- shall ensure: 252 (25.0%)
- shall not: 233 (23.1%)
- shall be: 201 (19.9%)
- shall take/make/provide/notify/keep/comply/give/...: 324 (32.1%)

"ensure" is the most common single verb, but it's one of many. The modal expresses **obligation strength** (mandatory/permissive/prohibitive), not the action. "shall ensure", "shall make", "shall notify" all carry the same obligation strength — the verb belongs in the `action` field.

**Decision**: Remove `ShallEnsure` from the Modal enum. Use `Shall` and put "ensure that..." into `action`. Modal enum becomes **8 values**:

```rust
pub enum Modal {
    Shall,        // "shall [verb]"
    Must,         // "must [verb]"
    May,          // "may [verb]" (permissive)
    ShallNot,     // "shall not [verb]"
    MustNot,      // "must not [verb]"
    MayNot,       // "may not [verb]"
    IsRequiredTo, // "is/are required to [verb]"
    HasDutyTo,    // "has a duty to" / "it shall be the duty of"
}
```

### Decision 2: No separate `clause_actor` field

**Question**: Should `clause_actor` be a new field or reuse `governed_actors`/`government_actors`?

**Decision**: Neither — drop it entirely. Same taxonomy, same labels, and subject-position actor is already implicitly captured by `duty_family` + v2 parser anchoring. A separate field would duplicate existing data with no analytical benefit.

The `ClauseStructure` struct drops `actor` — consumers use the existing `governed_actors`/`government_actors` columns alongside the decomposed clause.

Updated struct (4 fields, not 5):
```rust
pub struct ClauseStructure {
    pub applicability: Option<String>,  // free text
    pub modal: Modal,                   // 8-value enum
    pub qualifiers: Vec<Qualifier>,     // ~12-value enum, multi
    pub action: String,                 // free text
}
```

Storage columns: `clause_modal`, `clause_qualifiers`, `clause_applicability`, `clause_action`.

### Decision 3: Run at enrichment time, but keep function standalone-capable

**Question**: Enrichment time or separate pass?

**Decision**: At enrichment time — the `MatchSpan` (actor_start, modal_start, modal_end) is available inside `parse_v2()`, making the split into applicability/modal/action precise without re-discovering the modal position. Single pass, no extra I/O.

However, the decomposition function signature should accept `&str` (+ optional span), not depend on pipeline internals. This means it *can* also be called standalone on stored `clause_refined` text if needed later (e.g. backfill, debugging, REPL exploration). The span is a precision bonus, not a hard requirement — without it, the function re-finds the modal via regex.

```rust
/// Decompose a clause into structured components.
/// `span` is optional — when available (enrichment time), gives precise offsets.
/// Without it (standalone mode), falls back to regex modal detection.
pub fn decompose(clause: &str, span: Option<MatchSpan>) -> Option<ClauseStructure> { ... }
```

**Integration point**: after `extract_clause()` at line 114 of `mod.rs`, before confidence scoring. The `ClauseStructure` gets added to `TaxaRecord`.

### Decision 4: New module `clause_structure.rs` in `fractalaw-core/src/taxa/`

Follows existing module-per-concern pattern (`clause_refiner.rs`, `confidence.rs`, etc.). Contains:
- `Modal` enum
- `Qualifier` enum
- `ClauseStructure` struct
- `decompose()` function
- Qualifier regex patterns (compiled via `LazyLock`, matching the project's existing pattern)

---

## Implementation Results

### Corpus Coverage (n=3,000 enriched clauses)

| Metric | Value |
|--------|-------|
| Successfully decomposed | **2,937 (97.9%)** |
| Failed (no modal/no action) | 63 (2.1%) |

Failures are concentrated in government sub-types (Enabling 25, ParliamentaryReporting 18, Fees 13) — these are often title/preamble text or amendment instructions without a clear modal verb.

### Modal Distribution

| Modal | Count | % |
|-------|-------|---|
| Shall | 1,361 | 46.3% |
| May | 690 | 23.5% |
| Must | 498 | 17.0% |
| ShallNot | 293 | 10.0% |
| MustNot | 29 | 1.0% |
| MayNot | 25 | 0.9% |
| HasDutyTo | 22 | 0.7% |
| IsRequiredTo | 19 | 0.6% |

### Applicability (conditional preamble)

| | Count | % |
|--|-------|---|
| Has applicability | 253 | 8.6% |
| No applicability (direct) | 2,684 | 91.4% |

### Qualifier Distribution

Clauses with any qualifier: **477 (15.9%)**

| Qualifier | Count |
|-----------|-------|
| InWriting | 172 |
| InSoFarAs | 120 |
| Sfairp | 75 |
| AsSoonAsReasonablyPracticable | 49 |
| Immediately | 40 |
| SuitableAndSufficient | 38 |
| AsSoonAsPracticable | 28 |
| WhereNecessary | 25 |
| WhereAppropriate | 21 |
| WithoutDelay | 8 |
| Sfaip | 3 |
| AdequateAndAppropriate | 1 |

### Files Changed

| File | Change |
|------|--------|
| `fractalaw-core/src/taxa/clause_structure.rs` | **New** — Modal, Qualifier, ClauseStructure, decompose(), 12 tests |
| `fractalaw-core/src/taxa/mod.rs` | Added `clause_structure` module, `clause_structure` field on TaxaRecord, wired into `parse_v2()` |

---

## Session Closed

**Commit**: `4a007e3` — pushed to `origin/master`
**Status**: Complete. All four next steps delivered. 97.9% corpus coverage, 12 tests passing, pre-commit and pre-push hooks green.
