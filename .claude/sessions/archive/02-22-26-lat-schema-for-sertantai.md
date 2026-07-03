# LAT Schema for Sertantai PostgreSQL

This describes the `legislation_text` (LAT) table — one row per structural unit of legal text (title, part, chapter, heading, section, sub-section, paragraph, schedule entry, etc.). The scraper/parser should produce rows matching this schema.

Current dataset: **97,522 rows from 452 UK laws** (17 CSV source files covering ESH domains).

---

## Columns

### Identity & Position

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `law_name` | TEXT | NOT NULL | Parent law identifier. Format: `{JURISDICTION}_{type_code}_{year}_{number}`. Example: `UK_ukpga_1974_37` (Health and Safety at Work etc. Act 1974). FK to the legislation register (LRT). |
| `section_id` | TEXT | NOT NULL | **Primary key.** Structural citation — the canonical legal address. Format: `{law_name}:{citation}[{extent}]`. Stable across amendments — parliament assigns citations that never change. See examples and rules below. |
| `sort_key` | TEXT | NOT NULL | Machine-sortable string encoding parliamentary insertion ordering. `ORDER BY sort_key` recovers correct document order within a law. Derived from the citation. See encoding rules below. |
| `position` | INTEGER | NOT NULL | Snapshot document-order index (1-based) within the law at export time. Useful for range queries. Reassigned on re-export — not a stable identifier. |
| `section_type` | TEXT | NOT NULL | Structural type enum — see Section Types below. |
| `hierarchy_path` | TEXT | NULL | Slash-separated path in document structure. Example: `part.1/heading.2/provision.3/sub.1`. NULL for root-level rows (title). |
| `depth` | INTEGER | NOT NULL | Count of populated structural hierarchy levels. 0 = title/root, 1 = part, 2 = heading within part, etc. |

### Structural Hierarchy

Materialised path columns. Each is populated only when relevant to the row's position in the document tree. A section-level row has `part` and `chapter` filled (its ancestors) but `paragraph` is NULL. Repeated string values compress well in columnar stores via dictionary encoding.

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `part` | TEXT | NULL | Part number/letter |
| `chapter` | TEXT | NULL | Chapter number |
| `heading_group` | TEXT | NULL | Cross-heading group membership label. Value is the lead section/article number under the parent cross-heading (e.g., `18` means "under the cross-heading starting at section 18"). **Not a sequential counter** — values jump (1, 2, 18, 27...). NULL for rows outside any cross-heading group. Scoped to `(law_name, part/schedule)` — resets at schedule boundaries. The heading **text** is in the `text` column of `section_type = 'heading'` rows. |
| `provision` | TEXT | NULL | Section number (UK Acts) or article/regulation number (UK SIs). Merges the old `section` and `article` columns — `section_type` distinguishes which. |
| `paragraph` | TEXT | NULL | Paragraph number (sub-section level) |
| `sub_paragraph` | TEXT | NULL | Sub-paragraph number |
| `schedule` | TEXT | NULL | Schedule/annex number |

### Content

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `text` | TEXT | NOT NULL | The legal text content. Note: ~2.4% of rows have text starting with F-code markers (e.g., `F1 The amended text...`) — these are sections whose original text was entirely replaced by an amendment. |
| `language` | TEXT | NOT NULL | Language code: `en`, `de`, `fr`, `no`, `sv`, `fi`, `tr`, `ru` |
| `extent_code` | TEXT | NULL | Territorial extent at this provision level (e.g., `E+W`, `E+W+S+NI`, `S`). NULL when extent matches the parent law's default. |

### Amendment Annotation Counts

Per-section counts of annotation footnotes. Useful for identifying heavily amended provisions without joining to the annotations table.

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `amendment_count` | INTEGER | NULL | F-codes: textual amendments (words substituted/inserted/omitted) |
| `modification_count` | INTEGER | NULL | C-codes: modifications to how provisions apply |
| `commencement_count` | INTEGER | NULL | I-codes: commencement (bringing into force) |
| `extent_count` | INTEGER | NULL | E-codes: extent/territorial annotations |
| `editorial_count` | INTEGER | NULL | Editorial notes |

