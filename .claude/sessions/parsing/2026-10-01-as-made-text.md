---
session: "DRRP as made from the made text"
status: closed
opened: 2026-10-01
closed: 2026-10-01
outcome: abandoned
issue: 73
related: [72, "sertantai-legal#167"]
summary: >
  Not started. Jason dropped the bulk made-text fetch (R5) and the reuse rule (R6). The
  existing law-level fields are the whole-law view: everything the law imposed as far as
  our text shows, including later-repealed provisions. Suspect laws (live, amended, not
  making) go on a smell list instead, and made text is fetched per law only with his approval.
decisions:
  - what: "No bulk made text; whole-law view plus a per-law exception path"
    why: "is_making is the law's character, not its status: repeal never removes it and amendment rarely does. Re-classifying ~800 laws from made text isn't worth it when a smell list catches the exceptions"
    result: "Spec f4892ec; smell flag in taxa backfill 22db1f0 (18 laws on today's positions); legal's R5 session becomes the per-law exception path"
lessons: []
---

# Session: DRRP as made from the made text (CLOSED)

## Problem

`making_enrichment_verdict` and the as-made law fields feed legal's `is_making`. Today they're rolled up from the **current** text, which for an amended law isn't what the law imposed when it was made. The design (`docs/architecture/DRRP-TEMPORAL-PROPOSAL.md` R4–R6) computes as made from legislation.gov.uk's `/body/made/` text:
- only for amended laws (legal's manifest `amended`: 520 of 744 hub laws);
- reusing the current classification wherever a provision's text, and the definitions it depends on, are unchanged.

Priority 3 of the data-model work (Jason, 2026-10-01), after #72 correlatives.

## Todo

- ❌ **(legal)** Fetch `/body/made/data.xml` for a sample of amended laws into `legal_articles_made`, with a `lat-made` manifest key (R5)
- ❌ Cost sample: per law, provisions whose made text differs from current (normalised), including definition changes (R6). Estimate classification calls and cost
- ❌ **(Jason)** Decide: classify made text for all amended laws, or accept as made = first observed where the difference is small
- ❌ Build: made-text ingest; reuse rule; classify the differing provisions; as-made roll-up → the existing as-made fields (names unchanged, R1a)
- ❌ Dry run of as-made verdict changes → Jason review → publish (with the next combined publish)

## Dependencies

- ✅ R1a verdict split built (`33f307a`): the current view sits beside as made
- ✅ Legal manifest `amended` (L8.4)
- ⬜ Legal's made-text fetch (R5)

❌ items dropped (Jason, 2026-10-01, via legal): see the frontmatter decision. The smell list is tracked in `2026-09-30-issue-72.md`.
