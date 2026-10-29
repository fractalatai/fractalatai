-- data/sif.duckdb schema. Import file contract: .claude/plans/sif/SIF-IMPORT-SCHEMA.md
-- event_id = ECO_ETL row_sig (Id x 100 + report-type code): one row per report.

CREATE TABLE IF NOT EXISTS events (
    event_id             VARCHAR PRIMARY KEY,   -- row_sig
    case_id              BIGINT,                -- EcoOnline Id (a case can hold several reports)
    legacy_event_id      VARCHAR,               -- pre-2026-10 RowSignatureII hash (August batch only)
    site                 VARCHAR,
    narrative            VARCHAR,
    action               VARCHAR,
    report_type          VARCHAR,               -- reporter's type (RE_ReportType)
    ed_report_type       VARCHAR,               -- triager's type (ED_ReportType)
    fy                   VARCHAR,
    sector               VARCHAR,
    sub_sector           VARCHAR,
    event_date           VARCHAR,               -- ISO YYYY-MM-DD
    hazard_category      VARCHAR,
    event_type_class     VARCHAR,
    -- QQ SIFp labels (benchmark, not training)
    qq_sifp              VARCHAR,
    qq_ed_sifp           VARCHAR,
    qq_t_sifp            VARCHAR,
    qq_i_sifp            VARCHAR,
    qq_i_sifa            VARCHAR,
    ingested_at          TIMESTAMP DEFAULT current_timestamp,
    date_updated         TIMESTAMP,             -- ECO_ETL DateUpdated; drives updates
    source_file          VARCHAR,               -- import file that last wrote the row
    narrative_changed_at TIMESTAMP,             -- set when an update changed narrative/action: model results stale
    vanished_at          TIMESTAMP              -- absent from an import covering its date: merged (ETL-020)
);

CREATE TABLE IF NOT EXISTS classifications (
    event_id          VARCHAR PRIMARY KEY REFERENCES events(event_id),
    mechanism_codes   VARCHAR[],
    mechanism_labels  VARCHAR[],
    object_codes      VARCHAR[],
    object_labels     VARCHAR[],
    sif_gate          VARCHAR,
    stage1_confidence FLOAT,
    energy_types      VARCHAR[],
    source_cues       VARCHAR[],
    carrier_cues      VARCHAR[],
    environment_cues  VARCHAR[],
    body_vulnerability VARCHAR,
    severity_p10      VARCHAR,
    severity_p50      VARCHAR,
    severity_p90      VARCHAR,
    energy_reasoning  VARCHAR,
    stage2_confidence FLOAT,
    p_sif             FLOAT,
    sif_class         VARCHAR,
    review_flag       BOOLEAN,
    classified_at     TIMESTAMP DEFAULT current_timestamp,
    model_version     VARCHAR
);

CREATE TABLE IF NOT EXISTS reviews (
    event_id          VARCHAR REFERENCES events(event_id),
    reviewer          VARCHAR,
    decision          VARCHAR,
    notes             VARCHAR,
    reviewed_at       TIMESTAMP DEFAULT current_timestamp,
    PRIMARY KEY (event_id, reviewer)
);
