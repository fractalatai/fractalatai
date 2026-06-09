# Session: Actor Labels — Let the LLM Name, Dictionary Match

## Context

**Prior session**: `.claude/sessions/cascade/06-09-26-production-v01.md`
**Trigger**: Sertantai feedback on MHR publish + analysis of 24 invented labels

## The Insight

The LLM "invented" labels that fall into two categories:
1. **Format mismatches** (`Org_Employer`) — our label format confused the LLM
2. **Genuine discoveries** (`water undertaker`, `liquidator`, `special negotiating body`) — real legal actors not in our dictionary

Current approach: force the LLM to use our exact dictionary labels -> LLM struggles with format, misses actors not in the dictionary.

**New approach**: Let the LLM name actors freely in its own words, then:
1. **Fuzzy match** against the dictionary — `employer` -> `Org: Employer`, `HSE` -> `Gvt: Agency: Health and Safety Executive`
2. **Flag unmatched** as discoveries — new actors to review and potentially add to the dictionary
3. **No format constraints** in the prompt — the LLM uses natural language actor names

## Why This Matters

- The actor dictionary was hand-built from UK domestic law. EU retained law introduces actors we didn't anticipate (`Notified Body`, `downstream user`, `economic operator`)
- The dictionary will always lag behind the corpus. The LLM sees the text — it knows who the actors are
- Forcing exact label match costs quality (the LLM focuses on matching format instead of understanding the provision)
- Discoveries feed back into the dictionary — the corpus teaches us what actors exist

## Implementation — Complete

### Step 1: Actor Dictionary YAML (committed `73e3101`)

Created `docs/actor-dictionary.yaml` — 92 entries with canonical labels + trigger phrases. Single source of truth, version-controlled in git.

### Step 2: Python Actor Matcher (committed `73e3101`)

`.claude/skills/actor-match/scripts/actor_match.py` — `ActorMatcher` class with 2-pass matching:
- Pass 1: exact trigger match (order-sensitive, specific before generic)
- Pass 2: substring containment (longest trigger first for specificity)
- 15/15 tests passing

### Step 3: Rust Actor Matcher — Wired into Pipeline

Ported the matcher to Rust in `crates/fractalaw-cli/src/main.rs`:

**`ActorMatcher` struct**: Loads `docs/actor-dictionary.yaml` via `serde_yaml`. Same 2-pass matching as the Python version. Added `is_government()` helper for DRRP decomposition.

**`parse_tier3_actors()` rewritten**: Now takes `&ActorMatcher` instead of `&HashSet<&str>`. Every LLM label is resolved through the dictionary matcher — canonical matches get `label_source: "canonical"`, unmatched get `label_source: "invented"`. The `relates_to` field is also resolved through the matcher.

**Tier 2 prompt updated**: Removed friendly label mapping (`Org_Employer` format) and "Use the EXACT actor labels above" constraint. Now: *"Name each actor mentioned in the provision using natural language (e.g. 'employer', 'HSE', 'inspector', 'local authority')."*

**Tier 3 prompt updated**: Removed pre-built actor list and "Use the EXACT actor labels listed above — do not rename or paraphrase them". Now: *"Name each actor mentioned in or implied by this provision using natural language."*

**Bug fix**: Tier 2 government classification used `a.label.starts_with("Gov:")` (wrong prefix) — replaced with `matcher.is_government()` which uses the dictionary's category field.

### Step 4: Zenoh Dictionary Endpoint

Added `fractalaw/@{tenant}/dictionary/actors` as a zenoh resource:

**`crates/fractalaw-sync/src/zenoh_sync.rs`**:
- `keys::dictionary_actors(tenant)` — key expression
- `publish_dictionary(&yaml_bytes)` — puts raw YAML at the key (for publish sessions)
- `serve_dictionary(yaml_bytes)` — declares a queryable that responds on-demand (for `sync watch`)

**`crates/fractalaw-cli/src/main.rs`**:
- Both `sync publish` and `sync publish --provisions` publish the dictionary alongside taxa data
- `sync watch` declares a persistent queryable that serves the dictionary for the session lifetime

