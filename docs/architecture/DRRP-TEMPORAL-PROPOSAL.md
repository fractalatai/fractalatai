# DRRP over time: "as made" and "as amended" (PROPOSAL)

- **Status:** draft, revised after reviews by sertantai-legal and Gemini (2026-09-30). Section L (the LAT change contract) was added and reviewed on 2026-10-01. D1–D4 were approved by Jason on 2026-10-01; build per L8.
- **Relates to:** DRRP-CLASSIFICATION.md (#68), especially the "Revoked laws" special case; LAT sync (#62); correlatives (#72).
- **Nothing is implemented.** This document proposes a model and asks questions.

## The problem

A user should be able to ask "what are my obligations?" and get the right answer without first filtering out repealed or revoked laws. Today, one classification per provision tries to answer two different questions:

1. **What does the law impose now?** This is the compliance question, and it's what users need day to day.
2. **What did the law impose?** This covers the law's character, whether it was ever Making (legal's `is_making`), enforcement history, audits of past periods, and successor tracing.

The 2026-09-28 ruling (revocation doesn't change what a law did while live) answers question 2. It was never meant to stop question 1 reflecting repeal.

Three kinds of law show the tension:

| Law | "What did it impose?" | "What does it impose now?" |
|---|---|---|
| Revoked in one go | Everything it imposed when made | Nothing |
| Repealed piecemeal ("a thousand cuts"), then finally revoked | Everything it imposed when made | Nothing now; while it was being cut, only what was left |
| Partly repealed, still in force | Everything it imposed when made | Its live provisions only |
| Amended (text substituted, provisions inserted) | The original text's relations | The current text's relations, including inserted provisions |

## Proposal: two views of DRRP

Every provision and every law carries DRRP in two views. They use the same classification rules (DRRP-CLASSIFICATION.md) and the same payload shapes.

### View A: `as_amended` (current)

- **The text is the current revised text,** which is what the hub's LAT already holds (legislation.gov.uk `/body/data.xml`).
- **Provision level:**
  - A **repealed or revoked provision** (text removed, shown as dots, or legal marks it repealed) → `none`: no type, no actors and no correlatives. It claims nothing.
  - An **amended provision** is classified on its current text.
  - An **inserted provision** is present and classified.
  - A **prospective provision** (not yet in force) → `none` until commenced. (Question: see Q5.)
- **Law level:** the verdict and holder lists are rolled up from live provisions only.
  - A law revoked in one go → all provisions `none`, so the current verdict is `no_obligations`, or a distinct `revoked` verdict (see Q3).
  - A law cut piecemeal can become `no_obligations` before its final revocation.
- **Queries:** "my obligations" = provisions where I hold an Obligation in `as_amended`. No status filter is needed, because dead law returns nothing on its own.

### View B: `as_made` (historical character)

- **The text is the law as enacted,** legislation.gov.uk `/body/made/data.xml`.
- **Provision level:** classified on the made text. Inserted provisions don't exist in this view; later substitutions don't apply.
- **Law level:** `making_as_made` = the law created at least one Duty or Responsibility when made. That's the 2026-09-28 ruling, and it never changes with status. Legal's `is_making` maps here.
- **Queries:** "did this law impose duties?", "which laws ever imposed duties on employers?", and successor tracing (made duties of the old law against current duties of its replacement).

### Why these two points, and not full point-in-time

legislation.gov.uk offers point-in-time text (`/body/{date}/data.xml`), so the general model is **provision versions with validity intervals**, each classified. "As made" and "as amended" are the two ends of that timeline.

- The **data model** should allow versions (`valid_from`, `valid_to` per provision version), so a date-based view can be added later without a schema change.
- We'd **populate only the two ends now.** Every extra point costs a full classification pass, and no user question needs an intermediate date yet. The exception is audits of past periods (see Q6).

## How to get the "as made" view cheaply

Classifying the made text of every law doubles the work. Most of it can be avoided:

1. **Never amended laws** (no effects in legislation.gov.uk's changes table): as_made = as_amended. Copy the classification; no extra calls.
2. **Amended laws:** match made provisions to current provisions by section id and normalised text (the carry-forward matcher already does this: `scripts/benchmarks/carry_forward_gold.py`).
   - Identical or cosmetically changed → copy the classification.
   - Only provisions whose text differs, or which exist only in the made version, need classifying.
3. **Laws first loaded after revocation, with dotted text** (e.g. UK_uksi_2012_3030): the made text is the only source. Classify it once. It's cheap, because it's rare.
4. **Revoked laws already classified while live** (our existing classification predates revocation): keep that classification as their `as_made` approximation. Flag it `as_made_source = last_live` rather than `made_text` until, or unless, the made text is classified.

## Payload (fractalaw → sertantai-legal)

- **Provision payload:**
  - `drrp_types` / `actors` / `correlatives` = **`as_amended`**. That's what users query, and it's the field legal already stores. Repealed provisions arrive as classified `none` (`[]` types, `[]` actors, with a method), which clears stale values.
  - New: `made: {drrp_types, actors}` only where it differs from as_amended (and `null` where identical, meaning "same as current"). Or a separate `taxa/provisions_made/{law}` key. See Q2.
- **Law payload:**
  - The current holder lists and verdict (`as_amended`).
  - New: `making_as_made` (making / empowering / no_obligations) plus its holder lists, or just the verdict (Q2).
  - Legal's `is_making` resolves from `making_as_made`, so the ruling holds.

## How this changes current rules

- The **"Revoked laws" special case** becomes: status never changes `as_made`, and `as_amended` reflects repeal. Dead laws aren't re-processed for `as_amended`: a fully revoked law's provisions simply flip to `none`, with no model calls.
- The **dotted-text rule:** dotted provisions are `none` in `as_amended` and are never evidence of `no_obligations` in `as_made`.
- **Amendment text (#57):** unchanged. Inserted text belongs to the amended law's `as_amended` view.
- **Gold v2 benchmark:** labels `as_amended`, the current text, as it does now. An `as_made` benchmark isn't needed, because it's the same classifier on different text.

## Reviews and revised design (2026-09-30)

Reviewed by sertantai-legal and Gemini 2.5 Pro (raw: `data/code-review/drrp-temporal-proposal-gemini.md`). Both support the two views. The design below merges the two reviews and supersedes the sections above where they differ.

### R1. Two verdict fields, never one (legal: a hard requirement)
- **`making_enrichment_verdict` = as_made.** It feeds legal's `is_making` (MakingResolver), and its meaning is unchanged. The 2026-09-28 ruling holds: revocation never flips it.
- **`current_verdict` = as_amended:** `making | empowering | no_obligations | revoked` (plus the holder-unknown shape).
- **`revoked`** is distinct from `no_obligations` (both reviews): "no duties now" isn't "never had any".
- Both fields follow the #68 never-NULL rule.

### R2. Provision status comes from legal's LAT, not from dots (both reviews)
- Each LAT row carries `status: in_force | repealed | prospective`, set by legal's CLML parse:
  - repealed from the "[Repealed]"/"[Revoked]" markers, dotted text and repeal commentary;
  - prospective from `Status="Prospective"`.
- Repealed can be backfilled from existing rows; prospective needs a re-parse.
- Fractalaw reads `status` and stops inferring repeal from dots.
- **Savings (Gemini):** a repealed provision kept alive by a savings or transitional clause still applies to some cases. Proposed extra value: `repealed_saved`, or a `saved` flag, if legal's commentary can detect it. Until then, a savings clause is itself classified normally, so the saved obligation surfaces there.

### R3. Treatment per status in as_amended
| status | Provision DRRP (as_amended) | Counted in `current_verdict` / "my obligations" |
|---|---|---|
| `in_force` | classified on current text | yes |
| `repealed` | `none` (no types, actors or correlatives); no model calls | no |
| `prospective` | **decision needed (D1):** classified on its text and flagged, *or* `none` | no (not in force yet) |

### R4. as_made is law-level only, for now (legal; Gemini wanted provision-level)
- Law payload: `making_enrichment_verdict` plus as_made holder lists (names to agree in the spec, e.g. `made_duty_holder` or a `made` sub-object).
- Provision-level as_made is deferred. Every stated use (is_making, "did this law ever impose…", successor tracing) is law-level, and the versioned data model keeps the door open.

### R5. Made text: legal fetches it, for amended laws only
- `/body/made/data.xml`, the same CLML, stored in a separate `legal_articles_made` table with its own lat-made manifest key. The current LAT is never touched.
- Only for **amended laws** (about 800 of 1,069 LAT laws) plus laws first loaded after revocation. For never-amended laws, made = current.
- **No `last_live` approximation** (Gemini): as_made comes only from made text, or from current text when the law was never amended.

### R6. Reuse rule for as_made (Gemini)
- Copy a current classification to as_made only if the provision's text is unchanged (normalised) **and** no definition it depends on has changed.
- In practice: if an interpretation or definition provision of the law changed between made and current, re-classify the provisions that use the defined terms.
- The Q7 sample below measures how often that happens.

### R7. Extent (legal; Gemini wanted jurisdiction-aware DRRP)
- Extent isn't a DRRP dimension. Rows keep `extent_code` (from RestrictExtent), and territorial variants are separate rows (`s.23(3)[E+W]`, `[S]`).
- A provision repealed for one nation only is simply `repealed` on that nation's row.
- User queries filter by jurisdiction.

### R8. Point-in-time (both: not now)
- as_made plus repeal and commencement dates (from legal's amendment annotations) covers today's questions.
- Point-in-time, event-driven re-classification (Gemini's full model) waits for a real audit or enforcement question. The versioned model allows it.
- Retrospective amendments are rare, and we don't design around them.

### Sequencing (legal's suggestion, adopted)
1. Legal adds `status` to LAT rows (repealed backfilled now; prospective with the next re-parse).
2. Fractalaw switches the dotted-text rule to `status`: repealed → none, and they no longer count in the verdict. This delivers "my obligations" without a status filter.
3. Spec the verdict split (R1) and the as_made law-level payload (R4), with legal agreeing field names.
4. **Q7 sample:** legal fetches made text for a sample of amended laws. We measure how many provisions differ, including definition changes (R6), and the cost, before committing to all ~800.
5. Made-text classification for amended laws, then the as_made roll-up.

## L. Change over a law's life: the LAT change contract (2026-10-01, revised after review)

The views (R1–R8) say *what* we hold. This section says *how changes arrive*. Today the LAT sync (#62) sees only text differences between legal's previous and current LAT. It can't tell why a row changed, when the change took legal effect, whether our first copy was the law as made, or which effects legislation.gov.uk hasn't applied yet.

Reviewed by sertantai-legal (with figures from its dev DB) and Gemini 2.5 Pro in two rounds (`data/code-review/drrp-temporal-section-L-gemini.md`, `…-round2.md`). In round 2, Gemini agreed that all four refutations hold and agreed D1–D4 as recommended. Its two remaining gaps (behaviour per status in step 1; interim handling before `cause` exists) are closed in L8. Legal can supply most of the contract from data it already holds.

### L1. Worked lifecycle

| # | Event in the law | What legal's LAT shows | What fractalaw does | as_amended | as_made |
|---|---|---|---|---|---|
| 0 | **Made** and first loaded, unamended | Rows `in_force`/`prospective`; manifest `amended: false` | Classify all rows; version 1 per provision | Live roll-up | = as_amended while unamended |
| 0′ | **First loaded already amended** (usual for older laws) | Current rows; manifest `amended: true` | Classify the current rows (version 1 = first observed, not made). Queue the made-text job (R5) | Live roll-up | From the made text (R5) |
| 1 | **Amendment inserts provisions** | New section_ids; `cause: legislative`, `effective_from`, `changed_by`, `change_id` | New versions; classify the new rows + their stems | Gains their DRRP | Unchanged |
| 2 | **DRRP-laden provisions repealed** | Same ids, `status: repealed`; `cause: legislative` | New version, status only. **No re-parse.** The previous version keeps its classification in history | Those provisions → none; verdict re-rolled | Unchanged |
| 3 | **A provision amended; an actor drops out** | Same id, new text; `cause: legislative` | New version; re-parse it + dependents (L4) | Re-classified; holder lists and correlatives re-rolled | Unchanged |
| 4 | **Whole law repealed** | All rows `repealed` (or law status revoked; possibly `effects_unapplied` while the text is still live) | Status-only versions; no model calls | All none; `current_verdict = revoked` | Unchanged; `is_making` keeps its value |
| 5 | **Renumbered by amendment** (legal: 238 notes) | Legal emits a rename with `cause: legislative` + the note | History carries across ids (rename, not repeal + insert) | Unchanged DRRP under the new id | Unchanged |
| 6 | **Whole Part/Schedule substituted** (51 notes) | Many rows share one `change_id` | One change event, many versions; classify the substituted rows | Re-rolled | Unchanged |
| 7 | **Commenced piecemeal** ("for specified purposes", 2,644 notes) | `status: in_force_partial`, a list of dates, `partial` flag | Classify, flagged partial | Counts, flagged partial (D1) | — |
| 8 | **Extent-specific amendment** | Separate rows per extent (`s.23(3)[S]`; 2,753 rows in 49 laws) | Each row has its own history; **never merged** | Per row | Per row |
| 9 | **Revival** (no notes today) or **sunset/expiry** (rare) | Just another status change (repealed → in_force, or → repealed) | Status-only version | Follows status | Unchanged |

Status is a free state, not one-way.

### L2. The contract: what legal adds

**Per LAT row:**
- `status: in_force | in_force_partial | repealed | repealed_saved | prospective`. `repealed_saved` only where a savings note exists (D2).
- `effective_from`: the latest dated note affecting the row.
  - Legal holds 58,751 amendment notes, plus 17,177 commencement and 24,591 modification notes, all with `affected_sections`.
  - 82% carry a date and 95% cite the instrument, so expect ~18% null.
  - Partial commencement keeps the list of dates.
- `changed_by`: the citing instrument (e.g. S.I. 2006/984).
- `effect`: substituted | inserted | repealed | …
- `change_id`: a hash of (law, normalised note text). Never the F-number, which renumbers when notes are added.
- `extent_code` (exists already).
- These come from a pure, test-driven parser module over the notes legal already stores.

**Per law (manifest):**
- `amended`: made ≠ current.
- `as_of`: legislation.gov.uk's `md_dct_valid_date`, held today for 904 of 1,069 LAT laws.
- `effects_unapplied`: from the changes feed's `applied` flag (legal's LiveStatus already uses it; 669 laws are `revoked_unapplied`).
  - Mapping a cited target to a section_id is best effort.
  - Where it maps, the row's `status` is authoritative over the text; otherwise the effect is reported at law level only.

**Per change** (a change log beside the rename log):
- `cause`, decided **per parse operation, not guessed**, using a new `source_hash` (hash of the fetched CLML) on legal's parse events:
  - **`parser`**: the source CLML is unchanged, so only legal's code changed. This is exact.
  - **`scope`**: a LatScope change (explicit, with its own history).
  - **`correction`**: an admin or fix task (explicit source).
  - **`legislative`**: the source changed **and** there's evidence on the row or its ancestor: a new amendment note, or a status change.
  - **`unattributed`**: the source changed but there's no evidence.
- Legal never sends "unknown".
- Renumbering renames carry `cause: legislative` + the note.

### L3. What fractalaw does with each cause

| cause | Version history | Re-parse |
|---|---|---|
| `legislative`, text change | **New version**; the old one kept with its actors and DRRP | That provision + dependents (L4) |
| `legislative`, status only | New version | No (repealed → none). Prospective → in_force: classify if never classified |
| `parser` | **No version**: the current version is overwritten | Only if the normalised text changed; otherwise carry tier data |
| `scope` | Rows added or removed, no versions | New rows only |
| `correction` | Overwrite the current version | If the text changed |
| `unattributed` | **No version**: overwrite the current version and **flag for review** | If the text changed |

**The default flipped after review:** unattributed changes never create versions. Legal's rename log shows today's churn is mostly parser-driven (19,309 dropped + 1,879 extent_tag + 363 unique_text renames). Treating unknown as legislative would have created ~20,000 false versions.

### L4. Dependents: what else to re-parse when a provision changes

- **Stems and children:** if a stem changes, re-parse its list items. An inserted child changes its stem's content list too.
- **Definitions:** legal already maps terms to provisions: 66K `legislative_definitions` plus `definition_link`. Legal publishes "provisions using term X" per law, and when a definition changes, those provisions are re-parsed.
- **Referenced holders:** provisions citing a changed provision as their holder's source ("regulations under subsection (2)").
- **Cap:** same-law uses plus explicit cross-references only. **Never fan out across the corpus** (e.g. for Interpretation Act terms). Dry-run the count before a large cascade.

### L5. Fractalaw storage and ordering

- **`provision_versions`** (new, append-only): `section_id, version, text_md5, status, extent_code, effective_from, effective_dates[], partial, changed_by, effect, change_id, cause, observed_at, drrp, actors (jsonb snapshot)`.
  - `legislation_text` + `provision_actors` remain the current version.
  - `lat_archive` stays for parser/scope churn and undo.
- **Sequence = observation order.** Fractalaw never composes text: legislation.gov.uk serves the consolidated result, so a late-applied effect simply arrives as the next text. There's nothing to replay (Gemini's objection doesn't apply).
  - `effective_from` is an attribute for date queries. Versions are **never re-slotted** by it (the earlier insert-in-order rule is dropped).
  - We record what we saw and when, and never invent intermediate states.
- **Idempotency:** a change is keyed on (law, section_id, source_hash) + `change_id`. A re-delivered change is a no-op.
- **Verdicts:**
  - `current_verdict` (as_amended) is re-rolled after every applied change;
  - `making_enrichment_verdict` (as_made) is computed once (made text, or version 1 when unamended) and never re-rolled.

### L6. Publishing

Changed provisions + the law level, after each applied change. A status-only repeal publishes the provision as classified none, plus the re-rolled `current_verdict`.

### L7. Deliberately not done

- No reconstruction of history from before first observation (only made + observed onward).
- No classification of repealed text: the last classification survives in history.
- No corpus-wide definition cascades.

### L8. Build order

- **Legal**, after D3/D4:
  1. per-row `status` (including `in_force_partial`, `repealed_saved`);
  2. the structured note parser → `effective_from` / `changed_by` / `effect` / `change_id`;
  3. `source_hash` + `md_dct_valid_date` on parse events → `cause` per operation;
  4. manifest `amended`, `as_of`, `effects_unapplied`;
  5. the change log beside the rename log.
- **Fractalaw**, as each part lands:
  1. read `status` (replaces the dotted-text inference), with defined behaviour for **every** status from day one (Gemini, round 2):

     | status | Provision DRRP | Counted in `current_verdict` / "my obligations" |
     |---|---|---|
     | `in_force` | classified | yes |
     | `in_force_partial` | classified, flagged `partial` | yes (flagged) |
     | `repealed_saved` | classified (last classification kept), flagged `saved` | yes (flagged): it still applies to the saved cases |
     | `prospective` | none until commenced (D1) | no |
     | `repealed` | none, no model calls | no |
     | missing (legal not yet live) | today's behaviour (dotted text still excluded) | as today |

  2. apply changes by cause (L3). **Interim, until legal's L8.3 delivers `cause`:** every change is handled as `unattributed`. That means today's diff-apply (carry tier data; re-parse changed text), and **no versions** are written. History starts only once `cause` arrives, so false history is never created (Gemini, round 2);
  3. the `provision_versions` table;
  4. dependents (L4);
  5. the verdict split + payload (R1/R4).

## L9. Fractalaw step 2: versioning from legal's change log (PROPOSED, 2026-10-01)

Legal's #167 is complete. **L8.5** serves `lat-changes/{law}?since=`, a per-row change log:
- `op_key`, `section_id` / `old_section_id`;
- `change`: text_changed | inserted | removed | renamed | status_changed;
- `cause` **per row**: legislative only with the row's own evidence (a note new in this parse that targets the row or an ancestor, or a status change); otherwise unattributed, or the operation's parser / scope / correction;
- `change_ids`, `op_cause`, `source_hash`.

Renumbering is one `renamed` entry (also in lat-renames, match `renumbered`). `initial` parses log nothing. The log starts empty, with no backfill.

**Plan:**
1. The LAT sync (pull-lat / sync watch) pulls `lat-changes` for each law it applies, since a per-law watermark `changes_through` (like `renames_through`).
2. **Before** `apply_lat_diff` / `apply_lat_status` changes a row, each `legislative` entry snapshots the row's superseded state into `provision_versions`:
   - text, text_md5, status, effective_from, changed_by;
   - drrp_types, actors (jsonb, from provision_actors);
   - plus the entry's `change`, `cause`, `change_ids`, `op_key`, `source_hash` and `created_at`.
   - Current state stays in `legislation_text` / `provision_actors`.
   - History of a provision = its versions (oldest first) + the current row.
3. **Idempotent:** keyed on the change log's stable entry `id`. Legal offers to add it to the payload (additive; needs Jason's go-ahead). The fallback is a unique key on (law_name, coalesce(section_id, old_section_id), op_key, change): `removed` entries have a null `section_id`, and Postgres treats nulls as distinct. Legal confirms one entry per row per op (merge renames need equal text; a renumbering pair is one `renamed` entry; `status_changed` only where the row isn't already text_changed or inserted). A re-delivered log is a no-op.
   **Atomic (Gemini):** the snapshot and the apply for a law run in **one transaction** with `apply_lat_diff` / `apply_lat_status`, so history can't be lost to a half-applied change. On any failure the whole law rolls back and the watermark doesn't advance, so a retry resumes cleanly.
4. **Renamed:** the snapshot is keyed on `old_section_id`, plus a `renamed_to` pointer, so history follows the provision across ids. Tier data is already carried by the rename pass.
5. **inserted:** no snapshot (nothing was superseded). **removed:** a snapshot of the last state.
6. parser / scope / correction / unattributed entries: **no version**. They're applied as today (L3).
   **Status changes arriving only through `status_hash`** (legal recomputing status outside a parse, e.g. `mix lat.status` after a rule fix) are parser-like and **never versioned**. Status changes inside a parse are in the log with their cause (legal).
7. **Re-parse after apply:** unchanged from today. Text-changed and inserted rows are re-parsed (and stems with changed children); status-only changes are not.

**Not in step 2:**
- dependent re-parse through definitions (L4); the definition graph isn't served yet;
- the verdict split (R1/R4);
- unapplied effects (L10).

## L10. Unapplied effects in as_amended (PROPOSED RULE, 2026-10-01)

L8.4 gives `effects_unapplied` per law: effects legislation.gov.uk lists as "Not yet" applied to the text. In the hub there are 5,760 effects in 251 laws. Mapped exactly to a row: words substituted 1,352 (922 exact), words inserted 984 (733), inserted 892 (247), words omitted 412 (284), substituted 406 (214), omitted 365 (245), coming into force 114, others.

**The problem:** "not yet applied" mixes two different things:
- effects **in force** that legislation.gov.uk hasn't caught up with yet (the text is stale);
- effects **not yet in force** (prospective).

The effect items carry **no commencement date**, so flipping a provision to none on an unapplied omission could hide an obligation that still applies.

**Recommended rule:**
1. **Flag, don't flip.** A provision with an exactly-mapped unapplied effect keeps its as_amended classification. Legal shows "amendment pending (prospective / in force since …)" in its own UI from its own feed data, so fractalaw needn't send `unapplied_effects` back (legal).
2. **Word-level effects** (words substituted / inserted / omitted) never change the classification: there's no new text to classify, and the provision still exists.
3. **Whole-provision omission or repeal** (`omitted`, `repealed`, `revoked`, exactly mapped): decided by the effect's in-force data, which legislation.gov.uk's feed carries (`<ukm:InForceDates><ukm:InForce Date=… | Prospective="true">`):
   - **Prospective:** never flip. It isn't law yet, and the text isn't stale. Legal's sample: all 7 unapplied effects in anaw/2016/3 were prospective.
   - **A past `Date`:** in force, text stale → flip to none (status runs ahead of the text, L2).
   - **No in-force data:** flag only.
4. **Law-level** whole-law revocation (legal LiveStatus `revoked_unapplied`) feeds `current_verdict = revoked` when the verdict split is built (R1). The revocation is in force; only its application to the text lags.
5. **Unmapped effects** (null `section_id`): law-level flag only.
6. **The flag clears itself (Gemini):** `unapplied_effects` is recomputed from legal's manifest on every refresh, so once legislation.gov.uk applies the effect it drops out, and the new text arrives through the normal change log (L9).

**Reviews:** Gemini agrees with L9 (with the atomicity changes above) and with L10 as proposed (`data/code-review/drrp-temporal-L9-L10-gemini.md`). Legal agrees with both, with the changes folded in above: the entry-id key, status-hash-only changes never versioned, in-force data from the feed, and no `unapplied_effects` sent back.

**Legal's proposal (needs Jason's go-ahead):**
1. Parse `InForce` (date | prospective), `<ukm:Savings>` and the feed's **structured affected refs** (`<ukm:Section Ref=…>`) in ChangesFeed. The structured refs map to section_ids far better than the ~63% text-target parser.
2. Re-fetch the feeds for the ~357 laws with unapplied effects.
3. Add `in_force_date`, `prospective` and `saved` per item in `effects_unapplied`, with `section_id` taken from the structured refs.

### Decisions for Jason: D1–D4 APPROVED (Jason, 2026-10-01), as recommended
- **D1, prospective provisions:** classify and flag them ("coming into force" is visible, but excluded from current obligations), or `none` until commenced? Legal recommends: none until commenced; partly commenced → `in_force_partial`, classified and counted (flagged).
- **D2, savings:** `repealed_saved` status, or rely on the savings clause itself being classified? Legal recommends: `repealed_saved` only where a savings note exists (detectable from the notes); never inferred.
- **D3:** approve the sequencing, starting with legal's `status` column and our switch to it.
- **D4:** approve the change contract (L2), the cause handling with `unattributed` never versioned (L3), observation-order versions (L5) and the build order (L8).

## Original open questions (answered in the reviews above)

1. **How do we detect repeal?** Dots are a proxy. Does legal's CLML parse keep legislation.gov.uk's provision status (repealed, prospective, `Status` attributes, "Repealed" notes), so each LAT row can carry `status: in_force | repealed | prospective`? That's better than guessing from dots.
2. **Payload shape for `as_made`:** fields in the existing payloads, or separate keys? And provision-level `as_made` or law-level only (`making_as_made` + holder lists)? Law-level only is much cheaper and may be enough for the stated uses.
3. **A fully revoked law's current verdict:** `no_obligations`, or a distinct `revoked` value, so "no obligations now" can be told apart from "never had any"?
4. **Where are the made texts?** Should legal fetch `/made/` for every law, or only amended ones (known from the changes table)?
5. **Prospective provisions:** none until commenced? And provisions in force only in some extents ([E+W] vs [S])?
6. **Savings and transitional provisions:** a repealed provision can still apply to events before repeal. Is `as_made` plus the repeal date enough for audit questions ("what applied in 2015?"), or will point-in-time be needed?
7. **Cost:** with the reuse steps above, what fraction of provisions needs a second classification? This should be measured on a sample before committing.
