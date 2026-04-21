# Skill: LAT (Legislative Text) Quality Assurance

## When This Applies

**BEFORE** running taxa DRRP gap analysis or fitness analysis on a family. This skill validates that the upstream data (provision text from sertantai, stored in LanceDB) is adequate for downstream parsing. There is no value running DRRP or fitness QA if the input data is incomplete or malformed.

**Trigger**: User asks to QA a family, run gap analysis, or enrich a family. Run this check first unless the family has already been validated.

## Why This Exists

Two classes of upstream issue silently degrade DRRP/fitness results:

1. **Enrichment truncation**: `enrich_single_law()` queries LanceDB with `limit=500`. Any law with >500 provisions is silently truncated — the enricher only processes the first 500 rows. As of 2026-04-21, **80 laws (52,846 provisions)** are affected corpus-wide. This is a fractalaw bug, not a sertantai issue.

2. **Part-level blob duplication**: Sertantai produces both fine-grained section-level provisions (s.1(2)(a)) AND large Part/Chapter/Schedule blobs containing the full concatenated text of that structural unit. The blobs duplicate text already present at section level. 538 blobs >5KB exist corpus-wide (10MB total). These inflate provision counts, confuse purpose classification (mixed content), and waste enrichment budget on duplicate text.

## Checks

### Check 1: Enrichment Truncation

Identify laws where LanceDB row count exceeds the enricher's 500-row limit.

```python
#!/usr/bin/env /usr/bin/python3
import lancedb
from collections import Counter

db = lancedb.connect("data/lancedb")
tbl = db.open_table("legislation_text")

# Option A: Check a specific family
# Get law names from DuckDB first:
#   cargo run -p fractalaw-cli -- query \
#     "SELECT name FROM legislation WHERE family = '<FAMILY>'" 2>/dev/null
laws = [...]  # fill from DuckDB query

# Option B: Check ALL laws
all_data = tbl.search().select(["law_name"]).limit(500000).to_arrow()
law_counts = Counter()
for i in range(all_data.num_rows):
    law_counts[all_data.column("law_name")[i].as_py() or ""] += 1

truncated = [(law, count) for law, count in law_counts.items() if count > 500]
truncated.sort(key=lambda x: -x[1])

print(f"Laws truncated by 500-row limit: {len(truncated)}")
for law, count in truncated:
    print(f"  {law}: {count:,} provisions ({count - 500} lost)")
```

**Severity**: BLOCKER for any truncated law. The enricher will silently skip provisions beyond row 500. DRRP coverage numbers for these laws are meaningless until the limit is raised or removed.

**Fix**: Increase or remove the limit in `enrich_single_law()` at `crates/fractalaw-cli/src/main.rs` line ~2859:
```rust
let batches = lance.query_legislation_text(&filter, 500, 0).await?;
//                                                  ^^^ this limit
```

### Check 2: Part-Level Blob Duplication

Identify provisions that are structural blobs (Part, Chapter, Schedule) containing text already present at section level.

```python
#!/usr/bin/env /usr/bin/python3
import lancedb, re

db = lancedb.connect("data/lancedb")
tbl = db.open_table("legislation_text")

for law in laws:
    data = tbl.search().where(f"law_name = '{law}'").select(
        ["section_id", "text"]
    ).limit(10000).to_arrow()

    parts = []
    sections = []
    for i in range(data.num_rows):
        sid = data.column("section_id")[i].as_py() or ""
        text = data.column("text")[i].as_py() or ""
        struct = sid.split(":", 1)[1] if ":" in sid else sid

        if re.match(r'^(pt\.|ch\.|sch\.)', struct):
            parts.append((sid, len(text)))
        elif re.match(r'^(s\.|reg\.|art\.)', struct):
            sections.append((sid, len(text)))

    large_parts = [(s, l) for s, l in parts if l > 5000]
    if large_parts:
        print(f"{law}: {len(large_parts)} large Part blobs "
              f"(max {max(l for _, l in large_parts):,} chars)")
```

**Severity**: WARNING. Part blobs don't block enrichment but they:
- Inflate provision counts (confusing QA metrics)
- Consume enrichment budget on duplicate text
- Produce misleading purpose classifications (mixed-content text)