Sertantai can now:
- Subscribe to `fractalaw/@dev/dictionary/actors` to get the YAML on publish
- Query it on-demand in watch mode
- Use the YAML triggers for its own client-side actor matching

### Step 5: Tests (18/18 passing)

Updated all existing tests to use natural-language inputs:
- `parse_tier3_actors_canonical_labels` — `"employer"` -> `"Org: Employer"`
- `parse_tier3_actors_natural_language_resolved` — `"responsible person"` -> `"Ind: Responsible Person"`
- `parse_tier3_actors_invented_label` — `"water undertaker"` -> stays as-is (discovery)
- `parse_tier3_actors_relates_to` — `"employee"` in relates_to -> `"Ind: Employee"`

New matcher tests:
- `actor_matcher_exact_triggers` — employer, HSE, inspector, local authority
- `actor_matcher_substring_containment` — "the enforcing authority" -> `Gvt: Authority: Enforcement`
- `actor_matcher_discovery` — water undertaker, liquidator -> None
- `actor_matcher_specificity` — "secretary of state for defence" vs "secretary of state"
- `actor_matcher_is_government` — Gvt/EU categories return true, Org/Ind false

New sync test: `key_dictionary_actors` — verifies key expression format.

## Dependencies Added

- `serde_yaml = "0.9"` — workspace + fractalaw-cli (pure Rust, no C deps)

## Dictionary Location

`docs/actor-dictionary.yaml` — tracked in git. NOT in `data/` (gitignored). The Rust CLI loads it at runtime from the working directory; tests locate it via `CARGO_MANIFEST_DIR`.

## Existing Discoveries (from 24 invented labels)

| LLM label | Likely canonical | Action |
|---|---|---|
| Org_Employer | Org: Employer | Now resolved by matcher |
| water undertaker | NEW | Add to dictionary |
| liquidator | NEW | Add to dictionary |
| special negotiating body | NEW | Add to dictionary |
| Manufacturers | SC: Manufacturer | Now resolved by matcher |
| Importers | SC: Importer | Now resolved by matcher |
| competent national authorities | Gvt: Authority | Now resolved by matcher (substring) |
| young people | Ind: Person (or NEW?) | Review |

## What's Changed End-to-End

**Before**: LLM told "use EXACT labels" -> struggles with format -> invents `Org_Employer` -> marked `invented` -> sertantai can't use it

**After**: LLM told "name actors naturally" -> outputs `employer` -> matcher resolves to `Org: Employer` -> marked `canonical` -> sertantai uses it. Genuinely new actors -> marked `invented` -> discovery queue for dictionary expansion.

### Step 6: Dictionary Expansion from Corpus Discoveries

Ran the 24 existing `agentic_unvalidated` provisions (31 invented labels, 18 unique) through the matcher. Added 11 new entries to the dictionary from genuine discoveries:

| New entry | Category | Source |
|---|---|---|
| `Spc: Liquidator` | Spc | UK_uksi_2014_1639 insolvency |
| `Spc: Administrator` | Spc | UK_uksi_2014_1639 insolvency |
| `Spc: Receiver` | Spc | UK_uksi_2014_1639 insolvency |
| `Spc: Trustee in Bankruptcy` | Spc | UK_uksi_2014_1639 insolvency |
| `EU: Certification Body` | EU | UK_eur_2008_304 |
| `EU: Central Management` | EU | UK_eudr_2009_38 works councils |
| `EU: Special Negotiating Body` | EU | UK_eudr_2009_38 works councils |
| `Ind: Young Person` | Ind | UK_eudr_1994_33 young workers |
| `Org: Trade Association` | Org | UK_uksi_1999_1148 |
| `Svc: Water Undertaker` | Svc | UK_uksi_1999_1148 water industry |
| `Gvt: Authority` triggers expanded | Gvt | Added `competent national authorities` |

After expansion: 15/16 unique invented labels now resolve. Only remaining discovery: `producers of electricity from high-efficiency cogeneration` (too sector-specific for dictionary).

Dictionary grew from 92 to 103 entries. Python tests: 18/18. Rust tests: 19/19.

### Housekeeping

