---
session: "Actor dictionary fixes and Spc: Authorised Person reclass"
status: closed
opened: 2026-09-30
closed: 2026-09-30
outcome: success
related: [68, 74]
summary: >
  The gold v2 pilot referee found dictionary gaps. Scottish Ministers and Welsh Ministers
  got their own government labels; Health Body, Person in Control and Claimant were added;
  and the Spc: Appellant duplicate no longer matches new text. Spc: Authorised Person was
  reclassed as government, with a new match_group field keeping its regex order. Legal
  flipped in step, and the 52 affected laws were republished with no verdict change.
decisions:
  - what: "Spc: Authorised Person = government; no split label"
    why: "None of the 534 hub uses was licence-authorised: each exercises an enforcing authority's powers"
    result: "e5fac38; legal 902e193; 52 laws republished; legal remapped 63"
lessons:
  - title: "The regex bucket and the holder class are separate concerns"
    detail: "Moving a label to the government bucket let Ind: Person match 'person' first. match_group keeps the matching order while type decides the class."
    tag: architecture
bugs:
  - pattern: "Legal's dictionary subscriber handle was garbage-collected, so the subscription silently ended"
    category: integration
    module: "sertantai-legal ActorDictionary"
    affected: "Every dictionary put since the subscription existed"
    fix: "legal 2979c12; fractalaw also re-puts at the end of each publish (f76ac21)"
    status: fixed
---

# Session: Actor dictionary fixes and Authorised Person reclass (CLOSED)

## Todo

- ✅ Scottish/Welsh Ministers, Health Body, Person in Control and Claimant labels; Spc: Appellant trigger-only (`b52b62e`)
- ✅ Authorised Person → government with `match_group: governed` (`e5fac38`); dry run: no verdict changes
- ✅ 52 laws backfilled and republished; 5 stale law-level roll-ups re-rolled
- ✅ Dictionary delivery to legal verified ("Reloaded 130 actors")
