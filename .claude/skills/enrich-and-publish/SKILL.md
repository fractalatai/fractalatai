# Skill: Enrich & Publish

## When This Applies

When the user wants to run the DRRP parser on a set of laws and/or publish provision taxa to sertantai. This is a two-step workflow — always confirm scope before starting, and offer publish after enrichment completes.

## IMPORTANT: Understand What the User Wants

There are TWO distinct operations. The user may want one or both:

1. **Enrich** (`taxa enrich`) — runs the DRRP parser on provision text in LanceDB, populating taxa columns (drrp_types, actors, fitness, etc.). This is SLOW for large corpora (merge_insert write amplification). Only needed when:
   - Laws have never been enriched
   - The parser logic has changed (new actor patterns, new fitness terms)
   - The schema has changed (new struct fields)
   - `--force` is used to re-run on already-enriched laws

2. **Publish** (`sync publish --provisions`) — sends existing enrichment data from LanceDB to sertantai via zenoh. This is FAST (reads + network only). Use this when the data is already enriched and you just need to push it to sertantai.

**Always ask the user which they need.** If they say "run the parser" or "enrich", they want step 1. If they say "publish" or "send to sertantai", they want step 2. If they say "parse and publish", do both.

## Workflow

### Step 1: Confirm corpus scope

Ask the user: **"Which laws? Options:"**
- `--family "OH&S: Occupational / Personal Safety"` — all laws in a DuckDB family
- `--laws UK_ukpga_1974_37,UK_uksi_1999_3242` — specific law names
- `--laws $(cat data/qq-applicable-laws.csv)` — the customer's applicable laws
- No filter (all unenriched laws)

### Step 2: Enrich (if requested)

```bash
# Basic enrichment (unenriched laws only)
cargo run -p fractalaw-cli -- taxa enrich --gap-c [--family/--laws]

# Force re-enrichment (re-process even if already enriched)
cargo run -p fractalaw-cli -- taxa enrich --gap-c --force [--family/--laws]

# Force with skip-recent (avoid double-dipping within 24 hours)
cargo run -p fractalaw-cli -- taxa enrich --gap-c --force --skip-recent [--family/--laws]
```

**Requires GEMINI_API_KEY** for Tier 3 LLM classification:
```bash
source ~/.bashrc && GEMINI_API_KEY="$GEMINI_API_KEY" cargo run -p fractalaw-cli -- ...
```

**Disk warning:** Enrichment creates LanceDB fragment bloat via merge_insert. Monitor disk with `du -sh data/lancedb/` and `df -h /var/home`. If LanceDB exceeds 8 GB or free disk drops below 3.5 GB, kill the process, compact with `scripts/compact_lance.py`, and resume. See `bulk-enrichment` skill for full procedure.

### Step 3: Offer publish

After enrichment completes (or if user only wants publish), ask: **"Publish to sertantai now?"**

```bash
# Publish provisions via zenoh (requires sertantai running on port 7447)
cargo run -p fractalaw-cli -- sync publish --provisions [--family/--laws] --tenant dev --connect tcp/localhost:7447
```

**Note:** `--connect tcp/localhost:7447` is needed because sertantai already listens on that port. Without it, fractalaw tries to bind the same port and fails.

## Common scenarios

| User says | Do |
|---|---|
| "Parse OH&S" | Enrich with `--family`, then offer publish |
| "Publish QQ laws" | Publish only — no enrichment needed if already done |
| "Re-enrich and publish HSWA" | Enrich with `--force --laws UK_ukpga_1974_37`, then publish |
| "Run parser on customer corpus" | Enrich with `--laws $(cat data/qq-applicable-laws.csv)`, then offer publish |
| "Send the new data to sertantai" | Publish only |

## Flags reference

| Flag | Purpose |
|---|---|
| `--gap-c` | Enable Tier 1 parent inheritance + Tier 3 LLM classification |
| `--force` | Re-enrich even if taxa data already exists |
| `--skip-recent` | Skip laws enriched within the last 24 hours (avoids overlap) |
| `--family "..."` | Filter by DuckDB family name |
| `--laws "..."` | Comma-separated law names |
| `--tenant dev` | Zenoh tenant for publish (always `dev` for sertantai) |
| `--connect tcp/localhost:7447` | Connect to sertantai's zenoh instead of binding |
