# Cultural Graph: Initial Architectural Review

*2026-06-29 — Assessing fit between the Cultural Graph working schema and the fractalaw hub-and-spoke architecture*

## Source Documents

- `dialectics/output/safety-culture-dialectic/cultural-graph-working-schema.md` — the extraction protocol (5P nodes, ~14 cultural edge types, 6 triads, back-channel, validation architecture)
- `.claude/plans/micro-apps.md` — fractalaw micro-app brainstorm and Phase 3 priorities
- `docs/fractal-plan.md` — full fractal architecture research and planning document

---

## 1. Summary Assessment

**The cultural graph extraction task is a natural fit for the fractalaw edge-AI architecture.** The schema defines a structured information extraction problem (entities, relationships, classifications from narrative text) — exactly the workload profile that fine-tuned small language models on edge devices are designed for. The hub-and-spoke pattern maps directly to the schema's validation and revision lifecycle.

---

## 2. How the Pieces Map

### 2.1 Cultural Graph Tasks → Fractalaw Capabilities

| Cultural Graph Task | Fractalaw Equivalent | Infrastructure |
|---|---|---|
| Entity extraction (5P nodes from narrative) | Structured extraction | `fractal:ai/inference` or `fractal:ai/classify` on edge SLM |
| Relationship extraction (cultural edges) | Structured extraction | Same SLM, span-pair classification output |
| Triad positioning (6 barycentric coordinates) | Classification | ONNX classifier or second-stage SLM head |
| Back-channel collection (unmatched/low-confidence) | Confidence gating | Same pattern as DRRP classifier confidence thresholds |
| Parallel open-extraction (Layer 4, 10% sample) | Hub-side validation | Hub runs larger model (8B+) without schema guidance |
| Human validation (Layer 4, 5% sample) | Human-in-the-loop | Audit log + review workflow |
| Schema revision (quarterly) | Model retraining + hot-swap | New `.wasm` + model artifact pushed to edge via sync |

### 2.2 Data Flow: Edge → Hub → Edge

```
EDGE (site)                              HUB
─────────────                            ───
Worker tells narrative
        │
        ▼
[Narrative Graph Extractor]              
  SLM extracts:                          
  - 5P entity spans                      
  - cultural edge triples                
  - confidence scores                    
        │                                
  [Triad Classifier]                     
  ONNX positions narrative               
  on 6 triads (barycentric)              
        │                                
        ▼                                
  Arrow RecordBatch:                     
  (source, edge_type, target,            
   confidence, triad_coords[6],          
   narrative_ref, back_channel[])        
        │                                
        │──── Flight DoPut ────────────► [Graph Aggregator]
                                           Merges fragments across sites
                                           Runs Layer 4 validation:
                                             - 10% open-extraction comparison
                                             - 5% human validation sample
                                           Feeds back-channel clustering
                                                │
                                           [Graph Model Trainer]
                                             Quarterly: retrain edge SLM
                                             from corrected labels
                                                │
                                           New model artifact
                                                │
        ◄──── Flight DoGet / Sync ────────────┘
  Hot-swap: updated SLM
  + revised schema config
```

### 2.3 The Provision Bridge

The cultural graph's 5th P — "Provision" (outputs, services, capabilities delivered and received) — creates a natural join point with fractalaw's existing DRRP data. When a narrative references a regulatory provision ("we followed the permit-to-work procedure"), the edge SLM can link it to a provision already in the local LanceDB partition.

This is where regulatory compliance data and lived safety culture data meet in the same graph. A `works-around` edge linking a team to a specific DRRP provision is qualitatively different evidence from a compliance gap analysis — it's the workforce's own voice saying "this is what actually happens."

---

## 3. Proposed Micro-Apps

### 3.1 Narrative Graph Extractor (Edge)

