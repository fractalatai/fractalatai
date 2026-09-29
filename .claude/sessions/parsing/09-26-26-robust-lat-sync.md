---
session: Robust LAT Sync
status: closed
opened: 2026-09-26
closed: 2026-09-29
outcome: success

summary: >
  Built fractalatai #62: manifest-verified LAT sync between sertantai-legal and the Postgres hub. Per-law lat_hash and struct_hash
  are agreed with legal and cross-checked 980/980. Diff-apply applies legal's rename log first, then exact/marker/grown text
  matching; held rows are never guessed; changed or removed rows are archived, never deleted; each law applies in one
  transaction with a tier-data carry-over gate. Deletes go through deterministic verdicts, then review, then Jason. First real
  run: 711 laws synced with 0 gate failures. QQ Tier 0 (122) and the live-fix 8 were re-enriched and published with
  provenance, and verified by legal.

decisions:
  - what: Hash contract lat_hash + struct_hash, computed identically in Rust, SQL and legal
    why: Fire-once events drop; a manifest makes drift detectable and self-healing
    result: Contract with shared vectors; legal stores the hashes via triggers; 980/980 cross-check for both
  - what: Carry tier data only when it is provably still valid
    why: Parse never removes actor rows it doesn't re-find, so stale actors would stick
    result: Carry on the same text, marker-only differences, or new ⊇ old; otherwise archive and clear. Legal aligned its merge rule
  - what: Never delete; archive with a reason, always restorable
    why: Revocation data is unreliable on both sides (fractalaw #64; legal blank-target and territorial bugs)
    result: Reasons not_in_legal, revoked_unapplied, not_making, held_duplicate; 3 PPC orders restored after legal's tier 1 fix
  - what: Deletion needs evidence, not absence
    why: A law missing from the manifest may just be a legal gap
    result: Deterministic verdicts, review against the legislation.gov.uk changes feed (not data.xml), Jason approves
  - what: Zero actors and no duty text = evidenced no_obligations
    why: 50 Tier 0 laws had no duty text at all; holding them published nothing auditable
    result: Guard refined; provenance model fractalaw-law-drrp:no_duty_text
  - what: No revoked-law guard
    why: Jason — enriching revoked law is harmless (it was Making while in force; legal has no status-to-DRRP link)
    result: Dropped; #64 low priority

metrics:
  sync_first_run: { laws_synced: 711, gate_failures: 0, benchmark_held: 20, archived_removed_rows: 38161, text_changed_snapshots: 15296 }
  tier0: { laws: 122, making: 23, no_obligations: 89, empowering: 9, held: 1, provisions_published: 32274, provenance_events: 363 }
  livefix: { laws: 8, provisions_published: 5902, provenance_events: 24 }
  archives: { not_making: 96, not_in_legal: 58, revoked_unapplied: 2, regnal_duplicates: 5, held_duplicate_rows: 343, restored: 3 }
  llm_tier: { gemini_actors: 252, penalty_clause_false_obligations_fixed: 53, adjudicated_laws: 6 }

lessons:
  - title: Rebuild the binary before running it for real
    detail: The Tier 0 law-level publish went out without provenance because target/debug/fractalaw-sync predated the change and only cargo test had run. It was republished; saved to memory.
    tag: tooling
  - title: legislation.gov.uk lags in applying revocations
    detail: Whole-instrument revocations sit as unapplied effects for years, and data.xml is incomplete. Use the changes feed and count unapplied, savings and territorial-extent cases.
    tag: data
  - title: The LLM tier needs the same exclusions as the parser
    detail: 53 of 67 Gemini Obligations in Tier 0 were penalty clauses; the prompt lacked the offences/penalties rule. Check a test batch for systematic errors before promoting.
    tag: methodology
  - title: pgrep -f in a wait loop can match itself
    detail: until ! pgrep -f "<pattern>" loops forever when the loop's own command line contains the pattern. Wait on a PID instead.
    tag: tooling
  - title: Benchmarks were silver, not gold
    detail: The benchmark set was Gemini-labelled on older-generation LAT, and 5 laws were never labelled. Raised as #65.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/lat_sync.rs
  - crates/fractalaw-store/src/pg_lat_sync.rs
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-sync-cli/src/lat_sync.rs
  - .claude/skills/lat-sync/SKILL.md
  - .claude/skills/llm-agent-review/SKILL.md
  - data/qq-readiness/tier0/TIER0-publish-log.md
  - data/qq-readiness/livefix/LIVEFIX-publish-log.md
  - data/lat-sync/review22_lgu.csv

depends_on:
  - 09-26-26-actor-model-gaps.md

enables:
  - parsing/09-29-26-reenrichment-backlog.md
  - ml/09-28-26-benchmark-refresh.md
  - 'next sync: legal 42-parent-Act extent re-parse (1,462 renames) + first live sync watch'
---

# Session: Robust LAT Sync (CLOSED)

## Problem

LAT sync from legal to the hub is one-way and lossy. `upsert_lat` never deletes superseded rows, and events carry no hash or version, so a missed event is never recovered. The only delete path (`lat_deleted`) cascades away tier data. As a result, 345/802 hub laws have drifted from legal, and 128 are missing duty paragraphs (1,545 paragraphs). fractalatai #62.

## Todo

- ✅ Agree the manifest contract with legal (see Contract; #62 comment)
- ✅ Hash functions in `fractalaw-core/src/lat_sync.rs`: `lat_hash` + `struct_hash`, pinned to legal's vectors; 980/980 cross-checked for both
- ✅ Per-law sync state in the hub (`lat_sync_state`: lat_hash, struct_hash, row_count, renames_through, held ids, reparse_needed). DuckDB isn't needed.
- ✅ Consume legal's rename log (`lat-renames`): applied first, chains resolved; text match is the fallback; ambiguous/colliding → held
- ✅ Diff-apply (`fractalaw-store/src/pg_lat_sync.rs`): archive, not delete; text changed → snapshot + clear tier data + re-parse; sort_key-only → in place; renames carry tier data; one transaction per law
- ✅ `pull-lat --stale` (dry run by default, `--apply`, `--limit`); `sync watch` diff-applies events, never deletes on `lat_deleted`, re-compares the manifest at startup + every `--manifest-interval-mins` (default 60)
- ✅ Protect benchmarks: report-only unless `--allow-benchmark`; `--archive-laws` refuses them
- ✅ Gate: tier data on carried rows (actors, fitness, drrp_types, embedding, scope) must be identical, or roll back and stop the run. The report has `actors_carried`. Backup/pilot/batches are in the `lat-sync` skill runbook.
- ✅ Delete verification: deterministic verdicts (`verified_revoked` / `needs_review` / `unknown_to_legal` / `legal_gap_in_force`) → agent review (skill) → Jason approves → `--archive-laws`; `--restore-laws` undoes it
- ✅ Watch full-compare interval: 60 min default
- ✅ struct_hash (legal live; vectors pinned; 980/980)
- ✅ Tests: 11 core (vectors, normalise, planner incl. Wester Ross, renames, chains, duplicates), 4 store (scratch DB `fractalaw_lat_test`: carry-over, gate rollback, archive/restore, real-law no-op), sync decode, delete verdicts; the Wester Ross reg.4(2) → Obligation taxa test
- ✅ End-to-end on the scratch DB: 3 real laws applied from legal @dev (Wester Ross 20 → 28 rows incl. reg.4(2)); rerun → in_sync
- ✅ Data processing: backup → dry run → pilot → 25 batches (711 laws in sync, 0 gate failures) → QQ Tier 0 enriched, published, verified by legal
- ✅ Clean-up: drive capacity. Moved `~/fractalaw/models` (15.9 GB) and `~/.ollama/models` (16.1 GB) to `/mnt/ssd/offload/` (verified, symlinked; the SSD is in fstab); cleared npm/brew/pip caches. `/var/home` 96% → ~67%. Client data (data/, Postgres volume) stays on the LUKS-encrypted drive; the SSD is unencrypted.
- ✅ Clean-up: pushed `9cd2c65` (provenance) and `e7bc2ab` (Tier 0 fixes)
- ✅ Clean-up: archived the 96 laws legal discarded as not_making (`pull-lat --archive-laws … --archive-reason not_making`; 3,162 rows + 287 actor rows to `lat_archive`, restorable). This matches legal's 3,162 rows. DuckDB verdicts and provenance are kept.
- ✅ Clean-up: delete candidates. Reviewed 22 against legislation.gov.uk effects (`data/lat-sync/review22_lgu.csv`): 17 confirmed revoked (11 by unapplied whole-instrument effects). Archived 61 (44 auto-verified + 17), 4,828 rows + 3,196 actor rows, reason not_in_legal. 5 kept (Coal Industry Act 1994 likely wrongly marked revoked by legal; CoP(A)A 1989; GPSR 1994; Special Waste 1996; CRC 2010). Feedback sent to legal: its status parse must count unapplied whole revocations and keep `live`/`live_description` consistent.
- ✅ Clean-up: 5 regnal-year duplicates archived on the hub (1,544 rows, reason not_in_legal); the modern-named laws are in legal. DuckDB law rows remain (see note).
- ✅ Clean-up: 9 revoked #58 flips. All whole-revoked per the legislation.gov.uk changes feed (e.g. PPC 2000 by SI 2007/3538). Jason: enriching revoked law is fine (it was Making while in force; legal has no status→DRRP link). DuckDB and legal match on significance/application/parts, so no republish (legal agreed).
- ❌ Clean-up: revoked guard for enrichment/publish. Not needed (Jason 2026-09-28): enriching revoked law does no harm to legal's data, just costs a little pipeline time; legal's improved revocation detection means fewer will flow to us.
- ⏸️ Clean-up: DuckDB status inference (#64). Low priority now: nothing gates on it (the guard was dropped). Delete verification should prefer legal's `live_evidence` when present.
- ⏸️ Clean-up: 20 benchmark laws. Moved to the Benchmark Refresh session (#65, `ml/09-28-26-benchmark-refresh.md`): 5 are unlabelled (drop the flag, sync); 15 are Gemini silver on stale LAT (re-label, compare the SLM, redesign with smaller laws). Commented on legal #166 (Tier 0 big Acts need re-enrichment once scoped).
- ✅ Clean-up: held rows. 343 remaining across 23 laws (the dry-run total of 383 included benchmark plans), archived as `held_duplicate` with 251 actor rows (`pull-lat --archive-held`; restorable); held lists cleared.
- ⏸️ Clean-up: re-enrichment backlog. Moved to the pending session `parsing/09-29-26-reenrichment-backlog.md`
- ⏸️ Clean-up: 6 Tier 0 pending_slm actors. In the re-enrichment backlog session (next pod)
- ⏸️ Clean-up: audit the earlier corpus LLM labels for the penalty error. In the re-enrichment backlog session
- ⏸️ Clean-up: SLM false positives. Covered by #65 (Benchmark Refresh: SLM retraining decision)
- ✅ Clean-up: `.cargo/config.toml` `LIBRARY_PATH` → `/home/linuxbrew/.linuxbrew/opt/gcc/lib/gcc/current` (version-stable; survives brew gcc upgrades); rebuilt
- ⏸️ Clean-up: `sync watch` live run. With the next sync session (legal's 42-parent-Act rename batch)
- ✅ Clean-up: scratch DB `fractalaw_lat_test` and the scratch venv removed; recreate steps are in the `lat-sync` skill (Tests)
- ✅ Clean-up: session docs committed; #62, #63 and stale-LAT sessions closed

## Decisions (Jason, 2026-09-26)

- sort_key stays in `lat_hash` (confirmed in this session).
- Add delete verification (deterministic → agent → Jason).
- Add a structural hash covering the other LAT columns fractalaw consumes.
- Build fractalaw's side now, in this session.
- **Data processing is parked** until the code has landed and been tested. This covers re-pulls, deletions, the regnal-year duplicates, and re-sending the 9 revoked #58 verdicts. When unparked:
  - re-send the 9 revoked flips with no verdict and add a revoked guard;
  - delete the 5 regnal-year duplicates.

## Dependencies

- ✅ Legal: LAT manifest queryable, stored lat_hash (triggers), hash on events after commit; cross-checked 980/980
- ✅ Legal: updated test vectors, cross-checked by an independent Python implementation (synthetic, empty, 3 live laws all match)
- ✅ Legal: classified the 76 hub-only laws (66 revoked, 5 regnal-year duplicates, 5 in force awaiting legal LAT)
- ✅ Measurement (`parsing/09-26-26-stale-lat-repull.md`, `data/qq-readiness/lat/lat_compare.csv`)

## Contract (agreed with legal 2026-09-26)

- **Manifest:** `fractalaw/@{tenant}/data/legislation/lat-manifest/{law_name}` and `/*`, returning `{law_name, row_count, lat_hash, updated_at}` (JSON; Arrow for `*`).
- **Hash rows:** the full row set the LAT queryable serves, including empty-text structural rows (NULL → ""), ordered by section_id in byte order.
  - Each row is `section_id \t sort_key \t normalise(text) \n`: sort_key as-is, NULL → "", position excluded. The hash is the lowercase hex SHA-256.
  - The sort_key change was proposed by both sides after legal's sort_key bug. Legal relayed Jason's approval; **to be confirmed by Jason in the fractalaw session.**
  - `normalise`: NFC → collapse runs of the explicit Unicode White_Space set to a space → trim. Zero-width characters are kept.
- **Legal:** computes the hash on demand (~127 ms for the corpus); events carry the hash and row_count; `lat_deleted` = (0, sha256("")); the manifest is the source of truth.
- **section_id isn't stable** (legal #120 rewrote ids in place without events). Diff-apply carries tier data on a unique exact normalised-text match.
- **Test vectors:**
  - **Pin:** synthetic `TEST:reg.1`, sort_key `00001~`, text `"  A person  must   not\tdeploy.\u200b "` → `79bc96eafd545bbab104423e759bb2787d4b8aae4c34a2545eb41249a67cf07b` (with sort_key);
  - **Pin:** empty law → sha256("") `e3b0c442…`;
  - legal's checked-in fixture law, to follow.
  - Live values with sort_key, not to pin: UK_ssi_2016_88 (28 rows, f75e2c32…), UK_ukpga_1974_37 (835, 979269b6…), UK_uksi_2015_1947 (1,323, be6964f8…).
  - All were verified 2026-09-26 by an independent Python implementation against `legal_articles`.

## 76 hub-only laws (legal's classification, 2026-09-26)

- **66 revoked:** legal holds no LAT, and DuckDB has `status = revoked` for 50 of them. Under the manifest (row_count 0), diff-apply would delete their hub rows. The first run will list them for Jason's approval rather than applying silently.
  - **9 of the 11 #58 false→true flips are revoked laws:** UK_nisr_2008_55, UK_uksi_2000_1973, UK_uksi_2000_3184, UK_uksi_2001_1091, UK_uksi_2004_107, UK_uksi_2004_3212, UK_uksi_2009_785, UK_uksi_2010_105, UK_uksi_2014_255.
  - #58's verdict comparison didn't filter on `status`. **Open for Jason:** re-send or leave those verdicts, and add a revoked guard to enrichment/publish.
- **5 regnal-year duplicates**, which legal holds under modern names (e.g. `UK_ukpga_1875_Vict/38-39/17` → `UK_ukpga_1875_17`). These are the 3 #57 publish skips. **Drop or rename for Jason's approval.**
- **5 in force with no legal LAT** (UK_ssi_2005_157, UK_uksi_1998_892, UK_uksi_2015_10, UK_wsi_2014_3303, UK_ukpga_1994_27): legal's gap, queued for LAT parse.

## Legal updates (2026-09-26, later)

- **The 5 in-force hub-only laws are now LAT-parsed in legal:**

  | Law | Rows |
  |---|---|
  | UK_ssi_2005_157 | 258 |
  | UK_uksi_2015_10 | 141 |
  | UK_wsi_2014_3303 | 133 |
  | UK_ukpga_1994_27 | 20 |
  | UK_uksi_1998_892 | 9 |

  - `lat` events were emitted.
  - The hub holds stale copies. A plain `pull-lat` would leave the old-generation rows next to the new ones, so these wait for diff-apply, or for a one-off clean re-pull if Jason approves.
  - The last 4 are marked enriched in legal from stale hub LAT and need re-enrichment on fresh LAT.
- **Legal bug: `sort_key` ordering** (legal fix session pending).
  - Lettered items (c) and (d) are encoded as Roman numerals, so they sort after (g). Also, every `signed` row has an all-zero sort_key and sorts first.
  - This doesn't affect `lat_hash`, which orders by section_id.
  - **Fractalaw's exposure:**
    - `fitness.rs:540` concatenates child text in sort_key order, so stem + child text can arrive scrambled;
    - `pg.rs:544-553` assigns each part by the preceding sort_key. The signed row sorts first, but paragraph misorder stays within a section, so this is probably harmless;
    - `pg.rs:61/68` loads provisions in sort_key order;
    - `scripts/compliance/generate_controls.py:131`.
  - Nothing to change until legal re-parses. Section ids and text are unaffected.

## Legal side live + cross-check (2026-09-26)

- **Queryables:** `fractalaw/@dev/data/legislation/lat-manifest/{law}` and `/*`. Arrow IPC by default, `?format=json` for JSON.
  - `*` lists only laws with LAT (980).
  - A single law with no LAT returns row_count 0 and sha256("").
- **Hash storage:** legal now stores `legal_register.lat_hash`, maintained by triggers on section_id, sort_key and text. Computing on demand turned out to take 8.4 s for the corpus, not 127 ms.
- **Events:** `lat` persist and `lat_deleted` events carry row_count + lat_hash and are sent after commit. `lat.fix_section_ids` now emits events too.
- **Fixture vectors:** `sertantai-legal/backend/test/fixtures/lat_hash/vectors.json` (synthetic, empty, fixture_law `UK_uksi_2099_1` with 9 rows, `aa3c16e4…`). Copy them into fractalaw-core tests when building.
- **Cross-check** (scratch Python client, read-only):
  - all 3 vectors match;
  - 980/980 laws: the hash recomputed from `lat/{law}` rows matches the manifest on hash and row_count.
- **Client note:** `lat/{law}` for a law with no LAT replies with a zero-length payload. Treat that as 0 rows.

## Legal sort_key fix (2026-09-26, later)

- **Fixed:** paragraph segments are letters-only, so (c) and (d) no longer sort as Roman numerals, and `signed` rows sort after the body. The stored rows were rewritten in place: 20,848 rows in **579 laws**, sort_key only. **These 579 have a new lat_hash**, so the manifest will show them as stale.
  - Diff-apply must treat a sort_key-only change as an in-place update: no re-parse, tier data kept.
- **Still open on legal's side, held until fractalaw's diff-apply exists:**
  - **386 laws** carry sort_keys from older parser generations and need a legal re-parse. That can shift section_ids, and 101 of them are enriched.
  - **Parent-drop bug:** after a nested sub-paragraph the parser can drop the parent paragraph, e.g. UK_wsi_2025_1321 `reg.39(e)`, which should be `reg.39(2)(e)`. The fix changes section_ids.
  - Both rely on the text-match carry-over to preserve tier data.

## Legal handoff: ready to build (2026-09-26)

Legal's side is complete on @dev. Events arrive on `fractalaw/@dev/events/sync` (table `lat`), sent after commit, each with law_name + row_count + lat_hash:
- `persist`;
- `lat_deleted` (0 rows, empty hash);
- `persist` with `reason: section_ids_fixed`.

The `lat/{law}` queryable returns the full row set ordered by sort_key.

**What the first diff-apply run will see:**
- 579 laws with sort_key-only changes;
- 66 revoked hub-only laws, as delete candidates for Jason's approval;
- 5 regnal-year duplicates, pending Jason;
- 5 freshly parsed in-force laws, 4 needing re-enrichment;
- the 345 drifted laws.

**Legal will schedule after diff-apply lands:** the 386-law older-generation re-parse and the parent-drop fix.

**Also ready in legal, pending Jason's launch:** 2 PDF-only laws, UK_uksi_1979_791 (58 rows) and UK_uksi_1947_805 (3 rows, an amending order).

## Gemini review of the re-parse/sync plan (via legal, Jason agreed, 2026-09-26)

Files: `sertantai-legal/backend/data/code-reviews/2026-09-26-lat-sync-enrichment-{brief,review}.md`

- **Legal will preserve its own enrichment.** LatPersister will merge on re-parse instead of DELETE+INSERT: unchanged rows keep their data, text-matched renames carry it over, only changed text is blanked, ambiguous rows are flagged. Today any legal re-parse wipes its own enrichment (209K rows, 637 laws).
- **Legal records an explicit rename map** (old→new section_id, with ambiguous cases flagged). Fractalaw applies it first and uses its own text match only as a fallback.
- **For fractalaw:**
  - archive rather than delete;
  - per-law carry-over gate at 100% on unchanged-text rows;
  - DuckDB/hub backup before the first run;
  - define the watch full-compare interval.
  
  All added to the Todo.
- **Hash blind spot (review point):** fixes to position or other structural columns don't change `lat_hash`.
- **Timing:** the hub stays frozen (no re-pulls). Legal can go ahead with the parent-drop fix and the 386-law re-parse now; fractalaw's diff-apply will later see the hash changes plus the rename map.

## Build notes (2026-09-26)

- **Toolchain:** brew upgraded gcc 15 → 16, which left `/home/linuxbrew/.linuxbrew/bin/{gcc,g++}` pointing at missing `-15` binaries. They were repointed to `gcc-16`/`g++-16`. The CC string is unchanged, so the cached DuckDB build is still valid.
  - `.cargo/config.toml`'s `LIBRARY_PATH` still names `Cellar/gcc/15.2.0_1`, which no longer exists. Linking still works for now. **Update it when a DuckDB rebuild is next acceptable** (changing it triggers a rebuild).
- **Scratch DB:** `fractalaw_lat_test` on :5433 holds the hub schema + 3 laws. Tests use it and never touch `fractalaw`.
- **Production hub:** no `lat_sync_state`/`lat_archive` tables yet. They're created on first `pull-lat` with `--pg`.

## Matching across parser generations (Jason, 2026-09-26)

- **Rules 1–2 (`bc184e4`):** `match_key` mirrors legal's `LatMerge.match_key`: normalise, no space before punctuation, strip leading `[F…` / `(n)` / bare provision numbers. It adds the hub's own article prefixes, `4.—` and `3. `.
  - Same-id rows that differ only by these markers keep their tier data.
  - Hub-only rows get a second unique-match pass (`MarkerMatch`).
  - Scratch DB, 3 worst-case laws: actor rows carried went 0 → 5 of 34; the rest were archived.
- **Rule 3, revised (`0f32bdb`):** keep tier data (and re-parse) only when the new text **contains** the old text.
  - When the old text contains the new one (a parent that held its children's text), or the text is otherwise changed or empty, the snapshot is archived and the data cleared.
  - **Why:** `taxa parse` upserts actor rows and never removes ones it doesn't find again, so re-parse can't clean up carried stale actors. The first recommendation to Jason was wrong on this point, and he re-approved the revised rule.
  - On the 3 sample laws, all 16 containment rows were old ⊇ new, with 0 actors.
  - This is stricter than legal's rule 4, which also carries old ⊇ new and empty.
- **Legal aligned with the stricter rule 3** (2026-09-26). LatMerge now carries a row only when the keys are equal or the new text contains the old.
  - An extent_tag pair whose new text is empty is now logged as `dropped`; fractalaw's planner archives those.
  - Legal's enriched re-parse had already run 15 laws under the old rule; legal cleared enrichment on 237 rows in 11 laws.
  - **Backlog:** legal will send the list of laws and rows that need re-enrichment (Jason: fractalaw's backlog).
- **Legal's re-parse is complete** (2026-09-26): all 386 older-format laws; 10 enriched ones forced (Jason accepted the loss); 0 errors. Corpus sort breaks are down from ~8,000 to 668, and hashes are consistent. Legal is idle.
  - **Re-enrichment backlog:** `sertantai-legal/backend/data/reports/lat-reparse/reenrichment-list.csv`, 101 laws. Legal-side enriched rows went 34,550 → 25,414; 33,933 rows need enrichment, many of them new, because the finer split turned UK_ukpga_1991_56 from 809 into 8,288 rows.
  - All of these will show as changed in fractalaw's first `--stale` pass.
  - 2 control mappings are orphaned (EPA 1990 s.40(4), s.74(3), now repealed), pending Jason on legal's side.
- **Legal's sort breaks: 668 → 0** (2026-09-27, Jason's request). The breakdown is in `sertantai-legal/backend/data/reports/lat-reparse/sort-breaks-2026-09-27.csv`.
  - Causes: insert-order conflicts 208, title-valued Parts 133, `#n` ids 96, letter-then-digit 70, source markup 63, inexpressible numbering 58, tables 20, parallel-extent 20.
  - Fixes: encoder fixes (6-digit position pad, digit segments after a suffix letter), plus a deterministic document-order repair that re-keyed 3,230 rows.
  - **All 980 laws have a new lat_hash** (sort_key only). For fractalaw's first sync this is a metadata-only update.
  - **Caveat:** repaired keys borrow their predecessor's structural segments, so never decode part/provision from sort_key. Checked: fractalaw only orders and compares by sort_key (part assignment in `pg.rs`, loaders, fitness concat); `core/sort_key.rs` encodes and never decodes. Safe.

## Hub sync: processing (Jason unparked 2026-09-27)

- **Pushed:** `bc184e4`, `0f32bdb`.
- **Backup before the first run** (quick mode, verified):
  - SSD stage: `/mnt/ssd/fractalaw-backups/nas-stage-20260927/`;
  - NAS: `fractalaw-backups/20260927/`: pgdump 540M (8 tables, 288,089 legislation_text rows) + DuckDB 443M (19,481 laws); sha256 OK on the NAS copy.
- Stopped two stale background waiter loops left over from the #55/#57 sessions; each matched its own command line in `pgrep -f`.
- **Paused before the dry run (Jason, 2026-09-27):** legal is building a LAT tracker to produce the outstanding parse list, then parsing the gap laws. Jason will have legal message fractalaw when that's done.
  - Then: `pull-lat --stale` dry run → pilot → batches.
  - Newly parsed laws the hub doesn't hold go in via `--laws`, per the handoff decision.
- **Unparked for QQ Tier 0 (Jason, 2026-09-27; legal relayed the request, Jason confirmed here with "yes, let's run").**
  - **Dry run:** 731 laws planned, 71 delete candidates (44 verified_revoked, 22 needs_review, 5 unknown_to_legal), 383 rows held.
  - **Pilot** (UK_uksi_2021_1315, UK_ukpga_1996_37): all 200 actor rows accounted for (18 carried + 182 archived); both then in_sync.
  - **Batches of 30** (25 batches): 711 laws in sync, 0 gate failures. The 20 benchmark laws were held report-only.
    - 422 laws are flagged reparse_needed.
    - Archived: 38,161 removed rows (22,610 actor rows, 29,902 fitness mentions) and 15,296 text_changed snapshots (15,854 actors, 7,032 fitness).
  - **Delete candidates were not archived** (awaiting Jason).
  - **Tier 0 (122 laws):** pulled LRT for 12 laws missing from DuckDB and LAT for 71 laws new to the hub. DuckDB snapshot: `/mnt/ssd/fractalaw-backups/fractalaw_pre_tier0_20260927.duckdb`. Local pipeline running (parse → dep → embed → classify → infer → reconcile); a pod is needed next for SLM.
- **Tier 0 pod + LLM (2026-09-28):**
  - Pod: position SLM 17,217 actors (41 min, 0 errors); significance 4,780 provisions (15 min, 0 errors); fitness 1,361/1,362 mentions (1 JSON error). Pod stopped.
  - LLM tier via Gemini (`llm-batch`): 203 pending_llm actors. The script's `--law-file` now scopes default mode too, and accepts one law per line.
  - **53 of Gemini's 67 Obligations were penalty clauses** ("guilty of an offence… liable…"). The prompt gained an offence/penalty and "references elsewhere" rule; those 53 were cleared (before values in `tier0/llm_review/penalty53_before.csv`) and re-run, and all came back none/mentioned.
  - All 203 were promoted to `llm`. 14 Obligation labels remain; a few are debatable (void / Crown binding / a fee / an officer's offence).
  - `llm-agent-review` skill written: Claude-agent alternative, same contract. The agent wasn't spawned (tool-safety check outage; Jason asked to stop).
- **Tier 0 published and verified by legal (2026-09-28):**
  - is_making 3,600 → 3,623 (23 f→t, 0 downgrades); provisions 32,274 across 121 laws.
  - Provenance republished after the stale-binary miss; legal holds 363 per-family events, with hashes matching.
  - Legal's blank-event cleanup and the not-Making LAT discard await Jason.
  - Log: `data/qq-readiness/tier0/TIER0-publish-log.md`.
- **Legal's status fix, batch 0 (2026-09-28):** 9 QQ-register laws are no longer Revoked (blank-target rows, territorial revocations). None were archived on the hub.
  - Legal will flag any of our 61 archives whose `live` changes as later tiers land; restore if so.
  - GPSR 1994 and CRC 2010 are confirmed revoked by legal's changes table, and held pending Jason.
  - `lat-sync` skill: verification now uses the legislation.gov.uk changes table, not data.xml, and handles territorial and commencement cases.
- **Archived GPSR 1994 (48 rows) and CRC Order 2010 (585 rows)** as `revoked_unapplied` (new reason, commit 5be6f09; Jason 2026-09-28), on legal's changes-feed evidence: SI 2005/1803; SI 2013/1119 with savings; both unapplied.
- **Live-fix 8 (2026-09-28, Jason):** REACH, 561/2006, 98/24/EC, Confined Spaces (re-run for provenance); FEPA 1985, Forestry 1967, Special Waste 1996, DPA 2018 (full enrichment).
  - Full pipeline incl. pod and Gemini (49; 0 penalty Obligations); all 8 making.
  - Published 5,902 provisions + 8 law-level payloads with provenance.
  - **Verified by legal:** is_making unchanged (3,623); the source is now enrichment for all 8; 24 per-family events on the current lat_hash.
  - Log: `data/qq-readiness/livefix/LIVEFIX-publish-log.md`.
- **Legal status fix, tier 1 (2026-09-28):** restored UK_uksi_2015_1352, 2016_150 and 2016_398 on the hub (7 rows each, no actors). They're PPC Designation Orders revoked in England only by SI 2019/458, and still in force in Wales.
  - Pending Jason's rule: UK_uksi_1994_1057, 1996_3001, 1997_2560 (Surface Waters Classification; legal's "in force in S" rests on the WRA 1991 GB extent).
  - The other 55 archived laws remain revoked.
  - Legal won't parse LAT for the 3 restored PPC orders: they're not Making (designation orders; lean LAT). They stay on the hub and show as `legal_gap_in_force` delete candidates, which never auto-archive. An evidenced verdict, if wanted, goes through legal's tier process.
  - Surface Waters (UK_uksi_1994_1057, 1996_3001, 1997_2560) **stay archived**. Jason's rule: a parent Act's extent only narrows a law's regions and never determines them alone, so legal now has all three as Revoked. Legal will say if enabling-section extents later put any in force.
- **Legal re-parsed 42 parent Acts to fix provision extents (2026-09-29).**
  - The next `--stale` will see struct_hash changes on all 42, plus 1,462 section_id renames on `lat-renames` across 16 Acts (Environment Act 1995: 396; EPA 1990: 272; …). Enrichment was carried on legal's side.
  - **Re-enrichment backlog +4:** UK_ukpga_1990_16 (17 rows), UK_ukpga_1988_52 (4), UK_ukpga_1993_11 (3), UK_anaw_2017_2 (1).
  - Legal report: `backend/data/reports/lat-reparse/enabling_parents_b0.csv`; snapshot `lat_reparse_enabling_parents_b0`.
  - UK_uksi_1999_1892 T&CP (Trees) Regs is now "Revoked in E; in force in W" (not archived here).
