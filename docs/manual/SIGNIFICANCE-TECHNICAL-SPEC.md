# Significance Data: Technical Specification for Sertantai Integration

**Version**: 1.0
**Date**: 2026-07-02

## Overview

Fractalaw publishes per-provision and per-law significance data via Zenoh, enabling sertantai to display a prioritised compliance register. This document specifies the data contract, storage requirements, and integration patterns.

## Data Channels

Significance data arrives on the existing Zenoh publish channels. No new channels are needed.

| Channel | Key Expression | Encoding | Significance Fields |
|---------|---------------|----------|-------------------|
| Enrichment | `fractalaw/@{tenant}/taxa/enrichment/{law_name}` | Arrow IPC | 7 law-level fields |
| Provisions | `fractalaw/@{tenant}/taxa/provisions/{law_name}` | Arrow IPC | 7 provision-level fields |

## Provision-Level Fields

Published on the provisions channel. Present on Obligation provisions only; `null` for non-Obligation provisions and provisions not yet rated.

| Field | Type | Values | Description |
|-------|------|--------|-------------|
| `significance_scope_duty_bearer` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | Breadth of who bears the duty |
| `significance_scope_protected_class` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | Breadth of who is protected |
| `significance_gravity` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | What is at stake (health vs admin) |
| `significance_strength` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | Obligation strength (absolute vs procedural) |
| `significance_hierarchy` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | Structural position in the law |
| `significance_confidence` | `Float32` | 0.0 - 1.0 | SLM confidence (logprobs average) |
| `significance_overall` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | Weighted aggregate of all 5 dimensions |

### Overall Formula (Approach B)

```
score = 0.35 * gravity + 0.20 * scope_duty_bearer + 0.20 * scope_protected_class
      + 0.15 * strength + 0.10 * hierarchy

Where: HIGH=3, MEDIUM=2, LOW=1
Thresholds: score >= 2.5 -> HIGH, >= 1.75 -> MEDIUM, else LOW
```

## Law-Level Fields

Published on the enrichment channel. Present for laws with rated Obligation provisions; `null` otherwise.

| Field | Type | Values | Description |
|-------|------|--------|-------------|
| `significance_rating` | `Utf8` | `HIGH` / `MEDIUM` / `LOW` | Overall law significance |
| `significance_score` | `Float32` | 0.0 - 25.0 | Raw score for custom sorting |
| `significance_high_count` | `Int32` | 0+ | Count of HIGH-significance provisions |
| `significance_medium_count` | `Int32` | 0+ | Count of MEDIUM-significance provisions |
| `significance_low_count` | `Int32` | 0+ | Count of LOW-significance provisions |
| `significance_total_obligations` | `Int32` | 0+ | Total rated Obligation provisions |
| `significance_parts` | `Utf8` (JSON) | JSON array or `null` | Part-level breakdown (large Acts only) |

### Rating Formula (Approach L)

```
score = avg_provision_significance * log2(total_obligations + 1)
avg   = (3 * high_count + 2 * medium_count + 1 * low_count) / total_obligations

Rating: percentile-based across all laws
  Top 20%    -> HIGH
  Middle 47% -> MEDIUM
  Bottom 33% -> LOW
```

### Part Breakdown Schema

`significance_parts` is a JSON array, present only for Acts with >= 50 rated Obligation provisions and Part structural rows. Example:

```json
[
  {"part": "pt.I", "high": 31, "medium": 35, "low": 69, "total": 135},
  {"part": "pt.III", "high": 0, "medium": 3, "low": 10, "total": 13},
  {"part": "pt.IV", "high": 0, "medium": 10, "low": 14, "total": 24}
]
```

## Sertantai Storage

### ElectricSQL Schema Additions

**`uk_lrt` table** (law-level):

```sql
ALTER TABLE uk_lrt ADD COLUMN significance_rating TEXT;
ALTER TABLE uk_lrt ADD COLUMN significance_score REAL;
ALTER TABLE uk_lrt ADD COLUMN significance_high_count INTEGER;
ALTER TABLE uk_lrt ADD COLUMN significance_medium_count INTEGER;
ALTER TABLE uk_lrt ADD COLUMN significance_low_count INTEGER;
ALTER TABLE uk_lrt ADD COLUMN significance_total_obligations INTEGER;
ALTER TABLE uk_lrt ADD COLUMN significance_parts TEXT; -- JSON blob
```