| Attribute | Detail |
|-----------|--------|
| **One-liner** | Extracts 5P entities and cultural edge triples from organisational safety narratives using a fine-tuned SLM |
| **Runs on** | Edge |
| **Tier** | Standard (64MB memory, 1B fuel, 30s timeout) |
| **WIT imports** | `fractal:ai/inference`, `fractal:ai/classify`, `fractal:events/emit`, `fractal:audit/log` |
| **AI model** | Fine-tuned SLM (1-3B, Q4_K_M quantised, ONNX or GGUF) |
| **Input** | Narrative text (collected on-site from worker/team) |
| **Output** | Arrow batch: `(source_node, source_type, edge_type, target_node, target_type, confidence, narrative_id)` + back-channel residuals |
| **User story** | A safety practitioner at a manufacturing site facilitates a learning team after a near-miss. The team's narrative is captured on a tablet. The Narrative Graph Extractor processes it locally: "Team Alpha `works-around` the isolation procedure for the compressor" (confidence 0.87), "Team Alpha `speaks-up-to` Shift Supervisor" (confidence 0.72), "Shift Supervisor `defers-to-by-rank` Plant Manager" (confidence 0.61). The low-confidence extraction on `defers-to-by-rank` is flagged for back-channel review. No narrative text leaves the site — only structured graph triples sync to the hub. |

### 3.2 Triad Classifier (Edge)

| Attribute | Detail |
|-----------|--------|
| **One-liner** | Positions a narrative on 6 experiential triads (barycentric coordinates) as provenance metadata for extracted edges |
| **Runs on** | Edge |
| **Tier** | Lightweight (16MB memory, 100M fuel, 5s timeout) |
| **WIT imports** | `fractal:ai/classify`, `fractal:audit/log` |
| **AI model** | ONNX classifier (fine-tuned on triad-labelled narratives, single-digit MB) |
| **Input** | Narrative text |
| **Output** | 6 x 3-element vectors (barycentric coordinates per triad) |
| **User story** | The same near-miss narrative is classified: Governance=(0.15, 0.10, 0.75) — strongly Adaptation; Practice=(0.10, 0.30, 0.60) — Workaround-dominant; Voice=(0.65, 0.20, 0.15) — Speaking Up. This context travels with every edge extracted from this narrative. The hub can later ask: "Are `works-around` edges predominantly found in Adaptation-heavy contexts?" — a question neither the edge types nor the triads alone could answer. |

### 3.3 Graph Aggregator (Hub)

| Attribute | Detail |
|-----------|--------|
| **One-liner** | Merges graph fragments from multiple edge sites, runs Layer 4 validation, and maintains the back-channel |
| **Runs on** | Hub |
| **Tier** | Heavy (256MB memory, 10B fuel, 120s timeout) |
| **WIT imports** | `fractal:data/query`, `fractal:data/mutate`, `fractal:ai/inference`, `fractal:events/emit`, `fractal:audit/log` |
| **AI model** | Larger model (8B) for parallel open-extraction; embeddings for back-channel clustering |
| **Input** | Edge-synced graph triple batches |
| **Output** | Aggregated graph in DuckDB/LanceDB; back-channel clusters; validation metrics |
| **Responsibilities** | (1) Merge edge fragments into unified graph store. (2) Run parallel open-extraction on 10% sample (no schema guidance) — compare with schema-guided edge extractions. (3) Queue 5% sample for human validation. (4) Cluster unmatched back-channel items by semantic similarity. (5) Compute standing governance metrics (capture rate, back-channel ratio, schema-vs-open agreement). (6) Emit `schema-revision-needed` event when thresholds are breached. |

### 3.4 Graph Model Trainer (Hub)

| Attribute | Detail |
|-----------|--------|
| **One-liner** | Retrains the edge SLM when the cultural graph schema revises, using corrected labels from hub validation |
| **Runs on** | Hub (or RunPod GPU for larger training runs) |
| **Tier** | Heavy |
| **WIT imports** | `fractal:data/query`, `fractal:events/emit`, `fractal:audit/log` |
| **Trigger** | Quarterly, or on `schema-revision-needed` event |
| **Output** | New quantised model artifact (ONNX/GGUF) for edge deployment |
| **Training data** | Human-validated extractions + hub open-extraction corrections + back-channel resolutions |

---

## 4. Key Design Decisions

### 4.1 Two-Stage Extraction (Recommended)

**Option A — Two-stage**: SLM extracts entities + edges; separate ONNX classifier positions triads.
**Option B — Single model**: One fine-tuned model does both extraction and triad positioning.

**Recommendation: Option A.** Reasons:

