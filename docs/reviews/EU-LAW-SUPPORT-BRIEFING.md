# Briefing: EU Retained Law Support in Fractalaw

**Date**: 2026-06-05
**From**: sertantai-legal (data provider)
**To**: fractalaw (DRRP/Fitness enrichment service)

## Summary

sertantai-legal now parses LAT for EU retained laws (Regulations, Directives, Decisions). The Zenoh data pipeline delivers EU law LAT rows in the same Arrow IPC format as UK domestic laws, but with structural differences fractalaw needs to handle.

## What Changed in sertantai-legal

1. **LAT parser extended** — handles `<EURetained><EUBody>` XML structure from legislation.gov.uk
2. **EU Title → Part**, **EU Chapter → Chapter** in hierarchy
3. **EU laws use `art.` citation prefix** (not `reg.`)
4. **Country derived from law_name prefix** for partitioned storage

No changes to the Zenoh wire format, key expressions, or Arrow schema.

## Data Differences Fractalaw Will See

### 1. section_id prefix: `art.` vs `reg.`

UK SIs use `reg.` (they contain "Regulations"):
```
UK_uksi_2002_2677:reg.2(1)(a)
UK_uksi_2005_1541:reg.7(1)
```

EU laws use `art.` (they contain "Articles"):
```
UK_eur_2006_1907:art.Article 1(1)(a)
UK_eudr_2010_75:art.Article 3
```

**Impact**: Any code that pattern-matches or splits on `reg.` in section_ids needs to also handle `art.`.

### 2. provision field contains "Article N" not just "N"

UK SIs:
```
provision: "2"        → section_id: reg.2
provision: "7"        → section_id: reg.7(1)
```

EU laws:
```
provision: "Article 1"  → section_id: art.Article 1
provision: "Article 3"  → section_id: art.Article 3(1)(a)
```

**Impact**: If fractalaw extracts provision numbers for display or matching, it needs to handle the "Article " prefix. The number itself follows the same pattern (numeric, with possible letter suffixes like 3A).

### 3. hierarchy_path includes "Article" in provision component

UK SI:
```
hierarchy_path: "provision.2/sub.1/para.a"
```

EU law:
```
hierarchy_path: "part.I/chapter.1/provision.Article 1/sub.1/para.a"
```

**Impact**: Hierarchy path parsing that splits on `/` and extracts provision numbers.

### 4. EU type_code values

| type_code | Meaning | Citation prefix |
|-----------|---------|----------------|
| `eur` | EU Regulation (retained) | `art.` |
| `eudr` | EU Directive (retained) | `art.` |
| `eudn` | EU Decision (retained) | `art.` |
| `uksi` | UK Statutory Instrument | `reg.` |
| `ukpga` | UK Public General Act | `s.` |

The `type_code` is available in the LRT data served over Zenoh. Fractalaw can use it to branch logic if needed.

### 5. EU structural hierarchy

EU laws have an additional hierarchy level that UK SIs don't:

```
EU Regulation:          UK SI:
  EUTitle (→ part)       Part
    EUChapter (→ chapter)  Chapter (rare in SIs)
      Article (→ article)    Regulation (→ article)
        Para (→ sub_article)   Para (→ sub_article)
```

The section_type values are the same (`part`, `chapter`, `article`, `sub_article`, `paragraph`). EU laws just tend to have more structural depth.

### 6. Content/language differences

EU law text uses slightly different regulatory language:
- "Member States shall ensure..." (EU) vs "Every employer shall..." (UK)
- "The Agency shall..." (EU body referenced) vs "The Authority shall..." (UK regulator)
- Definitions often use numbered lists: "(1) 'substance' means..." 
- Cross-references use EU citation format: "Directive 2008/98/EC" not "S.I. 2011/988"

**Impact on DRRP**: The duty/right/responsibility/power extraction patterns may need tuning for EU phrasing. "Member States shall ensure" creates a Responsibility (government actor), not a Duty (governed actor). EU Regulations that create direct obligations on businesses (like REACH) do contain Duties.

**Impact on Fitness**: EU laws often specify fitness at a broader level — "installations listed in Annex I" rather than UK-style "every factory, mine, quarry". The fitness_place, fitness_sector, fitness_person dimensions may need EU-specific extraction patterns.

## Volume

| type_code | Laws in QQ corpus | Estimated LAT rows |
|-----------|-------------------|-------------------|
| eur | 418 | ~50,000-100,000 (REACH alone is 173 articles) |
| eudr | 152 | ~10,000-20,000 |
| eudn | 135 | ~2,000-5,000 (decisions are typically short) |
| **Total** | **705** | **~60,000-125,000** |

For comparison, current UK domestic LAT is ~184,000 rows from ~2,500 laws.

## Making Classification

EU type codes have Tier 0 making classification in sertantai-legal:
- `eur` → making (0.95 confidence) — Regulations create direct obligations
- `eudr` → not_making (0.9) — Directives bind Member States, not businesses directly
- `eudn` → not_making (0.5) — Decisions vary

Fractalaw's enrichment may confirm or override this. EU Directives that have been transposed into UK law (as UK SIs) are already in the corpus as domestic laws — the Directive itself is usually not_making.

## Test Law Suggestions

Good laws to validate EU enrichment against:

1. **UK_eur_2006_1907** (REACH) — large, complex, many duties on manufacturers/importers. 173 articles. The reference EU Regulation for testing.
2. **UK_eudr_2010_75** (Industrial Emissions Directive) — 84 articles, permits and controls. Tests directive structure (EUChapter, no EUTitle).
3. **UK_eur_2008_1272** (CLP Regulation) — classification and labelling of chemicals. Direct duties on suppliers.
4. **UK_eudr_2012_18** (Seveso III Directive) — major accident hazards. Tests complex article structure with Annexes.

## No Changes Needed in sertantai-legal

The Zenoh DataServer, Arrow serialization, and key expressions work for EU laws without modification. The data flows correctly. All changes are on the fractalaw side for DRRP/Fitness extraction.
