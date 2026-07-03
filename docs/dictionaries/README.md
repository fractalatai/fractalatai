# Dictionaries

Documentation for the actor and classifier dictionaries used in the DRRP pipeline.

## Sources of Truth

The actual pipeline data lives with the crates that own it — this directory contains **documentation only**.

| What | Location | Format |
|------|----------|--------|
| Actor dictionary (data) | `crates/fractalaw-core/data/actor-dictionary.yaml` | YAML — compiled into binary via `include_str!` |
| Correlative rules (data) | `crates/fractalaw-core/data/correlative-rules.yaml` | YAML — compiled into binary via `include_str!` |
| DRRP classifier weights | `crates/fractalaw-cli/config/drrp_classifier_v8.json` | JSON — loaded at runtime |
| Position classifier weights | `crates/fractalaw-cli/config/position_classifier_v3.json` | JSON — loaded at runtime |

## Training New Classifier Versions

```bash
# DRRP classifier
/usr/bin/python3 scripts/ml/retrain_drrp_classifier.py --output crates/fractalaw-cli/config/drrp_classifier_v9.json

# Position classifier
/usr/bin/python3 scripts/ml/train_position_classifier.py --output crates/fractalaw-cli/config/position_classifier_v4.json
```

After training, update the runtime paths in `crates/fractalaw-cli/src/commands/taxa.rs`.

## Contents

| File | Description |
|------|-------------|
| `ACTOR-DICTIONARY.md` | Human-readable reference for actor labels used across fractalaw and sertantai |