1. **Fractal principle** — each micro-app does one thing. The Narrative Graph Extractor and Triad Classifier are independently testable, deployable, and updatable.
2. **Validation** — triad positioning can be validated against human judgement independently of entity/edge extraction accuracy. If one component degrades, you know which.
3. **Resource efficiency** — the triad classifier is a lightweight ONNX model (single-digit MB, millisecond inference). The SLM is heavier (1-3B parameters). Separating them means the triad classifier runs in the `lightweight` tier.
4. **Schema evolution** — edge types change quarterly; triads are more stable (they're experiential dimensions, not data categories). Decoupling means you retrain the SLM without touching the triad classifier.

### 4.2 Confidence Calibration is Critical

The back-channel ratio is the schema's most important governance metric. If the edge SLM is over-confident (force-fitting narratives into the schema), the back-channel underreports and the falsification mechanism breaks.

Mitigation:
- **Calibrate on the hub** — the parallel open-extraction comparison (Layer 4) measures actual SLM accuracy, not just self-reported confidence.
- **Threshold tuning** — the Graph Aggregator adjusts the back-channel confidence threshold based on observed agreement rates. If schema-vs-open agreement drops below 70%, lower the confidence threshold on the edge SLM (more goes to back-channel).
- **Edge-side conservatism** — default the edge SLM to produce *more* back-channel items, not fewer. The cost of a false negative (missing a real relationship) is lower than a false positive (force-fitting the wrong type).

### 4.3 Data Sovereignty Advantage

Narratives are sensitive — they describe workplace relationships, power dynamics, workarounds. The architecture's strongest property here is that **narrative text never leaves the site**. The edge SLM processes locally; only structured graph triples (entity-edge-entity tuples with confidence scores) sync to the hub. The raw narrative stays on the edge device, referenced by ID.

This is a significant advantage over cloud-based NLP pipelines for safety culture data. It also simplifies GDPR compliance — personal data (names, roles mentioned in narratives) is processed locally and only appears in the graph as anonymised node references (team/role, not individual).

### 4.4 Training Data Bootstrap

The cultural graph schema has no labelled corpus yet. Bootstrap sequence:

1. **Deliverable 2** (this quarter): Zero-shot Open IE on 200-300 existing narratives from My Health and Safety Activity. Produces seed extractions.
2. **Hub large-model pass**: Run 8B+ model with schema guidance on the seed narratives. Produces candidate labels.
3. **Human review**: Domain experts validate/correct the candidate labels. This is the ground truth.
4. **First fine-tune**: Train the edge SLM on the validated labels. Deploy to edge.
5. **Quarterly refinement**: Each quarter's human validation sample (5%) and back-channel resolutions produce new training data. Retrain.

This is the same pattern as the DRRP pipeline: Elixir regex produces rough labels → hub AI polishes → human validates → edge model fine-tuned on the result.

---

## 5. Arrow Schema Sketch

### 5.1 Graph Triples Table

```
cultural_graph_triples {
    triple_id:        utf8          -- unique ID
    narrative_id:     utf8          -- FK to source narrative
    source_node:      utf8          -- entity text span
    source_type:      utf8          -- enum: people, plant, process, place, provision
    edge_type:        utf8          -- enum: trusts, speaks_up_to, learns_from, ...
    target_node:      utf8          -- entity text span
    target_type:      utf8          -- enum: people, plant, process, place, provision
    confidence:       float32       -- extraction confidence [0, 1]
    back_channel:     bool          -- true if below confidence threshold
    model_version:    utf8          -- edge SLM version that produced this
    extracted_at:     timestamp[ns] -- UTC
    site_id:          utf8          -- originating site
    provision_ref:    utf8          -- optional FK to DRRP provision (the bridge)
}
```

### 5.2 Narrative Triads Table

```
narrative_triads {
    narrative_id:     utf8          -- FK to source narrative
    -- Governance triad (barycentric, sums to 1.0)
    gov_control:      float32
    gov_autonomy:     float32
    gov_adaptation:   float32
    -- Orientation triad
    ori_compliance:   float32
    ori_learning:     float32
    ori_delivery:     float32
    -- Attribution triad
    att_individual:   float32
    att_system:       float32
    att_environment:  float32
    -- Practice triad
    pra_formal:       float32
    pra_informal:     float32
    pra_workaround:   float32
    -- Voice triad
    voi_speaking_up:  float32
    voi_silence:      float32
    voi_normalisation: float32
    -- Trade-off triad
    trd_efficiency:   float32
    trd_thoroughness: float32
    trd_resilience:   float32
    -- Metadata
    classifier_version: utf8
    classified_at:    timestamp[ns]
    site_id:          utf8
}
```

### 5.3 Back-Channel Table

```
back_channel {
    item_id:          utf8
    narrative_id:     utf8
    item_type:        utf8          -- enum: unmatched_entity, unmatched_relationship, low_confidence
    raw_text:         utf8          -- the phrase that didn't match
    nearest_type:     utf8          -- closest schema type (if any)
    nearest_score:    float32       -- similarity to nearest type
    cluster_id:       utf8          -- assigned during quarterly clustering (null initially)
    resolved:         bool          -- true after quarterly review
    resolution:       utf8          -- enum: new_type_proposed, merged_with_existing, noise, ...
    created_at:       timestamp[ns]
    site_id:          utf8
}
```

---

## 6. Narrative Sources and Ingestion Apps

Two concrete narrative sources have been identified. They have very different characteristics and drive two distinct edge app designs.

### 6.1 Source A: Cloud Reporting Tool (Excel Dumps)

**What it is:** An existing cloud-based reporting tool for accidents, hazards, positive observations, etc. No useful API. Content is dumped monthly into Excel spreadsheets by the safety team.

**The edge app: Reporting Data Extractor**

| Attribute | Detail |
|-----------|--------|
| **One-liner** | Ingests monthly Excel dumps from the cloud reporting tool, extracts cultural graph triples from incident/observation narratives |
| **Runs on** | Edge — deployed by the safety team on their workstation or a team laptop |
| **Tier** | Standard |
| **WIT imports** | `fractal:ai/inference`, `fractal:ai/classify`, `fractal:data/mutate`, `fractal:events/emit`, `fractal:audit/log` |
| **AI model** | Fine-tuned SLM (entity/edge extraction) + ONNX triad classifier |
| **Input** | Excel file (`.xlsx`) — the monthly dump |
| **Output** | Graph triples + triad positions → local store; structured signal → hub sync |
| **Persists locally** | The raw narratives from Excel, stored in local LanceDB. Never synced to hub. |

**Flow:**

```
Safety team receives monthly Excel dump
        │
        ▼
[Reporting Data Extractor]
  1. Parse Excel → Arrow RecordBatch
     (column mapping config per reporting tool — 
      narrative text, date, category, site, reporter role)
  2. Per narrative row:
     a. SLM extracts 5P entities + cultural edges
     b. ONNX classifier positions triads
     c. Confidence gating → back-channel residuals
  3. Store raw narratives locally (LanceDB, encrypted at rest)
  4. Store graph triples locally
  5. Emit signal batch for hub sync:
     - graph triples (no narrative text)
     - triad coordinates
     - back-channel items (phrases only, no full narrative)
     - source metadata (report_type, date, site, anonymised)
```

**Key design points:**

- **Column mapping is configurable.** Different organisations use different reporting tools with different column layouts. The app needs a config step where the safety team maps "which column is the narrative text, which is the date, which is the site?" This could be a one-time setup wizard or a persistent config file.
- **Batch processing.** This is a monthly batch, not real-time. The app processes hundreds of rows at once. The SLM runs inference in a loop — on a decent laptop with a 1-3B quantised model, expect 2-5 seconds per narrative, so a 500-row Excel takes 15-40 minutes. Acceptable for a monthly batch.
- **Deduplication.** The same incident may appear across months if the reporting tool exports rolling windows. Hash the narrative text + date + site to detect duplicates.
- **Report types as metadata.** Accidents, hazards, and positive observations are different narrative genres. An accident report skews toward `responds-to-failure-of`, `blames`, `learns-from`. A positive observation skews toward `recognises`, `trusts`, `cooperates-with`. The report type is valuable metadata that travels with the graph triples — it's a covariate for analysis, not a filter.

### 6.2 Source B: Randomised Sampling Survey (New Design)

**What it is:** A survey instrument designed from scratch, with questions aligned to the cultural graph schema. Deployed to a randomised sample of the workforce. The critical innovation: the survey app doesn't just collect answers — it uses the edge SLM to help respondents articulate their narratives, while simultaneously extracting the cultural graph signal.

**The edge app: Culture Survey**

| Attribute | Detail |
|-----------|--------|
| **One-liner** | An AI-assisted survey tool that helps respondents narrate workplace experiences, extracts cultural graph signal in real-time, persists narratives locally |
| **Runs on** | Edge — available to individual survey respondents on a tablet, phone (via browser), or shared kiosk |
| **Tier** | Standard |
| **WIT imports** | `fractal:ai/inference`, `fractal:ai/classify`, `fractal:events/emit`, `fractal:audit/log` |
| **AI model** | Fine-tuned SLM (dual role: conversational facilitation + extraction) |
| **Input** | Respondent's spoken or typed narrative, guided by survey prompts |
| **Output** | Graph triples + triad positions → local store → hub sync (signal only). Raw narratives persisted locally, never leave the device/site. |

**The conversational facilitation concept:**

This is where it gets genuinely novel. The SLM isn't just a post-hoc extractor — it's a real-time conversational partner that helps the respondent give a richer narrative. Think of it as a digital version of a skilled learning team facilitator who asks good follow-up questions.

```
SURVEY FLOW (respondent's perspective)

1. PROMPT: "Think of a recent situation at work where something 
   went well or didn't go as expected. In a few sentences, 
   describe what happened."

   Respondent: "We had a problem with the conveyor belt last 
   Tuesday. It kept jamming and we had to keep stopping the line."

2. SLM ANALYSIS (invisible to respondent):
   - Entities detected: [team=implicit "we", plant="conveyor belt"]
   - Edges detected: [adapts-to (weak signal)]
   - Missing: who was involved? what did people do? 
     who decided what?

3. FOLLOW-UP (generated by SLM, selected from question bank):
   "You mentioned the conveyor kept jamming. When the line had 
    to stop, how did the team decide what to do? Was there a 
    procedure, or did people figure it out as they went?"

   Respondent: "There's supposed to be a lockout procedure but 
   it takes ages so Dave just holds the guard open while someone 
   clears the jam. The supervisor knows but doesn't say anything 
   because we'd never hit target otherwise."

4. SLM ANALYSIS:
   - Entities: [person_role="Dave"→team_member, 
     process="lockout procedure", person_role="supervisor"]
   - Edges: [works-around (high confidence), 
     monitors (supervisor→team, inverted — failure to monitor),
     adapts-to (team→target pressure)]
   - Triads: Practice=(0.05, 0.25, 0.70) strongly Workaround;
     Trade-off=(0.70, 0.10, 0.20) Efficiency-dominant;
     Voice=(0.10, 0.70, 0.20) Silence-dominant

5. FOLLOW-UP: "Has anyone raised the time the lockout procedure 
    takes with management or the safety team?"

   ... and so on for 3-5 exchanges.

6. CLOSE: "Thank you. Your response has been recorded and will 
    be kept confidential at this site."
```

**Key design points:**

- **The SLM has a dual role.** It simultaneously (a) facilitates the conversation by generating contextually relevant follow-up questions that probe for missing graph signal, and (b) extracts entities, edges, and triad positions from the respondent's answers. This is the hardest model design challenge in the whole system — the facilitation and extraction tasks share the same context window but serve different purposes.

- **Question bank, not free generation.** The follow-up questions should be drawn from a curated bank of 50-100 questions, each tagged with the edge types and entity types they're designed to elicit. The SLM selects the most relevant question given what's been said and what's missing — it doesn't generate questions from scratch. This is safer (no hallucinated or leading questions), more consistent, and easier to validate. The SLM's role is selection and light adaptation (inserting specific references from the narrative), not open generation.

- **Narratives persist locally and never leave.** This is a hard architectural requirement. The narratives contain identifiable details (names, specific incidents, management criticism). They are stored in encrypted local LanceDB on the site device. Only the extracted graph signal (anonymised entity-edge triples, triad coordinates, back-channel items) syncs to the hub. The safety team at the site can review the narratives locally if needed; the hub sees only the structured signal.

- **Anonymisation at the extraction boundary.** When the SLM extracts entities, personal names are replaced with role labels: "Dave" becomes `team_member_1`, "the supervisor" becomes `shift_supervisor`. The mapping is stored locally (for the safety team's reference) but the hub only ever sees the role labels. This is where GDPR compliance lives — personal data processing happens on-device, only pseudonymised structured data crosses the network boundary.

- **Randomised sampling is a survey design concern, not an app concern.** The app doesn't need to handle randomisation. The safety team selects who gets surveyed (stratified random sample by team, shift, role). The app is given to those people. The app doesn't know or care about the sampling frame — that's the safety team's methodology.

- **Respondent experience matters.** If the SLM follow-ups feel like an interrogation, people will give monosyllabic answers and the signal quality collapses. The facilitation model needs to be trained (or prompted) to be warm, curious, non-judgmental, and brief. The best learning team facilitators are like this — they make people feel heard, not examined. This is a fine-tuning objective, not just a system prompt.

- **Session length.** Target 5-8 minutes per respondent. 3-5 exchange turns. Longer sessions produce richer narratives but lower completion rates. The SLM needs to judge when enough signal has been extracted and gracefully close.

### 6.3 Two Apps, One Pipeline

Both apps feed the same downstream pipeline:

```
Source A: Reporting Data Extractor          Source B: Culture Survey
(monthly batch from Excel)                  (randomised sample, ongoing)
        │                                           │
        ▼                                           ▼
  Local narrative store                     Local narrative store
  (LanceDB, encrypted)                     (LanceDB, encrypted)
        │                                           │
        ▼                                           ▼
  Graph triples + triads                    Graph triples + triads
  + back-channel                            + back-channel
        │                                           │
        └──────────── Flight DoPut ─────────────────┘
                            │
                            ▼
                    [Graph Aggregator] (hub)
                      Merges both sources
                      Tags source_type (reporting_tool | survey)
                      Source type is an analytical dimension:
                        - Do reporting tool narratives and 
                          survey narratives produce different
                          edge type distributions?
                        - (Yes, almost certainly — reporting
                          tool is incident-biased, survey
                          captures everyday work)
```

**The two sources are complementary, not redundant.** The reporting tool captures what the organisation *notices* (incidents, hazards, near-misses — things that became visible). The survey captures what the organisation *doesn't notice* (everyday workarounds, normalised silence, routine trust relationships — things that are invisible until someone asks). The cultural graph needs both to avoid survivorship bias.

### 6.4 Revised Micro-App Inventory

| # | Micro-App | Location | Tier | New/Updated |
|---|-----------|----------|------|-------------|
| 1 | **Reporting Data Extractor** | Edge | Standard | NEW |
| 2 | **Culture Survey** | Edge | Standard | NEW |
| 3 | Narrative Graph Extractor | Edge | Standard | Becomes an internal component of #1 and #2, not a standalone app |
| 4 | Triad Classifier | Edge | Lightweight | Unchanged — used by both #1 and #2 |
| 5 | Graph Aggregator | Hub | Heavy | Updated — handles two source types |
| 6 | Graph Model Trainer | Hub | Heavy | Unchanged |

---

## 7. Governance Metrics (Hub-Side Dashboards)

These map directly from the schema's Section 8, computed by the Graph Aggregator:

| Metric | Source | Target |
|--------|--------|--------|
| Capture rate (matched / total) | Edge extraction confidence > threshold | 70-80% Q1 → 85-90% Q3 |
| Back-channel ratio (unmatched / total) | Edge extractions below threshold | 20-30% Q1, declining |
| Schema-vs-open agreement | Hub open-extraction vs edge schema-guided | 60-70% Q1 → 80%+ Q3 |
| Human-validation agreement | Human review vs edge extraction | Calibration baseline |
| Per-edge-type extraction rate | Count per cultural edge type | Identify dormant/dominant types |
| Per-site extraction volume | Count per site | Identify engagement gaps |
| Triad drift | Mean triad coordinates over time per site | Cultural trajectory signal |

---

## 8. Open Questions

1. ~~**Narrative collection mechanism**~~ — **ANSWERED.** Two sources: (a) monthly Excel dumps from cloud reporting tool, (b) randomised sampling survey with AI-assisted facilitation. See Section 6.

2. **Entity resolution** — When two narratives from different sites both mention "Shift Supervisor," are these the same node or different nodes? The 5P model says nodes are teams/roles (not individuals), but cross-site entity resolution for common role names needs a strategy. Likely: scope nodes to site (Site-42/Shift-Supervisor ≠ Site-17/Shift-Supervisor) and let the hub aggregate by role archetype for cross-site analysis.

3. **Graph storage** — The Arrow schemas above are tabular. For graph traversal queries (e.g., "show all paths from Team Alpha to Plant Manager through cultural edges"), should the hub store the graph in a dedicated graph format (e.g., Kuzudb, which has Arrow integration), or is DuckDB with recursive CTEs sufficient at the expected scale?

4. **Triad training data** — Entity/edge extraction can bootstrap from Open IE. Triad positioning requires humans to place narratives on 6 triads. How many labelled narratives are needed for a viable triad classifier? Likely 500-1000 minimum for 6 x 3-class outputs. The Culture Survey itself could generate this training data if early rounds include a "how would you characterise this situation?" self-assessment step alongside the narrative.

5. **Edge type granularity** — Some cultural edge types may be too similar for a 1-3B SLM to distinguish reliably (e.g., `cooperates-with` vs `shares-information-with`). The parallel open-extraction comparison will surface this, but it may drive initial edge type consolidation before the first training run.

6. **Temporal dynamics** — The schema captures snapshots. How do you track *change* in cultural edges over time? A `works-around` edge observed in Q1 but absent in Q3 is a signal. The Arrow schema needs a temporal dimension (extraction_period or similar) for trajectory analysis.

7. **Excel column variance** — Different organisations (or even different sites within the same organisation) may use different reporting tools with different Excel formats. How much configuration flexibility does the Reporting Data Extractor need? A column-mapping config file per source tool is minimum. Could the SLM itself help with column identification ("which column contains the narrative text?") during initial setup?

8. **Survey deployment form factor** — The Culture Survey app needs to be accessible to frontline workers who may not have company laptops. Options: (a) WASM app running in a browser on a shared tablet/kiosk at the site, (b) native app on company devices, (c) progressive web app accessible on personal phones. Each has different implications for local persistence and model loading. A browser-based WASM deployment (option a) is most aligned with the existing architecture — Wasmtime compiles to WASM that runs in browsers via wasm-bindgen, and ONNX models can run via ONNX Runtime Web.

9. **Facilitation model fine-tuning objective** — The dual-role SLM (facilitate + extract) needs a training objective that balances both. Over-optimising for extraction quality may produce interrogative follow-ups. Over-optimising for respondent experience may produce feel-good questions that don't elicit graph signal. The training data needs to include human-rated examples of good facilitation, not just extraction accuracy.

10. **Survey ethics and consent** — Randomised sampling of workplace narratives about safety culture, power dynamics, and workarounds is sensitive. Even with on-device processing and anonymised signal, respondents need to understand and consent to what the AI is doing with their words. The survey needs a clear consent screen, an explanation of what leaves the device (structured signal only) and what doesn't (their words), and an opt-out mechanism. This is a design requirement, not just a legal one — trust in the survey instrument affects narrative quality.

---

## 9. Sequencing Recommendation

| Phase | Work | Depends On |
|-------|------|------------|
| **Now** | Finalise cultural graph schema (Deliverable 1). Run emergence pass on 200-300 existing narratives from the reporting tool Excel dumps (Deliverable 2). | Schema document (done), access to Excel dumps |
| **Q3 2026** | Hub large-model extraction on seed narratives. Human review to produce training set. Define Arrow schemas. Design question bank for Culture Survey (50-100 questions tagged with target edge types). | Emergence pass results |
| **Q4 2026** | First SLM fine-tune (RunPod). Build **Reporting Data Extractor** as the simpler first app — batch Excel processing, no conversational component. Deploy to safety team. | Training data, Phase 3 host runtime |
| **Q1 2027** | First quarterly validation cycle on reporting tool data. Back-channel clustering. Begin **Culture Survey** app development — the conversational facilitation component is the harder problem, benefits from having extraction model validated first. Pilot survey at one site. | Validated extraction model, question bank |
| **Q2 2027** | Culture Survey pilot results. Tune facilitation quality. Graph Aggregator dashboard comparing reporting-tool vs survey signal. Multi-site rollout of both apps. Provision bridge to DRRP data. | Pilot data from both sources |

**Build the Reporting Data Extractor first.** It's a batch processor — simpler runtime, no conversational component, no real-time model inference during user interaction. It validates the core extraction pipeline (SLM + triad classifier + back-channel + hub sync) on known data. Once that works, the Culture Survey adds the conversational facilitation layer on top of the same extraction stack. Don't attempt both simultaneously.

This slots into the fractalaw roadmap after Phase 3 (micro-app runtime) and overlaps with Phase 4 (distribution/sync), which is when the hub-edge data flow becomes operational.
