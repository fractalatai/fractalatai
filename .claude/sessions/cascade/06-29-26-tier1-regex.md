# Session: Tier 1 — Regex Parse & Embed (PENDING)

## Problem

Regex parse and embedding are the foundation. Every downstream tier depends on them. Currently regex parses out-of-scope provisions (see Tier 0), and embeddings have gaps on in-scope provisions.

## Work

1. ⬜ Apply base case filter to regex parse (only parse in-scope provisions)
2. ⬜ Verify all in-scope provisions have embeddings
3. ⬜ Define descriptive stats for this tier:
   - In-scope provisions with regex_drrp / regex_position in provision_actors
   - In-scope provisions with embeddings
   - Actors created per law
4. ⬜ Run on QQ corpus — re-parse with base case filter
5. ⬜ Passing descriptive stats = session close signal
6. ⬜ Update corpus-stats skill with Tier 1 checks

## QA checks (close signal)

- count(in-scope provisions without embedding) = 0
- count(in-scope provisions without actors in provision_actors) = 0 (where DRRP detected)
- count(out-of-scope provisions WITH actors in provision_actors) = 0
- Every actor has regex_drrp and regex_position

## Depends on

- 06-29-26-tier0-base-case
