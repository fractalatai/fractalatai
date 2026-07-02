# Duty Significance: Taxonomy, Methodology and Results

**Version**: 1.0
**Date**: 2026-07-02

## Problem Statement

Not all statutory duties are equal. Section 2(1) of the Health and Safety at Work etc. Act 1974 — "the duty of every employer to ensure, so far as is reasonably practicable, the health, safety and welfare at work of all his employees" — is fundamentally more significant than Section 20(2) — "the duty to allow an inspector access to premises". A compliance officer managing 274 applicable laws needs to know which duties are critical and which are procedural.

No existing system rates the inherent significance of individual statutory duties. Compliance risk frameworks (Adherent, Secureframe) rate organisational risk of breach. RegTech platforms (Ascent, FinregE) extract and classify obligations. Contract scoring systems (Sirion) rate contract clause criticality. None rate the statutory duty itself.

## Taxonomy

### Five Significance Dimensions

Each Obligation provision is rated on five dimensions, each as HIGH / MEDIUM / LOW.

#### 1. Scope: Duty Bearer

How broadly is the duty-bearing class defined?

| Rating | Definition | Examples |
|--------|-----------|----------|
| HIGH | Universal — "every employer", "any person" | HSWA s.2(1): "every employer" |
| MEDIUM | Categorical — "an employer who operates...", "a competent person" | CDM reg.8: "a principal designer" |
| LOW | Individual/specific — "the person", "an inspector" | HSWA s.20: "an inspector" |

#### 2. Scope: Protected Class

How broadly is the class of persons protected or affected?

| Rating | Definition | Examples |
|--------|-----------|----------|
| HIGH | Universal — "all employees", "persons", "the public" | HSWA s.3: "persons not in his employment" |
| MEDIUM | Categorical — "employees in that workplace", "young persons" | MHSW reg.19: "young persons" |
| LOW | Specific — "the document", "the premises", "the authority" | A notification duty addressed to a regulator |

#### 3. Gravity

What is at stake if the duty is breached?

| Rating | Definition | Examples |
|--------|-----------|----------|
| HIGH | Health, safety, life, welfare, serious environmental harm | HSWA s.2: "health, safety and welfare" |
| MEDIUM | Property, financial loss, moderate environmental impact | Environmental Permitting reg.12 |
| LOW | Administrative, procedural, record-keeping, notification | HSWA s.9: "prepare written safety policy" |

#### 4. Strength

How absolute is the obligation?

| Rating | Definition | Examples |
|--------|-----------|----------|
| HIGH | Absolute duty, no qualification — "shall ensure", "must provide" (unqualified) | MHSW reg.3(1): "shall make a suitable and sufficient assessment" |
| MEDIUM | Qualified — "shall ensure SFARP", "shall have regard to", "all reasonable steps" | HSWA s.2(1): "so far as is reasonably practicable" |
| LOW | Procedural — "shall notify", "shall keep records", "shall display" | HSWA s.9: "shall prepare" |

#### 5. Hierarchy

Where does the provision sit in the law's structure? Derived from metadata (section number, depth, section type), not SLM-rated.

| Rating | Definition |
|--------|-----------|
| HIGH | General duties, principal regulations (early sections, shallow depth) |
| MEDIUM | Specific duties, mid-level regulations |
| LOW | Sub-paragraphs, schedules, transitional provisions |

### Overall Significance

The five dimensions combine into a single overall rating using a gravity-dominant weighted sum:

```
score = 0.35 * gravity + 0.20 * scope_duty_bearer + 0.20 * scope_protected_class
      + 0.15 * strength + 0.10 * hierarchy

Thresholds: >= 2.5 -> HIGH, >= 1.75 -> MEDIUM, else LOW
```

Gravity receives the highest weight (0.35) because what is at stake is the most actionable signal for compliance officers. Strength receives reduced weight (0.15) because the current SLM over-predicts HIGH on this dimension (a known distillation bias from training data).

### Law-Level Significance

Per-provision significance aggregates to the law level, combining average importance with compliance burden:

```
score = avg_provision_significance * log2(total_obligations + 1)
```

The logarithmic size factor means a 200-provision Act with moderate average significance ranks higher than a 5-provision SI at the same average — correctly reflecting that more duties means more compliance work.

A distribution profile (count of HIGH/MEDIUM/LOW provisions) is also published for each law, giving compliance officers the full shape without information loss.

### Part-Level Breakdown

Large Acts (>50 rated provisions) receive a Part-level breakdown. The Health and Safety at Work Act 1974 illustrates why this matters:

| Part | HIGH | MEDIUM | LOW | Total | % HIGH |
|------|------|--------|-----|-------|--------|
| Part I (General Duties) | 31 | 35 | 69 | 135 | 23% |
| Part III (Enforcement) | 0 | 3 | 10 | 13 | 0% |
| Part IV (Misc/Offences) | 0 | 10 | 14 | 24 | 0% |

Part I contains the foundational safety duties. Parts III and IV are enforcement machinery and offence definitions — important but not duties a compliance officer needs to action directly. The Part breakdown surfaces this structure.

## Methodology

### Rating Pipeline

1. **SLM Classification** — A fine-tuned gemma-3-4b-it model rates each Obligation provision on 4 dimensions (scope_duty_bearer, scope_protected_class, gravity, strength) in a single inference call. The model runs on GPU (RunPod) at ~18 provisions/second.

