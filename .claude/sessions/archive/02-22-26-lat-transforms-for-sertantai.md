# LAT Data Transforms Reference

How raw scraped data gets transformed into the `legislation_text` and `amendment_annotations` tables. This documents every transform applied during our CSV→Parquet pipeline so Sertantai can replicate the same logic when building rows for PostgreSQL.

See also: `LAT-SCHEMA-FOR-SERTANTAI.md` (column definitions, types, DDL).

Source: `data/export_lat.sql` (DuckDB), `crates/fractalaw-core/src/sort_key.rs` (Rust).

---

## Table of Contents

1. [ID Normalisation (Acronym Stripping)](#1-id-normalisation-acronym-stripping)
2. [Record Type → section_type Mapping](#2-record-type--section_type-mapping)
3. [Content Row Detection](#3-content-row-detection)
4. [Provision Merging (section + article → provision)](#4-provision-merging)
5. [Heading → heading_group Rename](#5-heading--heading_group-rename)
6. [Region → extent_code Mapping](#6-region--extent_code-mapping)
7. [Building section_id (Structural Citation)](#7-building-section_id-structural-citation)
8. [Building sort_key (Normalised Sort Encoding)](#8-building-sort_key-normalised-sort-encoding)
9. [Parallel Territorial Provisions](#9-parallel-territorial-provisions)
10. [Disambiguation (Position Suffix)](#10-disambiguation-position-suffix)
11. [Hierarchy Path Construction](#11-hierarchy-path-construction)
12. [Depth Calculation](#12-depth-calculation)
13. [Amendment Annotation Counts](#13-amendment-annotation-counts)
14. [Annotation ID Synthesis](#14-annotation-id-synthesis)
15. [Annotation affected_sections Linkage](#15-annotation-affected_sections-linkage)
16. [Exclusions and Filters](#16-exclusions-and-filters)

---

## 1. ID Normalisation (Acronym Stripping)

Legacy Airtable IDs carry acronym suffixes/prefixes. All IDs must be stripped to the canonical form `{JURISDICTION}_{type_code}_{year}_{number}`.

### Three patterns

| Pattern | Example | Stripped |
|---------|---------|---------|
| `UK_ACRO_type_year_num` | `UK_CMCHA_ukpga_2007_19` | `UK_ukpga_2007_19` |
| `UK_type_year_num_ACRO` | `UK_ukpga_1974_37_HSWA` | `UK_ukpga_1974_37` |
| `UK_year_num_ACRO` | `UK_2007_19_CMCHA` | `UK_2007_19` |

### Detection logic

```
if ID matches ^UK_[A-Z]+_[a-z]+_      → strip the ACRO_ after UK_
if ID matches ^UK_[a-z]+_\d+_[\w]+_[A-Z] → strip trailing _ACRO
if ID matches ^UK_\d+_\d+_[A-Z]       → strip trailing _ACRO
else → no change
```

The key distinction: acronyms are UPPERCASE, type_codes are lowercase. `UK_HSWA_ukpga_1974_37` has `HSWA` (uppercase) as the acronym before `ukpga` (lowercase type_code).

### Where it applies

- `law_name` column on all tables
- `section_id` (contains law_name as prefix)
- `annotation.id` (contains law_name as prefix)
- `affected_sections` array values
- Any cross-references between tables

**For new scrapers**: if you're generating IDs from scratch (not migrating Airtable data), just use the canonical format directly and skip this transform entirely.

---

## 2. Record Type → section_type Mapping

Source CSV files use a `Record_Type` column with values like `section`, `sub-section`, `article,sub-article`, etc. These map to the normalised `section_type` enum:

| Record_Type (CSV) | section_type (output) |
|--------------------|-----------------------|
| `title` | `title` |
| `part` | `part` |
| `chapter` | `chapter` |
| `heading` | `heading` |
| `section` | `section` |
| `sub-section` | `sub_section` |
| `article` | `article` |
| `article,heading` | `heading` |
| `article,sub-article` | `sub_article` |
| `sub-article` | `sub_article` |
| `paragraph` | `paragraph` |
| `sub-paragraph` | `sub_paragraph` |
| `schedule` | `schedule` |
| `annex` | `schedule` |
| `table` | `table` |
| `sub-table` | `table` |
| `figure` | `note` |
| `signed` | `signed` |
| `commencement` | `commencement` |
| `table,heading` | `heading` |

Key points:
- Hyphens become underscores: `sub-section` → `sub_section`
- `annex` maps to `schedule` (same structural role)
- `article,heading` and `table,heading` both map to `heading`
- `figure` maps to `note`

**For new scrapers**: map your jurisdiction's terminology to this enum. See the section_type table in the schema doc.

---

## 3. Content Row Detection

Not all rows in the source are content. Annotation rows (amendments, modifications, commencements, etc.) are interleaved in the CSV. Content rows are identified by exclusion:

**A row is a content row if Record_Type:**
- Is not NULL or empty
- Does NOT end with `,content` (annotation content rows like `modification,content`)
- Does NOT start with `amendment,` (amendment annotations like `amendment,textual`)
- Does NOT start with `subordinate,` or `editorial,`
- Is NOT one of: `commencement,heading`, `modification,heading`, `extent,heading`, `editorial,heading`, `subordinate,heading` (annotation group headings — carry no data)

**For new scrapers**: if you're scraping legislation.gov.uk directly, you control what becomes a content row vs an annotation row. The key principle is: one row per structural unit of the document (title, parts, chapters, headings, sections, sub-sections, paragraphs, schedules, etc.).

---

## 4. Provision Merging

The old schema had separate `Section` and `Article` columns. These are merged into a single `provision` column because they're the same concept — the primary provision number — with `section_type` distinguishing what kind it is.

| section_type | Source column | provision value |
|-------------|--------------|----------------|
| `section`, `sub_section` | `Section` | `25A`, `3`, `41` |
| `article`, `sub_article` | `Regulation` (or `Article`) | `2A`, `16B` |
| `paragraph`, `sub_paragraph` | Sometimes `Paragraph`, sometimes `Section` | varies |

In our CSVs the column was `Section||Regulation` (a combined column). If your scraper has separate section/article fields, just pick the non-null one.

---

## 5. Heading → heading_group Rename

The column formerly called `heading` is renamed to `heading_group`. No value transformation — just a name change.

**What it actually contains**: the first section/article number under the parent cross-heading. NOT a sequential counter. For HSWA 1974 Part I:

| heading_group | Cross-heading text | Sections covered |
|---------------|-------------------|-----------------|
| `1` | "Preliminary" | s.1 only |
| `2` | "General duties" | s.2–s.9 |
| `18` | "Enforcement" | s.18–s.26 |
| `33` | "Provisions as to offences" | s.33–s.42 |

Values include: plain numbers, alpha-suffixed (`10A`, `25A`), single letters (`A`, `D`), dotted decimals (`1.1`, `2.1`). NULL for rows outside any cross-heading group.

---

## 6. Region → extent_code Mapping

Source CSV has a `Region` column with human-readable territorial extent. This maps to a compact code:

| Region value | extent_code |
|-------------|-------------|
| Contains all four (England, Wales, Scotland, NI) | `E+W+S+NI` |
| England + Wales + Scotland | `E+W+S` |
| England + Wales + Northern Ireland | `E+W+NI` |
| England + Scotland | `E+S` |
| England + Wales | `E+W` |
| England + Northern Ireland | `E+NI` |
| `England` | `E` |
| `Wales` | `W` |
| `Scotland` | `S` |
| `Northern Ireland` | `NI` |
| Starts with `GB` | `E+W+S` |
| Starts with `UK` | `E+W+S+NI` |
| NULL or empty | NULL |
| Anything else | Pass through as-is |

**For new scrapers**: if scraping legislation.gov.uk XML, the extent is available in the `Extent` attribute on each provision element. Map it to the same codes.

---

## 7. Building section_id (Structural Citation)

The `section_id` is the primary key. Format: `{law_name}:{citation}[{extent}]`

### Citation construction rules

The citation prefix depends on `section_type` and (for articles) the instrument class:

| section_type | Class | Prefix | Example |
|-------------|-------|--------|---------|
| `section`, `sub_section` | any | `s.` | `s.25A`, `s.25A(1)` |
| `article`, `sub_article` | `Regulation` | `reg.` | `reg.2`, `reg.2(1)(b)` |
| `article`, `sub_article` | other | `art.` | `art.16B` |
| `schedule` | any | `sch.` | `sch.2` |
| `part` | any | `pt.` | `pt.1` |
| `chapter` | any | `ch.` | `ch.1` |
| `heading` | any | `h.` | `h.18` |
| `title` | any | `title.` | `title.1` |
| `signed` | any | `signed.` | `signed.1` |
| `commencement` | any | `commencement.` | `commencement.1` |
| `paragraph` (in schedule) | any | `sch.{N}.para.` | `sch.2.para.3` |
| `table`, `note`, etc. | any | `{type}.` | `table.50` |

### Sub-section and paragraph suffixes

Append parenthesised sub-section and paragraph values when present:

```
provision=25A, sub=1        → s.25A(1)
provision=25A, sub=1, para=a → s.25A(1)(a)
provision=2, sub=1, para=b  → reg.2(1)(b)
```

### Schedule-scoped rows

Rows inside a schedule (headings, parts, chapters, paragraphs) get prefixed with `sch.{N}.`:

```
heading inside schedule 2     → sch.2.h.5
part inside schedule 2        → sch.2.pt.1
paragraph inside schedule 2   → sch.2.para.3
```

The schedule container row itself gets `sch.{N}` with no extra prefix.

### Position fallbacks

When the structural value is missing (e.g., a section row with no provision number), the `position` integer is used as fallback. For singleton types like `title`, `signed`, `commencement`, position is always appended to handle multiples.

### Full examples

```
UK_ukpga_1974_37:s.25A          — section 25A of HSWA 1974
UK_ukpga_1974_37:s.25A(1)       — sub-section (1) of section 25A
UK_ukpga_1974_37:s.23[E+W]      — E+W territorial version (see §9)
UK_ukpga_1995_25:s.41A          — inserted section 41A
UK_uksi_2002_2677:reg.2A(1)(b)  — COSHH regulation 2A(1)(b)
UK_ukpga_1974_37:sch.2.para.3   — schedule 2, paragraph 3
UK_ukpga_1974_37:title.1        — title row
UK_ukpga_1974_37:pt.1           — Part 1
UK_ukpga_1974_37:h.18           — heading group starting at section 18
```

---

## 8. Building sort_key (Normalised Sort Encoding)

The sort_key makes provision numbers sortable lexicographically. Naive string sort puts `s.10` before `s.2` — the sort_key fixes this.

### Algorithm

**Input**: bare provision number (e.g., `3`, `3A`, `41ZA`, `19DZA`)
**Output**: three zero-padded segments joined by dots (e.g., `003.000.000`)

Steps:
1. Extract leading ASCII digits → base number, zero-padded to 3 digits
2. Parse remaining uppercase letters into up to 2 suffix groups:
   - **Z-prefix** (Parliament's "insert before A" mechanism): `ZA=001, ZB=002, ..., ZZ=026`
   - **Plain letter**: `A=010, B=020, ..., Z=260` (gaps of 10 for nested insertions)
3. Pad to exactly 3 segments with `000`
4. Join with `.`

### Why Z-prefixes sort before plain letters

Parliament uses Z-prefixes to insert *before* existing letter-suffixed sections. The sort order is:

```
s.3     → 003.000.000     (original section)
s.3ZA   → 003.001.000     (inserted BEFORE 3A by a later amendment)
s.3ZB   → 003.002.000
s.3A    → 003.010.000     (original amendment insertion)
s.3AA   → 003.010.010     (nested insertion after 3A)
s.3AB   → 003.010.020
s.3B    → 003.020.000
s.4     → 004.000.000
```

The 10x multiplier on plain letters (`A=010`) leaves:
- Gaps `001-009` for Z-prefix insertions
- Gaps `011-019` for double-letter nested insertions (AA, AB, etc.)

### Complete examples

| Provision | sort_key | Breakdown |
|-----------|----------|-----------|
| `3` | `003.000.000` | base=3, no suffix |
| `3ZA` | `003.001.000` | base=3, Z-prefix ZA=1 |
| `3ZB` | `003.002.000` | base=3, Z-prefix ZB=2 |
| `3A` | `003.010.000` | base=3, letter A=10 |
| `3AA` | `003.010.010` | base=3, letter A=10, nested letter A=10 |
| `3AB` | `003.010.020` | base=3, letter A=10, nested letter B=20 |
| `3B` | `003.020.000` | base=3, letter B=20 |
| `4` | `004.000.000` | base=4, no suffix |
| `19DZA` | `019.040.001` | base=19, letter D=40, Z-prefix ZA=1 |
| `19AZA` | `019.010.001` | base=19, letter A=10, Z-prefix ZA=1 |
| `41` | `041.000.000` | base=41 |
| `41A` | `041.010.000` | base=41, letter A=10 |
| `41B` | `041.020.000` | base=41, letter B=20 |
| `41C` | `041.030.000` | base=41, letter C=30 |
| `42` | `042.000.000` | base=42 |
| (empty) | `000.000.000` | fallback for structural rows |

### Which value feeds the sort key

| section_type | Input to normalize | Example |
|-------------|-------------------|---------|
| `section`, `sub_section`, `article`, `sub_article`, `paragraph`, `sub_paragraph` | `provision` | `25A` → `025.010.000` |
| `heading` | `heading_group` | `18` → `018.000.000` |
| Schedule `paragraph`/`sub_paragraph` without provision | `paragraph` value | `3` → `003.000.000` |
| `title`, `part`, `chapter`, `schedule`, `signed`, etc. | (none — structural) | `000.000.000` |

### Pseudocode (reference implementation)

```python
def normalize_provision(s: str) -> str:
    s = s.strip().upper()
    if not s:
        return "000.000.000"

    # Extract leading digits
    i = 0
    while i < len(s) and s[i].isdigit():
        i += 1
    base = int(s[:i]) if i > 0 else 0

    suffix = s[i:]
    segments = [base]

    j = 0
    while j < len(suffix) and len(segments) < 3:
        if suffix[j] == 'Z' and j + 1 < len(suffix) and suffix[j+1].isalpha():
            # Z-prefix: ZA=1, ZB=2, ..., ZZ=26
            segments.append(ord(suffix[j+1]) - ord('A') + 1)
            j += 2
        elif suffix[j].isalpha():
            # Plain letter: A=10, B=20, ..., Z=260
            segments.append((ord(suffix[j]) - ord('A') + 1) * 10)
            j += 1
        else:
            break

    # Pad to 3 segments
    while len(segments) < 3:
        segments.append(0)

    return f"{segments[0]:03d}.{segments[1]:03d}.{segments[2]:03d}"
```

### PostgreSQL implementation

```sql
CREATE OR REPLACE FUNCTION normalize_provision(s TEXT)
RETURNS TEXT LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    input TEXT;
    base_num INT;
    suffix TEXT;
    segments INT[] := ARRAY[]::INT[];
    j INT;
    ch CHAR;
    ch2 CHAR;
BEGIN
    input := upper(trim(coalesce(s, '')));
    IF input = '' THEN
        RETURN '000.000.000';
    END IF;

    -- Extract leading digits
    base_num := coalesce(
        (regexp_match(input, '^(\d+)'))[1]::INT, 0
    );
    suffix := regexp_replace(input, '^\d+', '');
    segments := ARRAY[base_num];

    j := 1;
    WHILE j <= length(suffix) AND array_length(segments, 1) < 3 LOOP
        ch := substr(suffix, j, 1);
        IF ch = 'Z' AND j + 1 <= length(suffix) THEN
            ch2 := substr(suffix, j + 1, 1);
            IF ch2 BETWEEN 'A' AND 'Z' THEN
                -- Z-prefix: ZA=1, ZB=2, ..., ZZ=26
                segments := segments || (ascii(ch2) - ascii('A') + 1);
                j := j + 2;
                CONTINUE;
            END IF;
        END IF;
        IF ch BETWEEN 'A' AND 'Z' THEN
            -- Plain letter: A=10, B=20, ..., Z=260
            segments := segments || ((ascii(ch) - ascii('A') + 1) * 10);
            j := j + 1;
        ELSE
            EXIT;
        END IF;
    END LOOP;

    -- Pad to 3 segments
    WHILE array_length(segments, 1) < 3 LOOP
        segments := segments || 0;
    END LOOP;

    RETURN lpad(segments[1]::TEXT, 3, '0') || '.'
        || lpad(segments[2]::TEXT, 3, '0') || '.'
        || lpad(segments[3]::TEXT, 3, '0');
END;
$$;
```

---

## 9. Parallel Territorial Provisions

Some UK laws have the same section number with different text for different territorial extents. Example: HSWA 1974 s.23(4) exists in three versions — England+Wales, Northern Ireland, and Scotland — each referencing different fire safety legislation.

### Detection

A `(law_name, provision)` pair is "parallel" when it exists with multiple distinct `extent_code` values.

### How it affects section_id

When parallels are detected for a law, an `[extent]` qualifier is appended:

```
UK_ukpga_1974_37:s.23[E+W]     — England+Wales version
UK_ukpga_1974_37:s.23[NI]      — Northern Ireland version
UK_ukpga_1974_37:s.23[S]       — Scotland version
```

Sections in laws **without** parallel provisions get no qualifier (the common case).

### How it affects sort_key

A `~extent` suffix is appended to the sort key:

```
023.000.000~E+W
023.000.000~NI
023.000.000~S
```

The tilde (`~`) sorts after digits and letters, so all versions of a section group together. Within a section, extent variants sort alphabetically.

### Scale

In our UK dataset: 29 laws, 719 section-level rows have parallel provisions. This is relatively rare but must be handled correctly to avoid primary key collisions.

---

## 10. Disambiguation (Position Suffix)

After building the citation-based section_id, some rows may still collide — particularly heading/part/chapter rows that reset numbering across parts or schedules within the same law.

**Example**: a law with Part 1 containing heading "18" and Part 2 also containing heading "18" would produce two rows with the same `section_id` = `UK_xxx:h.18`.

### Resolution

1. Count occurrences of each base section_id across the whole law
2. Where count > 1, append `#position` to disambiguate:
   ```
   UK_xxx:h.18#25    — first occurrence (at position 25)
   UK_xxx:h.18#142   — second occurrence (at position 142)
   ```

In our dataset: 2,206 rows (2.3%) needed disambiguation.

**For new scrapers**: if your citation construction is more precise (e.g., including the parent part in heading citations), you may avoid most of these collisions. The position fallback is a safety net.

---

## 11. Hierarchy Path Construction

The `hierarchy_path` column is a slash-separated path showing the row's position in the document tree.

### Format

```
schedule.{N}/part.{N}/chapter.{N}/heading.{N}/provision.{N}/sub.{N}/para.{N}
```

Each segment is included only when that level has a value. Examples:

| Row | hierarchy_path |
|-----|---------------|
| Title row | (empty string or NULL) |
| Part 1 | `part.1` |
| Heading 18 in Part 1 | `part.1/heading.18` |
| Section 25A under heading 18 | `part.1/heading.18/provision.25A` |
| Sub-section (1) of s.25A | `part.1/heading.18/provision.25A/sub.1` |
| Schedule 2, paragraph 3 | `schedule.2/para.3` |

### Construction rules

- Schedule: from the `flow` column in our CSVs (values like `1`, `2`, `3` for schedule numbers; `pre`, `main`, `post`, `signed` are NOT schedule values and are skipped)
- Each level is `{label}.{value}` where label is: `schedule`, `part`, `chapter`, `heading`, `provision`, `sub`, `para`
- Segments joined with `/`
- Empty/NULL values at any level are skipped

---

## 12. Depth Calculation

The `depth` column counts populated structural hierarchy levels:

```sql
depth = (schedule IS NOT NULL)::INT
      + (part IS NOT NULL AND part != '')::INT
      + (chapter IS NOT NULL AND chapter != '')::INT
      + (heading_group IS NOT NULL AND heading_group != '')::INT
      + (provision IS NOT NULL AND provision != '')::INT
      + (sub_paragraph IS NOT NULL AND sub_paragraph != '')::INT
      + (paragraph IS NOT NULL AND paragraph != '')::INT
```

Title/root rows = 0, a section under a part/heading = 3, etc.

---

## 13. Amendment Annotation Counts

Each content row carries counts of how many annotations of each type apply to it.

| Column | Source | Counting method |
|--------|--------|----------------|
| `amendment_count` | `Changes` column on the content row | Count comma-separated codes starting with `F` |
| `modification_count` | C/I/E annotation rows | Count child rows with Record_Type = `modification,content` |
| `commencement_count` | C/I/E annotation rows | Count child rows with Record_Type = `commencement,content` |
| `extent_count` | C/I/E annotation rows | Count child rows with Record_Type in (`extent,content`, `editorial,content`) |
| `editorial_count` | C/I/E annotation rows | Count child rows with Record_Type = `editorial,content` |

**C/I/E annotation linkage**: each annotation row has an ID like `{parent_section_id}_cx_1`. Stripping the suffix (`_cx_N`, `_mx_N`, `_ex_N`, `_ax_N`, `_xx_N`, `_px_N`) recovers the parent content row's ID.

**For new scrapers**: if you're scraping legislation.gov.uk XML directly, you can count the `<CommentaryRef>` elements per provision grouped by commentary type (F/C/I/E).

---

## 14. Annotation ID Synthesis

Each annotation gets a synthetic unique ID:

```
{law_name}:{code_type}:{seq}
```

Where:
- `law_name` is the canonical law identifier
- `code_type` is `amendment`, `modification`, `commencement`, or `extent_editorial`
- `seq` is a per-law, per-code_type counter (1, 2, 3...) assigned during export

Example: `UK_ukpga_1974_37:amendment:1`, `UK_ukpga_1974_37:modification:1`

The ordering for `seq` assignment: sorted by `code` (e.g., F1 before F2), then `source`, then original ID.

---

## 15. Annotation affected_sections Linkage

The `affected_sections` column is an array of `section_id` values from `legislation_text` that each annotation applies to. Three linkage mechanisms:

### C/I/E annotations (from interleaved annotation rows in the law text)

Each annotation row ID has a suffix indicating its parent:
```
{parent_section_id}_cx_1  → modification
{parent_section_id}_mx_1  → commencement
{parent_section_id}_ex_1  → extent/editorial
```

Strip the suffix → get the parent content row's section_id. Result: `affected_sections = [parent_section_id]` (always exactly one).

### F-code annotations from LAT (textual amendments interleaved in the law text)

Content rows carry a `Changes` column: comma-separated F-codes like `"F3,F2,F1"`. This is **inverted**: for each F-code, find all content rows that reference it in their Changes column → those are the affected sections.

### F-code annotations from AMD files (separate amendment CSV files)

These have an `Articles` column containing comma-separated section IDs directly. Map each through ID normalisation to get the citation-based section_ids.

### Coverage

- C/I/E: 100% linkage (structural parent relationship)
- F from LAT: ~87% (588 F-codes have no content rows referencing them in Changes)
- F from AMD: 100% (direct cross-reference)

### Source column

The `source` column on annotations records provenance:

| source | Meaning | Count in our data |
|--------|---------|-------------------|
| `lat_cie` | C/I/E annotations from LAT CSV | 7,522 |
| `lat_f` | F-code annotations from LAT CSV | 588 |
| `amd_f` | F-code annotations from AMD CSV | 11,341 |

---

## 16. Exclusions and Filters

### Rows excluded from legislation_text

1. **NULL text rows** (249 rows): leaked non-UK rows with NULL text, law_name, section_type. Filtered with `WHERE text IS NOT NULL`.

2. **UK_uksi_2016_1091** (~1,342 rows): The Electromagnetic Compatibility Regulations 2016. Parser bug caused 606 duplicate annotation IDs due to heavy post-Brexit territorial duplication. Excluded from all exports pending parser fix.

3. **Annotation rows**: rows with Record_Type indicating they're annotations rather than content (see §3 Content Row Detection). These go to `amendment_annotations` instead.

4. **Annotation heading rows**: `commencement,heading`, `modification,heading`, etc. — grouping labels from legislation.gov.uk rendering. Carry no data beyond what `code_type` provides. Dropped entirely.

### Result

- `legislation_text`: 97,522 rows from 452 UK laws
- `amendment_annotations`: 19,451 rows from 137 UK laws
- `annotation_totals`: 135 laws

---

## Transform Order Summary

For each scraped law, the transforms apply in this order:

1. Normalise law_name (strip acronyms if migrating legacy data)
2. Separate content rows from annotation rows
3. Map Record_Type → section_type
4. Merge section/article → provision
5. Map Region → extent_code
6. Detect parallel territorial provisions for the law
7. Assign position (1-based row number in document order within the law)
8. Build citation string from section_type + structural columns
9. Add `[extent]` qualifier if parallel provisions detected
10. Disambiguate colliding section_ids with `#position` suffix
11. Construct `section_id` = `{law_name}:{citation}[{extent}]`
12. Build sort_key by normalising the provision/heading_group number
13. Add `~extent` to sort_key if parallel provisions
14. Build hierarchy_path from structural columns
15. Calculate depth
16. Count amendment annotations per section
17. Build annotation rows with synthetic IDs and affected_sections linkage