### Embeddings (populated later — schema only)

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `embedding` | FLOAT4[] | NULL | Semantic embedding vector (384 dimensions). NULL until AI pipeline runs. |
| `embedding_model` | TEXT | NULL | Model used: `all-MiniLM-L6-v2`, etc. |
| `embedded_at` | TIMESTAMPTZ | NULL | When embedding was generated |

### Pre-tokenized Text (populated later — schema only)

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `token_ids` | INTEGER[] | NULL | Pre-tokenized token IDs for the text column |
| `tokenizer_model` | TEXT | NULL | Tokenizer model used |

### Migration

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `legacy_id` | TEXT | NULL | Original Airtable positional encoding. Has 1.5% collision rate — not a key. Preserved for backward-compatible lookups during migration only. |

### Timestamps

| Column | PG Type | Nullable | Description |
|--------|---------|----------|-------------|
| `created_at` | TIMESTAMPTZ | NOT NULL | Record creation time |
| `updated_at` | TIMESTAMPTZ | NOT NULL | Last update time |

**Total: 30 columns** (28 core + 2 pre-tokenized text columns added for AI pipeline)

---

## Section Types

The `section_type` enum normalises structural terminology across jurisdictions. Each country's scraper maps its local terminology to this set.

| `section_type` | Description | UK Act example | UK SI example |
|----------------|-------------|----------------|---------------|
| `title` | Document title | title | title |
| `part` | Major division | Part I, Part II | Part 1, Part 2 |
| `chapter` | Chapter | Chapter I | Chapter 1 |
| `heading` | Cross-heading (groups sections) | "General duties" | "Interpretation" |
| `section` | Section (UK Acts) | s.2, s.25A | -- |
| `sub_section` | Sub-section | s.2(1), s.25A(3) | -- |
| `article` | Article / regulation (UK SIs) | -- | reg.2, art.16B |
| `sub_article` | Sub-article | -- | reg.2(1) |
| `paragraph` | Paragraph | para.1 | para.1 |
| `sub_paragraph` | Sub-paragraph | para.1(a) | para.1(a) |
| `schedule` | Schedule / annex | Schedule 1 | Schedule 1 |
| `commencement` | Commencement provision | -- | commencement |
| `table` | Table | table | table |
| `note` | Note / footnote / figure | note | note |
| `signed` | Signatory block | signed | signed |

---

## section_id — Structural Citation Format

The `section_id` is the primary key. It encodes parliament's canonical addressing scheme — "Section 41A of the Environment Act 1995" never changes, even when further amendments insert 41B, 41C, or 41ZA.

### Format

```
{law_name}:{citation}[{extent}]
```

- `{law_name}` — the parent law identifier (e.g., `UK_ukpga_1974_37`)
- `{citation}` — the structural citation using the `section_type` to determine prefix
- `[{extent}]` — optional extent qualifier, present only when a law has parallel territorial provisions for the same section number

### Citation Prefix by Section Type

| section_type | Prefix | Example citation |
|-------------|--------|-----------------|
| section | `s.` | `s.25A`, `s.25A(1)` |
| sub_section | `s.` | `s.25A(1)(a)` |
| article (regulation) | `reg.` | `reg.2A`, `reg.2A(1)(b)` |
| article (article) | `art.` | `art.16B` |
| schedule | `sch.` | `sch.2.para.3` |
| part | `part.` | `part.1` |
| chapter | `chapter.` | `chapter.1` |
| heading | `heading.` | `heading.18` |
| title | `title` | `title` |
| signed | `signed` | `signed#3` (position-qualified when multiples) |
| commencement | `commencement` | `commencement#2` |

### Examples

```
UK_ukpga_1974_37:s.25A          -- section 25A of HSWA 1974
UK_ukpga_1974_37:s.25A(1)       -- sub-section (1) of section 25A
UK_ukpga_1995_25:s.41A          -- inserted section 41A of Environment Act
UK_uksi_2002_2677:reg.2A(1)(b)  -- inserted regulation 2A(1)(b) of COSHH
UK_ukpga_1995_25:sch.2.para.3   -- schedule 2, paragraph 3
UK_ukpga_1974_37:s.23[E+W]      -- E+W territorial version of section 23
UK_ukpga_1974_37:s.23[NI]       -- NI territorial version
UK_ukpga_1974_37:s.23(4)[S]     -- Scotland version of sub-section (4)
```

