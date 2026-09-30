-- Adjudicated actor tier (DRRP-CLASSIFICATION.md: human adjudication is the top
-- source tier and survives reconcile). First use: benchmark gold labels carried
-- forward across a LAT sync (2026-09-30). Additive only.
ALTER TABLE provision_actors ADD COLUMN IF NOT EXISTS adj_drrp TEXT;
ALTER TABLE provision_actors ADD COLUMN IF NOT EXISTS adj_position TEXT;
ALTER TABLE provision_actors ADD COLUMN IF NOT EXISTS adj_note TEXT;
