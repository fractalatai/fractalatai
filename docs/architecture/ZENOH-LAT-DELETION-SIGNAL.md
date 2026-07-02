# Zenoh Signal: LAT Deletion

Specification for the `lat_deleted` change notification published by sertantai-legal when LAT data is deleted for a law. The taxa/fitness service (fractalaw) should subscribe to this signal and delete its local copy of LAT and amendment annotation data for the affected law.

## Signal Source

**Endpoint**: `DELETE /api/lat/laws/:law_name/data` (sertantai-legal)

**Trigger**: Admin user clicks "Delete LAT" on a law in the `/admin/lrt` LAT Cleanup view. Typically used for:
- Laws with `live` = `❌ Revoked / Repealed / Abolished` that still carry LAT data
- Laws incorrectly flagged as `is_making` where full-text parse found no making function

## Zenoh Channel

| Property | Value |
|----------|-------|
| Key expression | `fractalaw/@{tenant}/events/sync` |
| Direction | sertantai-legal → fractalaw (push) |
| Encoding | UTF-8 JSON |
| Delivery | At-most-once (Zenoh pub/sub, no persistence) |

## Payload

```json
{
  "table": "lat",
  "action": "lat_deleted",
  "metadata": {
    "law_name": "UK_ukpga_1974_37",
    "lat_deleted": 1500,
    "annotations_deleted": 42
  },
  "timestamp": "2026-03-27T10:30:22Z"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `table` | `string` | Always `"lat"` for this signal |
| `action` | `string` | Always `"lat_deleted"` for this signal |
| `metadata.law_name` | `string` | Canonical law identifier (e.g., `UK_ukpga_1974_37`). Use this to identify which law's data to delete. |
| `metadata.lat_deleted` | `integer` | Number of LAT rows deleted from sertantai-legal. Informational — the receiver should delete ALL local LAT rows for this law regardless of count. |
| `metadata.annotations_deleted` | `integer` | Number of amendment annotation rows deleted. Informational. |
| `timestamp` | `string` (ISO 8601) | When the deletion occurred in sertantai-legal. |

## Expected Receiver Behaviour

When fractalaw receives a notification with `action: "lat_deleted"`:

### 1. Delete local LAT data

Delete all locally-stored LAT sections for the law identified by `metadata.law_name`.

The law_name maps to `lat.law_name` and `amendment_annotations.law_name` in the source schema. Whatever local representation fractalaw uses (database rows, in-memory cache, files), all LAT content for this law should be purged.

### 2. Delete local amendment annotations

Delete all locally-stored amendment annotations (F-codes, C-codes, I-codes, E-codes) for the same `law_name`.

### 3. Do NOT delete taxa/fitness data

The following data is **independent** of LAT and must be preserved:
- DRRP analysis results (duty_holder, power_holder, rights_holder, responsibility_holder)
- Duty type classifications
- POPIMAR classifications
- Role classifications
- Purpose classifications
- Fitness tags (person, process, place, plant, property, sector)
- Fitness detail (article-level applicability)

These are derived from different analysis pipelines and remain valid even after LAT deletion.

### 4. Do NOT re-query

After deletion, the law no longer has LAT data in sertantai-legal. Do not attempt to re-query:
- `fractalaw/@{tenant}/data/legislation/lat/{law_name}` — will return `[]`
- `fractalaw/@{tenant}/data/legislation/amendments/{law_name}` — will return `[]`

The LRT record itself (`/lrt/{law_name}`) still exists with `lat_count: 0`.

## Idempotency

The signal may arrive when fractalaw has no local data for the law (e.g., fractalaw restarted and hasn't cached this law, or a duplicate notification). Receivers should handle this gracefully — deleting zero rows is fine.

## Missed Signals

Zenoh pub/sub is at-most-once with no persistence. If fractalaw is offline when the signal fires, it will not receive it. To handle this:

- On startup or reconnect, fractalaw can query `fractalaw/@{tenant}/data/legislation/lrt` for the full LRT dataset
- Any law with `lat_count: 0` that fractalaw holds local LAT data for should be treated as a deletion
- Alternatively, fractalaw can query LAT per-law on demand and trust the empty response

## Sequence Diagram

```
Admin UI                sertantai-legal              Zenoh mesh            fractalaw
   │                         │                          │                      │
   │  DELETE /api/lat/       │                          │                      │
   │  laws/{name}/data       │                          │                      │
   │────────────────────────>│                          │                      │
   │                         │                          │                      │
   │                         │  BEGIN transaction       │                      │
   │                         │  DELETE amendment_annotations WHERE law_name=X  │
   │                         │  DELETE lat WHERE law_name=X                    │
   │                         │  COMMIT                  │                      │
   │                         │                          │                      │
   │                         │  (PG trigger fires:      │                      │
   │                         │   uk_lrt.lat_count → 0)  │                      │
   │                         │                          │                      │
   │                         │  ChangeNotifier.notify() │                      │
   │                         │─────────────────────────>│                      │
   │                         │                          │  {table: "lat",      │
   │                         │                          │   action: "lat_deleted",
   │                         │                          │   metadata: {...}}   │
   │                         │                          │─────────────────────>│
   │                         │                          │                      │
   │                         │                          │          Delete local LAT
   │                         │                          │          Delete local annotations
   │                         │                          │          Keep taxa/fitness
   │  200 OK                 │                          │                      │
   │<────────────────────────│                          │                      │
```

## Other Change Notification Actions (for context)

The same `events/sync` channel carries other actions. Fractalaw should filter on `action`:

| Action | Table | When | Receiver should |
|--------|-------|------|-----------------|
| `scrape_import` | `uk_lrt` | Batch scrape saves new LRT records | Re-query LRT |
| `csv_enrichment` | `uk_lrt` | CSV data update (function, taxa) | Re-query LRT |
| `parse_complete` | `uk_lrt` | Single law parse finishes | Re-query that law |
| `bulk_update` | `uk_lrt` | Admin bulk field update | Re-query LRT |
| **`lat_deleted`** | **`lat`** | **LAT data deleted for a law** | **Delete local LAT + annotations** |