- **LanceDB**: 161,888 rows, 1 fragment, 1 version, 452 MB — clean, no compaction needed
- **Disk**: cleared 7.5 GB from caches (ccache, Homebrew, pip) — 9.9 GB free

### Step 7: Cleanup — Upgrade agentic_unvalidated Provisions

Ran all 24 `agentic_unvalidated` provisions through the matcher with CamelCase splitting for old `Org_` format labels:
- **23/24 upgraded** to `agentic` at confidence 0.90
- **1 remaining**: `UK_eudr_2012_27:art.Article 15(7)` — `producers of electricity from high-efficiency cogeneration` (genuinely too specific)

### Step 8: Live Validation — Unconstrained Gemini Prompt

Tested on `UK_uksi_2003_164` (Environmental Assessment SI) with `TIER2_PROVIDER=gemini`:

- 43 provisions classified by Gemini with unconstrained prompt
- **29 fully validated** (all labels canonical) — `agentic` at 0.90
- **15 with invented labels** — provisions with descriptive/role-based actors (e.g. "the entity making a determination under this regulation"), correctly flagged as `agentic_unvalidated`
- Position distribution: 34 active, 19 counterparty, 7 beneficiary, 7 mentioned
- Common resolved labels: `Gvt: Agency` x25, `Gvt: Minister` x16, `SC: Applicant` x12, `Ind: Person` x12

**Verdict**: unconstrained prompts work well. Gemini outputs clean natural language that the matcher resolves. Invented labels are genuine discoveries (descriptive role actors), not format bugs.

Added 3 more entries from this test: `Spc: Appellant`, `Spc: Licence Holder`, `Gvt: Consultation Body`. Dictionary now at **105 entries**.

### Step 9: Zenoh Dictionary Stream — Definitive Solution

**Problem**: Sertantai queried the dictionary at startup via `get`, but fractalaw's publish session had already exited. Timing never lined up.

**Solution**: Two complementary patterns:
- `sync publish`: `put` the dictionary YAML (fires sertantai's subscriber → auto-reload ETS)
- `sync watch`: queryable (responds to sertantai startup `get` queries)
- Sertantai falls back to bundled YAML snapshot if neither is available

**Sertantai changes** (`lib/sertantai_legal/legal/actor_dictionary.ex`):
- Added `subscribe_to_updates/0` — declares a Zenoh subscriber on `fractalaw/@{tenant}/dictionary/actors`
- Added `handle_info(%Zenohex.Sample{})` — parses YAML payload, repopulates ETS
- Subscriber declared in `init/1` alongside the initial `load_dictionary/0` call

**Result**: `[info] [ActorDictionary] Reloaded 105 actors from Zenoh publish` — one reload per publish session, not per law.

### Step 10: Full QQ Corpus Publish

Published 76,315 provisions across 165/274 QQ laws (109 had no enriched provisions — not yet ingested via LAT). Dictionary and all provision taxa received by sertantai.

## Session Complete

All items delivered:
1. Actor dictionary YAML — 105 entries, version-controlled in `docs/`
2. Python + Rust actor matchers — 2-pass matching (exact trigger → substring), 18/18 + 19/19 tests
3. Tier 2/3 prompts rewritten — unconstrained natural language, matcher resolves post-hoc
4. Zenoh dictionary endpoint — `put` on publish (fires subscriber), queryable on watch (startup query)
5. Sertantai integration — subscriber auto-reloads on publish, snapshot fallback
6. Dictionary expanded from corpus discoveries (92 → 105 entries)
7. 23/24 agentic_unvalidated provisions upgraded to agentic
8. Full QQ corpus published (76,315 provisions, 165 laws)

## Next Steps

1. Discovery pipeline: accumulate invented labels across corpus, periodic review → dictionary expansion
2. Enrich the 109 QQ laws with no provisions (need LAT ingestion first)
3. RTX 3090 hardware upgrade for local 12B model

## References

- Actor dictionary: `docs/actor-dictionary.yaml`
- Python matcher: `.claude/skills/actor-match/scripts/actor_match.py`
- Sertantai briefing: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
- Gemini review of approach: `docs/reviews/gemini-actor-labels-review-20260609.md`
- Prior session: `.claude/sessions/cascade/06-09-26-production-v01.md`