### Parallel Territorial Provisions

29 UK laws (719 section-level rows) have parallel provisions where the same section number has different text for different territorial extents. Example: HSWA 1974 s.23(4) exists in three versions — E+W, NI, and S — each referencing different fire safety legislation.

The `[extent]` qualifier is added to `section_id` **only** when a law has parallel provisions. Sections with a single territorial version (the common case) have no qualifier.

### Why Citation-Based, Not Position-Based

Amendments insert new sections into existing laws. An integer position breaks on insertion:

```
Before:                After inserting s.41A:
  position 41 -> s.41    position 41 -> s.41
  position 42 -> s.42    position 42 -> s.41A  <-- inserted
                          position 43 -> s.42   <-- renumbered!
```

Citation-based IDs are stable. Position-based IDs require renumbering everything downstream on every insertion.

---

## sort_key — Normalised Sort Encoding

The `sort_key` encodes parliamentary insertion ordering into a lexicographically-sortable string. Naive string sort of citations is wrong (`s.10` sorts before `s.2`). The sort key fixes this.

### Encoding Rules

| Citation | sort_key | Notes |
|----------|----------|-------|
| `s.3` | `003.000.000~` | Numeric base zero-padded to 3 digits |
| `s.3ZA` | `003.001.000~` | Z-prefix: ZA=001, ZB=002... ZZ=026 (sorts before letter suffixes) |
| `s.3ZB` | `003.002.000~` | |
| `s.3A` | `003.010.000~` | Letter suffix: A=010, B=020, C=030... |
| `s.3AA` | `003.010.010~` | Double letter: nested insertion |
| `s.3AB` | `003.010.020~` | |
| `s.3B` | `003.020.000~` | |
| `s.4` | `004.000.000~` | |
| `s.23[E+W]` | `023.000.000~E+W` | Parallel provisions: extent as sort suffix |
| `s.23[NI]` | `023.000.000~NI` | Versions group together, sort alphabetically |
| `s.23[S]` | `023.000.000~S` | |

### Rules Summary

- **Numeric base**: zero-padded to 3 digits (handles up to section 999)
- **Z-prefix**: ZA=001 through ZZ=026 (Parliament uses these to insert *before* letter suffixes)
- **Letter suffix**: A=010, B=020, C=030... Z=260 (gaps for nested insertions)
- **Double letters**: AA=010+010, AB=010+020, etc.
- **Sub-levels**: additional `.NNN` segments for paragraph/sub-paragraph
- **Extent qualifier**: `~{extent}` suffix — tilde sorts after digits/letters, so all versions of a section group together

---

## Cross-Jurisdiction Generalisability

The citation-based design was validated against 8 jurisdictions. Every surveyed jurisdiction uses letter-suffix insertion for amendments — the pattern is universal.

| Jurisdiction | Symbol | Insertion example | section_id example |
|-------------|--------|-------------------|--------------------|
| UK | s./reg./art. | s.3A, s.3ZA | `UK_ukpga_1974_37:s.25A(1)` |
| Germany | section | section 5a | `DE_2020_ArbSchG:§5a.Abs.1` |
| Norway | section | section 16 a. | `NO_1973_03-09-14:§16a` |
| Turkey | Madde | Madde 27/A | `TUR_1983_2872:m.27/A` |
| Austria | section | section 4a | `AUT_2005_121:§4a` |
| Denmark | section | section 72 a | `DK_2020_1406:§72a.stk.2` |
| Finland | section | 13 h section | `FIN_1994_719:§13h` |
| Sweden | section | 3 a section | `SWE_2020_1:§3a` |

The sort key normalisation is jurisdiction-specific (different citation prefix mappings), but the three-column structure (`section_id`, `sort_key`, `position`) holds everywhere.

---

## Sample PostgreSQL DDL