**`lat` table** (provision-level):

```sql
ALTER TABLE lat ADD COLUMN significance_scope_duty_bearer TEXT;
ALTER TABLE lat ADD COLUMN significance_scope_protected_class TEXT;
ALTER TABLE lat ADD COLUMN significance_gravity TEXT;
ALTER TABLE lat ADD COLUMN significance_strength TEXT;
ALTER TABLE lat ADD COLUMN significance_hierarchy TEXT;
ALTER TABLE lat ADD COLUMN significance_confidence REAL;
ALTER TABLE lat ADD COLUMN significance_overall TEXT;
```

## Integration Patterns

### 1. Compliance Register — Law Ranking

Sort customer's applicable laws by `significance_score` descending. Display `significance_rating` as a badge (HIGH/MEDIUM/LOW). Show the distribution profile (`high_count`/`medium_count`/`low_count`) as a stacked bar or summary.

```sql
SELECT name, title_en, significance_rating, significance_score,
       significance_high_count, significance_medium_count, significance_low_count
FROM uk_lrt
WHERE name = ANY($customer_laws)
  AND significance_rating IS NOT NULL
ORDER BY significance_score DESC;
```

### 2. Compliance Register — Provision Filtering

Within a law, allow filtering by `significance_overall`. Compliance officers focus on HIGH provisions first.

```sql
SELECT section_id, text, significance_overall,
       significance_gravity, significance_scope_duty_bearer
FROM lat
WHERE law_name = $law_name
  AND significance_overall IS NOT NULL
ORDER BY
  CASE significance_overall
    WHEN 'HIGH' THEN 1 WHEN 'MEDIUM' THEN 2 ELSE 3
  END,
  sort_key;
```

### 3. Dashboard — Significance Summary

Aggregate across customer's register for a dashboard view:

```sql
SELECT
  significance_rating,
  count(*) as law_count,
  sum(significance_high_count) as total_high_provisions
FROM uk_lrt
WHERE name = ANY($customer_laws)
  AND significance_rating IS NOT NULL
GROUP BY significance_rating;
```

### 4. Large Act Drill-Down

For Acts with `significance_parts IS NOT NULL`, parse the JSON and display per-Part significance:

```typescript
const parts = JSON.parse(law.significance_parts);
// Render as table or stacked bars per Part
// e.g., "Part I: 31 HIGH, 35 MEDIUM, 69 LOW"
```

### 5. Dimension Detail View

When a user clicks on a provision's significance badge, show the 5 individual dimensions:

| Dimension | Rating | Description |
|-----------|--------|-------------|
| Gravity | HIGH | Health & safety at stake |
| Scope (duty bearer) | HIGH | All employers |
| Scope (protected class) | MEDIUM | Employees in workplace |
| Strength | MEDIUM | SFARP-qualified |
| Hierarchy | HIGH | General duty (Part I) |
| **Overall** | **HIGH** | |
| Confidence | 0.94 | |

## Data Volumes

| Metric | Count |
|--------|-------|
| Rated Obligation provisions | 40,468 |
| Laws with significance | 553 |
| Laws rated HIGH | 109 (20%) |
| Laws rated MEDIUM | 246 (47%) |
| Laws rated LOW | 166 (33%) |
| Provisions rated HIGH | 5,359 (13%) |
| Provisions rated MEDIUM | 10,023 (25%) |
| Provisions rated LOW | 25,086 (62%) |

## Notes

- Significance fields are `null` until a law has been through the significance pipeline (SLM rating + hierarchy derivation + backfill)
- The `significance_confidence` field reflects the SLM's certainty. Values below 0.9 indicate the SLM was uncertain and the provision may have been elevated to LLM review.
- `significance_parts` is only populated for large Acts with Part structural hierarchy. Most SIs and Regulations do not have Parts.
- Arrow IPC encoding means the new columns arrive automatically in the existing RecordBatch — sertantai's Arrow decoder picks up new columns without code changes, but the ElectricSQL schema must be updated to store them.