**Mitigation**: The enricher could skip Part/Chapter/Schedule-level rows (section_type check) or fractalaw could filter them at query time. Alternatively, sertantai could stop emitting Part-level text blobs for laws that already have section-level provisions.

### Check 3: Provision Granularity

Compare provision structure against a known-good reference (HSWA 1974).

```python
#!/usr/bin/env /usr/bin/python3
import lancedb, re

db = lancedb.connect("data/lancedb")
tbl = db.open_table("legislation_text")

for law in laws:
    data = tbl.search().where(f"law_name = '{law}'").select(
        ["section_id", "text", "section_type"]
    ).limit(10000).to_arrow()

    types = {}
    lengths = []
    for i in range(data.num_rows):
        st = data.column("section_type")[i].as_py() or "(null)"
        text = data.column("text")[i].as_py() or ""
        types[st] = types.get(st, 0) + 1
        lengths.append(len(text))

    median = sorted(lengths)[len(lengths)//2] if lengths else 0
    max_len = max(lengths) if lengths else 0

    print(f"{law}: {len(lengths)} provisions, "
          f"median={median}, max={max_len:,}")
    print(f"  types: {types}")
```

**Reference values** (HSWA 1974 — well-parsed):
- 234 provisions, median 314 chars, max 3,793 chars
- Types: paragraph=128, sub_section=68, section=22, heading=7, part=2, sub_paragraph=7
- 0% empty purpose after enrichment

**Red flags**:
- Max provision >20,000 chars — likely a Part-level blob or concatenated text
- >50% of provisions have empty purpose after enrichment — enrichment truncation
- >2,000 provisions for a single law — check for structural duplication
- section_type distribution heavily skewed to one type — unusual parsing

### Check 4: Empty Text and Headings

Count provisions with empty or minimal text that will never produce useful DRRP.

```python
empty = sum(1 for l in lengths if l == 0)
tiny = sum(1 for l in lengths if 0 < l <= 20)
headings = types.get("heading", 0)

print(f"  Empty: {empty}, Tiny (<=20): {tiny}, Headings: {headings}")
```

These are expected in small quantities (headings, section numbers). If >20% of provisions are empty/tiny, the sertantai parsing may be producing too many structural fragments.

### Check 5: Section ID Consistency

Verify section IDs follow expected patterns and don't have gaps.

```python
import re

# Expected patterns
SECTION_RE = re.compile(r'^[A-Z]{2}_\w+:(s\.|reg\.|art\.|pt\.|ch\.|sch\.|h\.|title\.|table\.)')

malformed = []
for i in range(data.num_rows):
    sid = data.column("section_id")[i].as_py() or ""
    if sid and not SECTION_RE.match(sid):
        malformed.append(sid)

if malformed:
    print(f"  Malformed section IDs: {len(malformed)}")
    for sid in malformed[:5]:
        print(f"    {sid}")
```

## Workflow

Run checks in order. Stop and report if a BLOCKER is found.

```
1. Check 1 (Truncation)     — BLOCKER if any law exceeds 500 provisions
2. Check 2 (Part blobs)     — WARNING, note for metrics adjustment
3. Check 3 (Granularity)    — INFO, flag outliers
4. Check 4 (Empty/headings) — INFO
5. Check 5 (Section IDs)    — INFO
```

If Check 1 finds truncated laws:
- Report the truncation to the user
- Note that DRRP metrics for truncated laws are unreliable
- The enrichment limit fix is a prerequisite for meaningful gap analysis

If all checks pass, proceed to `taxa-gap-analysis` skill.

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-cli/src/main.rs:~2859` | `enrich_single_law()` — the 500-row limit |
| `crates/fractalaw-store/src/lance.rs` | `query_legislation_text()` — LanceDB query |
| `data/lancedb/legislation_text.lance/` | LanceDB table with provision text |
| `.claude/skills/taxa-gap-analysis/SKILL.md` | Downstream DRRP analysis (run after this QA passes) |
| `.claude/skills/lancedb-validation/SKILL.md` | LanceDB query patterns |
