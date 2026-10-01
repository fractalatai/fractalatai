# DRRP over time: "as made" and "as amended" (PROPOSAL)

- **Status:** draft, revised after reviews by sertantai-legal and Gemini (2026-09-30). Section L (the LAT change contract) was added on 2026-10-01 for a second review. Awaiting Jason's decisions D1–D4.
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

## L. Change over a law's life: the LAT change contract (2026-10-01)

The views (R1–R8) say *what* we hold. This section says *how changes arrive*, which the earlier draft left loose. Today the LAT sync (#62) compares legal's previous and current LAT text and sorts each row into a bucket: unchanged, renamed, grown, text_changed, inserted, archived or held. It can't see:
- **why** a row changed: an amendment, legal improving its parser, or a change of scope (#166);
- **when** the change took legal effect, as opposed to when we noticed it;
- **whether our first copy was the law as made**, or already amended;
- **effects that legislation.gov.uk hasn't applied yet** (revoked_unapplied).

### L1. Worked lifecycle (Jason's scenario)

| # | Event in the law | What legal's LAT shows | What fractalaw does | as_amended | as_made |
|---|---|---|---|---|---|
| 0 | **Made** and first loaded. Current text = made text only if the law hasn't been amended yet | Rows, `status: in_force` (or `prospective`); manifest `amended: false` | Classify all rows; version 1 per provision | Live roll-up | = as_amended while `amended: false` |
| 0′ | **First loaded already amended** (the usual case for older laws) | Rows of the current text; manifest `amended: true` | Classify the current rows (version 1 = "first observed", not made). Queue a made-text job (R5) | Live roll-up | From the made text (R5), not from version 1 |
| 1 | **An amendment inserts provisions** (s.4A, reg.5ZA) | New section_ids, `status: in_force` or `prospective`; change `cause: legislative`, `effective_from`, `changed_by` | New versions; classify the new rows (plus any stem whose children changed) | Gains their DRRP | Unchanged (they didn't exist when made) |
| 2 | **DRRP-laden provisions are repealed** | Same ids; `status: repealed`; text dotted or "[Repealed]"; `cause: legislative` | New version with status repealed. **No re-parse; no model calls.** The previous version keeps its classification in the version history | Those provisions → none; verdict re-rolled (may move making → no_obligations) | Unchanged |
| 3 | **An existing provision is amended and an actor drops out** | Same id, new text; `cause: legislative`, `effective_from`, `changed_by` | New version; re-parse that provision only (+ dependents, L4). The previous version's actors stay in history | New classification; holder lists and correlatives re-rolled | Unchanged |
| 4 | **The whole law is repealed** | Every row `status: repealed` (or the law's status revoked); text often fully dotted | One version per provision (status only); no model calls | All none; `current_verdict = revoked` | Unchanged; `is_making` keeps its value |

The opposite order is just as possible (repeal first, amendment later). Each event is applied independently to the current versions.

### L2. What legal adds (the contract)

**Per LAT row:**
- `status: in_force | repealed | prospective` (R2). It is **authoritative over the text**: a row can say `repealed` while legislation.gov.uk still shows live text (an unapplied effect).
- `effective_from`: the date this row's current text or status took legal effect, where legal's amendment annotations know it. Otherwise null.
- `changed_by`: the instrument that made the last change (from the F-note commentary), or null.
- `extent_code` (exists already).

**Per law (manifest):**
- `amended: bool`: made ≠ current, from legal's amendment data.
- `as_of`: the date legislation.gov.uk's revised text is up to date to.
- `effects_unapplied`: legislation.gov.uk effects not yet in the text (e.g. revoked_unapplied), so status can run ahead of the text.

**Per change**, in the rename log, or a change log beside it: every changed, inserted or removed row carries a `cause`:
- **`legislative`**: a real amendment, repeal, insertion, substitution or commencement;
- **`parser`**: legal changed how it parses (ids, sort keys, text cleaning, structure). The law didn't change;
- **`scope`**: a scoped LAT widened or narrowed (#166);
- **`correction`**: legal fixed bad data.

### L3. What fractalaw does with each cause

| cause | Version history | Re-parse | Notes |
|---|---|---|---|
| `legislative`, text change | **New version** (old one kept with its actors and DRRP) | That provision + dependents | `effective_from` / `changed_by` stored on the version |
| `legislative`, status only (repealed, commenced) | New version | No (repealed → none). Prospective → in_force: classify only if never classified | |
| `parser` | **No new version**: the current version is overwritten | Only if the normalised text changed (`match_key`); otherwise carry tier data (today's behaviour) | Not a legal event, so no history |
| `scope` | Rows added or removed without versions | New rows only | Partial coverage is flagged (#66) |
| `correction` | Overwrite the current version | If the text changed | |
| unknown (legal didn't say) | Treated as `legislative` | As legislative | Safe default: keeps history rather than losing it |

### L4. Dependents: what else to re-parse when a provision changes

- **Stems and children.** If a stem changes, its list items are re-parsed (they're labelled with their stem). If a child is inserted, its stem's content list changed too.
- **Definitions.** If an interpretation provision's definition of a term changes, re-parse the provisions that use the term. The same rule is used for reuse in R6.
- **Referenced provisions.** If a provision that others cite as the holder's source changes ("regulations under subsection (2)"), re-parse the citing provisions.

### L5. Fractalaw storage

- **`provision_versions`** (new, append-only): `section_id, version, text_md5, status, extent_code, effective_from, observed_at, cause, changed_by, drrp (types), actors (jsonb snapshot), superseded_at`.
  - `legislation_text` + `provision_actors` remain the **current version**.
  - This replaces `lat_archive`'s ad-hoc role for legislative history. `lat_archive` stays for parser/scope churn and undo.
- **Law verdicts** are re-rolled after every applied change:
  - `current_verdict` (as_amended) changes over time;
  - `making_enrichment_verdict` (as_made) is computed once, from the made text or from version 1 when `amended: false`, and never re-rolled by later changes.
- **Ordering.** Versions are ordered by `effective_from`, falling back to `observed_at`. A version with an earlier `effective_from` than the current one (a late-applied effect) is inserted in order. The current view still follows the latest by `observed_at`, so it reflects legal's latest text and status.

### L6. Publishing

Only changed provisions and the law level are republished, after each applied change. A status-only repeal publishes the provision as classified none (R3) plus the re-rolled `current_verdict`.

### L7. What this deliberately doesn't do

- **Reconstruct intermediate history** (versions before we first observed a law). Only made (R5) and from-first-observation onward are kept. Point-in-time stays deferred (R8).
- **Classify repealed text.** A repealed provision's last classification survives in its version history, and nothing new is spent on it.

### Decisions for Jason
- **D1, prospective provisions:** classify and flag them ("coming into force" is visible, but excluded from current obligations), or `none` until commenced (legal's default; simpler)?
- **D2, savings:** add a `repealed_saved` status (it needs legal's commentary detection), or rely on the savings clause itself being classified?
- **D3:** approve the sequencing, starting with legal's `status` column and our switch to it.
- **D4:** approve the change contract (L2: legal's `cause`, `effective_from`, `changed_by`, `amended`, `as_of`, `effects_unapplied`) and fractalaw's version history (L5).

## Original open questions (answered in the reviews above)

1. **How do we detect repeal?** Dots are a proxy. Does legal's CLML parse keep legislation.gov.uk's provision status (repealed, prospective, `Status` attributes, "Repealed" notes), so each LAT row can carry `status: in_force | repealed | prospective`? That's better than guessing from dots.
2. **Payload shape for `as_made`:** fields in the existing payloads, or separate keys? And provision-level `as_made` or law-level only (`making_as_made` + holder lists)? Law-level only is much cheaper and may be enough for the stated uses.
3. **A fully revoked law's current verdict:** `no_obligations`, or a distinct `revoked` value, so "no obligations now" can be told apart from "never had any"?
4. **Where are the made texts?** Should legal fetch `/made/` for every law, or only amended ones (known from the changes table)?
5. **Prospective provisions:** none until commenced? And provisions in force only in some extents ([E+W] vs [S])?
6. **Savings and transitional provisions:** a repealed provision can still apply to events before repeal. Is `as_made` plus the repeal date enough for audit questions ("what applied in 2015?"), or will point-in-time be needed?
7. **Cost:** with the reuse steps above, what fraction of provisions needs a second classification? This should be measured on a sample before committing.
