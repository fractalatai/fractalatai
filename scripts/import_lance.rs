// Quick script to import legislation_text.parquet into LanceDB with extended schema
// Run with: cargo script import_lance.rs

use std::sync::Arc;
use arrow::array::*;
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use fractalaw_core::schema::legislation_text_schema;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read existing Parquet (30 columns)
    let parquet_path = std::path::Path::new("data/legislation_text.parquet");
    let batches = fractalaw_store::read_parquet(parquet_path)?;

    println!("Read {} rows from Parquet", batches.iter().map(|b| b.num_rows()).sum::<usize>());

    // Get the 47-column schema
    let target_schema = legislation_text_schema();

    // Extend each batch with 17 null columns
    let mut extended_batches = Vec::new();
    for batch in batches {
        let num_rows = batch.num_rows();

        // Build 17 new null/empty columns
        let mut new_columns: Vec<Arc<dyn Array>> = vec![];

        // List columns: empty lists
        let item_field = Arc::new(Field::new("item", DataType::Utf8, true));
        for _ in 0..4 { // drrp_types, governed_actors, government_actors, popimar, purposes (actually 5 but we'll do purposes separately)
            let mut builder = ListBuilder::new(StringBuilder::new());
            for _ in 0..num_rows {
                builder.append_value([]);
            }
            new_columns.push(Arc::new(builder.finish()));
        }

        // purposes (5th list column)
        let mut builder = ListBuilder::new(StringBuilder::new());
        for _ in 0..num_rows {
            builder.append_value([]);
        }
        new_columns.push(Arc::new(builder.finish()));

        // Utf8 columns: nulls
        for _ in 0..7 { // duty_family, duty_sub_type, clause_refined, ai_holder, ai_clause, ai_qualifier, ai_clause_ref, ai_model (actually 8)
            let mut builder = StringBuilder::new();
            for _ in 0..num_rows {
                builder.append_null();
            }
            new_columns.push(Arc::new(builder.finish()));
        }

        // ai_model (8th utf8)
        let mut builder = StringBuilder::new();
        for _ in 0..num_rows {
            builder.append_null();
        }
        new_columns.push(Arc::new(builder.finish()));

        // Float32 columns: nulls
        for _ in 0..2 { // taxa_confidence, ai_confidence
            let mut builder = Float32Builder::new();
            for _ in 0..num_rows {
                builder.append_null();
            }
            new_columns.push(Arc::new(builder.finish()));
        }

        // Timestamp columns: nulls
        for _ in 0..2 { // taxa_classified_at, ai_polished_at
            let mut builder = TimestampNanosecondBuilder::new().with_timezone("UTC");
            for _ in 0..num_rows {
                builder.append_null();
            }
            new_columns.push(Arc::new(builder.finish()));
        }

        // Combine existing + new columns
        let mut all_columns: Vec<Arc<dyn Array>> = batch.columns().to_vec();
        all_columns.extend(new_columns);

        let extended = RecordBatch::try_new(target_schema.clone(), all_columns)?;
        extended_batches.push(extended);
    }

    println!("Extended to {} columns", target_schema.fields().len());

    // Write to LanceDB
    let lance = fractalaw_store::LanceStore::open(&std::path::PathBuf::from("data/lancedb")).await?;
    lance.create_table_from_batches("legislation_text", extended_batches).await?;

    println!("✓ Created legislation_text table in LanceDB");

    Ok(())
}