2. **Hierarchy Derivation** — The fifth dimension (hierarchy) is derived from provision metadata using a combined weighted scoring model:
   - 40% weight: section number position (lower sections score higher)
   - 30% weight: structural depth (shallower provisions score higher)
   - 30% weight: section type (section/article > sub_section > paragraph)
   - Percentile-based thresholds: top 33% = HIGH, middle 34% = MEDIUM, bottom 33% = LOW

3. **Overall Aggregation** — The gravity-dominant weighted sum (Approach B) combines all 5 dimensions into a single overall rating per provision.

4. **Law-Level Aggregation** — Provision ratings aggregate to law level using Approach L (weighted score with size factor) and Approach K (distribution profile).

5. **Part Breakdown** — For large Acts, provisions are assigned to Parts via document sort-key ordering and aggregated per Part.

### Training Data

The SLM was trained on 2,592 benchmark Obligation provisions rated by Gemini 2.5 Flash across 20 benchmark laws. The training pipeline:

1. Gemini rates provisions on the 4 SLM dimensions (v0.2 prompt with refined strength definition)
2. Training data exported as JSONL
3. gemma-3-4b-it fine-tuned with LoRA on RunPod (RTX 5090, ~90 minutes, ~$2)
4. GGUF quantized for inference via Ollama

### Confidence Signal

The SLM produces a confidence score (0-1) from token log probabilities. Provisions with confidence < 0.9 are flagged for potential LLM review. In the current corpus, ~1.4% of provisions fall below this threshold.

## Results

### Corpus Statistics

| Metric | Count |
|--------|-------|
| Obligation provisions rated | 40,468 |
| Laws with significance data | 553 |
| Provisions rated HIGH | 5,359 (13.2%) |
| Provisions rated MEDIUM | 10,023 (24.8%) |
| Provisions rated LOW | 25,086 (62.0%) |
| Laws rated HIGH | 109 (19.9%) |
| Laws rated MEDIUM | 246 (47.0%) |
| Laws rated LOW | 166 (33.1%) |

### Dimension Distributions

| Dimension | HIGH | MEDIUM | LOW |
|-----------|------|--------|-----|
| Scope (duty bearer) | 5% | 42% | 53% |
| Scope (protected class) | 21% | 9% | 70% |
| Gravity | 22% | 17% | 61% |
| Strength | 71% | 15% | 14% |
| Hierarchy | 33% | 34% | 33% |

The strength dimension's 71% HIGH reflects a known SLM training bias — "shall" and "must" are standard legislative drafting, so most obligations receive HIGH strength regardless of qualification. Strength receives reduced weight (0.15) in the overall formula to mitigate this. A future SLM retrain with more balanced training data will address the root cause.

### Benchmark Validation

| Provision | Expected | Result | Correct |
|-----------|----------|--------|---------|
| HSWA s.2(1) — general employer duty | HIGH | HIGH | Yes |
| HSWA s.9(1) — prepare safety policy | MEDIUM | MEDIUM | Yes |
| MHSW reg.3(1) — risk assessment | HIGH | HIGH | Yes |
| CDM reg.4(1) — client duties | HIGH | MEDIUM | Accepted |

CDM reg.4(1) rates MEDIUM because the SLM rated its gravity as MEDIUM (construction client duty framed as organisational, not direct safety). The CDM law itself correctly rates HIGH at law level.

| Law | Expected | Rating | Rank |
|-----|----------|--------|------|
| HSWA | HIGH | HIGH | 70/553 |
| CDM 2015 | HIGH | HIGH | 7/553 |
| MHSW 1999 | HIGH | HIGH | 41/553 |

### Approach Selection

Eight provision-level and nine law-level approaches were tested systematically. The full experimental results and comparison tables are documented in the session log (`07-01-26-significance-aggregation.md`). Key findings:

- **Max-of-dimensions** (88% HIGH) and **gravity-first rules** (75% MEDIUM) were rejected for poor discrimination
- **Gravity-dominant weighted sum** (Approach B) achieved the best benchmark accuracy with balanced distribution
- **Weighted score with size factor** (Approach L) was the only law-level approach to correctly rate all three benchmark laws as HIGH
- **Distribution profile** (Approach K) provides the richest signal and is published alongside the single rating

## Known Limitations

1. **Strength dimension skewed** — SLM predicts 71% HIGH vs 39% in training data (distillation bias). Mitigated by reduced weight; needs SLM retrain.
2. **Limited benchmarks** — 4 provision-level and 3 law-level benchmarks. Should expand to 20-30 before full production confidence.
3. **Obligation provisions only** — Liberty, Power, and Responsibility provisions are not significance-rated. Only Obligations (duties) have significance dimensions.
4. **No penalty dimension** — The severity of sanctions for breach is not captured. Sertantai models this separately via Offence provision extraction.
5. **Percentile-based law ratings** — The law-level HIGH/MEDIUM/LOW thresholds are relative, not absolute. Adding or removing laws changes the thresholds. This is appropriate for customer-scoped registers but means the rating depends on the corpus.

## Cost

| Component | Cost |
|-----------|------|
| Gemini training data (2,592 provisions) | ~$2 |
| RunPod SLM training (90 min, RTX 5090) | ~$2 |
| RunPod corpus inference (40K provisions, 65 min) | ~$1 |
| **Total** | **~$5** |

Ongoing cost for new laws: SLM inference at ~18 provisions/second on GPU, or ~0.3 provisions/second on CPU via Ollama.