```sql
CREATE TABLE legislation_text (
    -- Identity & Position
    law_name        TEXT        NOT NULL,
    section_id      TEXT        NOT NULL PRIMARY KEY,
    sort_key        TEXT        NOT NULL,
    position        INTEGER     NOT NULL,
    section_type    TEXT        NOT NULL,
    hierarchy_path  TEXT,
    depth           INTEGER     NOT NULL,

    -- Structural Hierarchy
    part            TEXT,
    chapter         TEXT,
    heading_group   TEXT,
    provision       TEXT,
    paragraph       TEXT,
    sub_paragraph   TEXT,
    schedule        TEXT,

    -- Content
    text            TEXT        NOT NULL,
    language        TEXT        NOT NULL DEFAULT 'en',
    extent_code     TEXT,

    -- Amendment Annotation Counts
    amendment_count     INTEGER,
    modification_count  INTEGER,
    commencement_count  INTEGER,
    extent_count        INTEGER,
    editorial_count     INTEGER,

    -- Embeddings (populated later)
    embedding           FLOAT4[],
    embedding_model     TEXT,
    embedded_at         TIMESTAMPTZ,

    -- Pre-tokenized (populated later)
    token_ids           INTEGER[],
    tokenizer_model     TEXT,

    -- Migration
    legacy_id       TEXT,

    -- Timestamps
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_lat_law_name ON legislation_text (law_name);
CREATE INDEX idx_lat_sort_order ON legislation_text (law_name, sort_key);
CREATE INDEX idx_lat_section_type ON legislation_text (section_type);
CREATE INDEX idx_lat_language ON legislation_text (language);
CREATE INDEX idx_lat_provision ON legislation_text (law_name, provision) WHERE provision IS NOT NULL;
```

---

## Amendment Annotations Table

One row per legislative change annotation. Links amendment footnotes to the LAT sections they affect.

```sql
CREATE TABLE amendment_annotations (
    -- Identity
    id              TEXT        NOT NULL PRIMARY KEY,
    law_name        TEXT        NOT NULL,
    code            TEXT        NOT NULL,
    code_type       TEXT        NOT NULL,
    source          TEXT        NOT NULL,

    -- Content
    text            TEXT        NOT NULL,
    affected_sections TEXT[],

    -- Timestamps
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ann_law_name ON amendment_annotations (law_name);
CREATE INDEX idx_ann_code_type ON amendment_annotations (code_type);
```

| Column | Description |
|--------|-------------|
| `id` | Synthetic key: `{law_name}:{code_type}:{seq}` — per-law, per-code_type counter. Example: `UK_ukpga_1974_37:amendment:1` |
| `law_name` | FK to legislation register |
| `code` | Annotation code from legislation.gov.uk: `F1`, `F123`, `C42`, `I7`, `E3`. Not unique per law. |
| `code_type` | `amendment` (F-codes), `modification` (C-codes), `commencement` (I-codes), `extent_editorial` (E-codes) |
| `source` | Data provenance: `lat_cie`, `lat_f`, `amd_f` |
| `text` | The annotation text describing the change |
| `affected_sections` | Array of `section_id` values from `legislation_text` that this annotation applies to |

---

## Key Design Decisions (Why We Did It This Way)

1. **Citation-based section_id over integer position** — Amendments insert new sections. Integer positions require renumbering downstream rows. Citations are parliament's canonical, permanent addresses. "Section 41A" never changes.

2. **Three-column identity (section_id + sort_key + position)** — Citations don't sort correctly as strings (`s.10` < `s.2`). The sort_key fixes this with normalised encoding. The position integer remains for convenience/range queries but is ephemeral.

3. **`provision` merges old `section`/`article`** — UK Acts use "sections", UK SIs use "articles/regulations". Same data, different label. The `section_type` column already distinguishes them, so one column suffices.

4. **`heading_group` (renamed from `heading`)** — Not a sequential counter. It's the lead section number of the parent cross-heading. Values jump: 1, 2, 18, 27, 33. The old name was misleading.

5. **Parallel territorial provisions** — 29 UK laws have the same section number with different text for different regions (e.g., s.23 has separate E+W, NI, and S versions). The `[extent]` qualifier on section_id handles this. Only added where needed — most sections have no qualifier.

6. **UK_uksi_2016_1091 excluded** — Parser bug produced 606 duplicate annotation IDs due to heavy post-Brexit territorial duplication. Excluded from all exports pending parser fix.
