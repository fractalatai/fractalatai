-- legislation_text table for pgvector (matches actual LanceDB schema)
CREATE TABLE IF NOT EXISTS legislation_text (
    law_name        TEXT NOT NULL,
    section_id      TEXT PRIMARY KEY,
    sort_key        TEXT,
    position        INTEGER,
    section_type    TEXT,
    hierarchy_path  TEXT,
    depth           INTEGER DEFAULT 0,
    part            TEXT,
    chapter         TEXT,
    heading_group   TEXT,
    provision       TEXT,
    paragraph       TEXT,
    sub_paragraph   TEXT,
    schedule        TEXT,
    text            TEXT,
    language        TEXT,
    extent_code     TEXT,
    amendment_count     INTEGER DEFAULT 0,
    modification_count  INTEGER DEFAULT 0,
    commencement_count  INTEGER DEFAULT 0,
    extent_count        INTEGER DEFAULT 0,
    editorial_count     INTEGER DEFAULT 0,
    embedding       vector(384),
    embedding_model TEXT,
    embedded_at     TIMESTAMPTZ,
    token_ids       INTEGER[],
    tokenizer_model TEXT,
    legacy_id       TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    updated_at      TIMESTAMPTZ DEFAULT NOW(),
    drrp_types          TEXT[],
    governed_actors     TEXT[],
    government_actors   TEXT[],
    duty_family         TEXT,
    duty_sub_type       TEXT,
    popimar             TEXT[],
    purposes            TEXT[],
    clause_refined      TEXT,
    taxa_confidence     REAL,
    taxa_classified_at  TIMESTAMPTZ,
    ai_holder       TEXT,
    ai_clause       TEXT,
    ai_qualifier    TEXT,
    ai_clause_ref   TEXT,
    ai_confidence   REAL,
    ai_model        TEXT,
    ai_polished_at  TIMESTAMPTZ,
    fitness_polarity    TEXT[],
    fitness_person      TEXT[],
    fitness_process     TEXT[],
    fitness_place       TEXT[],
    fitness_plant       TEXT[],
    fitness_property    TEXT[],
    fitness_sector      TEXT[],
    scope                TEXT,
    significance_scope_duty_bearer TEXT,
    significance_scope_protected_class TEXT,
    significance_gravity TEXT,
    significance_strength TEXT,
    significance_confidence REAL,
    significance_hierarchy TEXT,
    significance_overall TEXT,
    extraction_method    TEXT,
    ancestor_distance    INTEGER,
    holder_inferred_from TEXT,
    actors              JSONB,
    drrp_history        TEXT,
    drrp_history        TEXT
);

CREATE INDEX IF NOT EXISTS idx_lt_law_name ON legislation_text (law_name);
CREATE INDEX IF NOT EXISTS idx_lt_extraction_method ON legislation_text (extraction_method);
CREATE INDEX IF NOT EXISTS idx_lt_drrp_types ON legislation_text USING GIN (drrp_types);

-- Per-actor classification signals (one row per provision×actor)
CREATE TABLE IF NOT EXISTS provision_actors (
    section_id        TEXT NOT NULL,
    actor_label       TEXT NOT NULL,
    actor_category    TEXT,
    regex_drrp        TEXT,
    regex_position    TEXT,
    cls_drrp          TEXT,
    cls_position      TEXT,
    cls_confidence    REAL,
    slm_drrp          TEXT,
    slm_position      TEXT,
    slm_confidence    REAL,
    llm_drrp          TEXT,
    llm_position      TEXT,
    inferred_drrp     TEXT,
    inferred_position TEXT,
    drrp              TEXT,
    position          TEXT,
    extraction_method TEXT,
    reconcile_confidence TEXT,
    PRIMARY KEY (section_id, actor_label)
);

CREATE INDEX IF NOT EXISTS idx_pa_section ON provision_actors (section_id);

-- Gold benchmark data for QA
CREATE TABLE IF NOT EXISTS gold_benchmarks (
    section_id    TEXT NOT NULL,
    actor_label   TEXT NOT NULL,
    gold_drrp     TEXT,
    gold_position TEXT NOT NULL,
    PRIMARY KEY (section_id, actor_label)
);

-- SLM training labels (definitive prompt, one model, per provision); raw responses, resumable and versioned.
-- Never read by the pipeline tiers; training data only (session parsing/2026-10-01-training-labels-slm.md)
CREATE TABLE IF NOT EXISTS drrp_training_labels_raw (
    section_id     TEXT NOT NULL,
    law_name       TEXT NOT NULL,
    text_md5       TEXT NOT NULL,
    model          TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    split          TEXT,
    stratum        TEXT,
    response       JSONB,
    error          TEXT,
    usage          JSONB,
    dict_version   TEXT NOT NULL DEFAULT 'pre-reconcile',  -- actor dictionary the prompt carried (drrp_dictionary_versions)
    created_at     TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (section_id, text_md5, model, prompt_version, dict_version)
);

-- Actor dictionary versions the labelling prompts carried: label → triggers/patterns, so a run can tell
-- which labels were added since a provision was labelled and relabel only the provisions they affect.
CREATE TABLE IF NOT EXISTS drrp_dictionary_versions (
    dict_version TEXT PRIMARY KEY,
    labels       JSONB NOT NULL,
    created_at   TIMESTAMPTZ DEFAULT now()
);

-- Gold v3 (phase 0a, session parsing/2026-10-06-phase0a-gold-set-scaffolding.md): one row per
-- (provision, field[, actor]), each with the rule IDs that justify it (docs/architecture/DRRP-RULE-CATALOGUE.md),
-- a one-line reason, the tier evidence, a difficulty grade and Jason's decision. Frozen by gold_version.
CREATE TABLE IF NOT EXISTS drrp_gold (
    gold_version   TEXT NOT NULL,             -- e.g. 'gold-v3-draft'; frozen copies get 'gold-v3.0'
    section_id     TEXT NOT NULL,
    text_md5       TEXT NOT NULL,             -- the text the label was made for (a LAT re-pull makes it stale)
    field          TEXT NOT NULL,             -- relation | raw_type | purpose | purpose_fine | actor
    actor_label    TEXT NOT NULL DEFAULT '',  -- field = actor only ('' otherwise)
    actor_position TEXT NOT NULL DEFAULT '',  -- '' for a label's entry; a second entry for the same label in another role carries that position (#78)
    proposed       JSONB NOT NULL,            -- value; actor: {position, holds (Obligation|Liberty|both|none), inferred, act: [..]}
    rule_ids       TEXT[] NOT NULL,           -- catalogue IDs, e.g. {REL-01,HOLD-05}; '{NEW}' = no rule fits
    reason         TEXT NOT NULL,             -- one sentence linking the rule to this text
    evidence       JSONB,                     -- {tier: value} for regex, cls, slm, gemini, mini, referee, adjudicated
    agree          BOOLEAN,                   -- Gemini and referee labels (where they exist) agree (NULL: none); others shown, not counted
    difficulty     TEXT NOT NULL,             -- easy | hard | new_edge
    catalogue_ver  TEXT NOT NULL,             -- git hash of DRRP-RULE-CATALOGUE.md used
    decision       TEXT,                      -- approve | change | query (NULL = not reviewed)
    decided        JSONB,                     -- the corrected value when decision = change
    comment        TEXT,
    decided_at     TIMESTAMPTZ,
    created_at     TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (gold_version, section_id, field, actor_label, actor_position)
);
