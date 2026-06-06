#!/usr/bin/env python3
"""Migrate actors struct in LanceDB to latest schema.

Adds missing fields (label_source, reason) with defaults.
All existing actors get label_source="canonical", reason=None.
"""
import lancedb
import pyarrow as pa
import pyarrow.parquet as pq

ACTORS_TYPE = pa.list_(pa.struct([
    pa.field("label", pa.string(), nullable=False),
    pa.field("role", pa.string(), nullable=False),
    pa.field("recipient_type", pa.string(), nullable=True),
    pa.field("label_source", pa.string(), nullable=False),
    pa.field("reason", pa.string(), nullable=True),
]))

db = lancedb.connect("data/lancedb")
table = db.open_table("legislation_text")
rows = table.count_rows()
print(f"Exporting {rows} rows...")
arrow = table.to_arrow()

actors_idx = arrow.schema.get_field_index("actors")
actors_col = arrow.column(actors_idx)

new_entries = []
for val in actors_col:
    if val is None or not val.is_valid:
        new_entries.append(None)
    else:
        py_val = val.as_py()
        if not py_val:
            new_entries.append(None)
        else:
            new_entries.append([
                {
                    "label": a.get("label", ""),
                    "role": a.get("role", ""),
                    "recipient_type": a.get("recipient_type"),
                    "label_source": a.get("label_source", "canonical"),
                    "reason": a.get("reason"),
                }
                for a in py_val
            ])

new_actors = pa.array(new_entries, type=ACTORS_TYPE)
columns = list(range(arrow.num_columns))
columns.remove(actors_idx)
new_table = arrow.select(columns)
new_table = new_table.add_column(
    actors_idx, pa.field("actors", ACTORS_TYPE, nullable=True), new_actors
)

print(f"New actors type: {new_table.schema.field('actors').type}")

pq.write_table(new_table, "backups/legislation_text_pre_label_source.parquet")
print("Parquet backup saved")

db.drop_table("legislation_text")
rebuilt = db.create_table("legislation_text", data=new_table)
print(f"Rebuilt: {rebuilt.count_rows()} rows")

sample = rebuilt.search().where("actors IS NOT NULL", prefilter=True).limit(3).to_arrow()
for i in range(min(3, len(sample))):
    sid = sample.column("section_id")[i].as_py()
    actors = sample.column("actors")[i].as_py()
    print(f"  {sid}: {actors}")
