use arrow::array::Array;

use crate::open_duck;
use crate::utils::*;
use crate::ZenohArgs;

pub(crate) async fn cmd_sync_pull(data_dir: &std::path::Path, url: &str) -> anyhow::Result<()> {
    let duck = open_duck(data_dir)?;
    duck.create_drrp_tables()?;

    // Determine the `since` timestamp from the last sync.
    let since = duck.get_last_sync_at()?;
    if let Some(ref ts) = since {
        eprintln!("Last sync: {ts}");
    } else {
        eprintln!("First sync — pulling all annotations");
    }

    let client = fractalaw_sync::SyncClient::new(url.to_string());
    let since_dt = since
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let annotations = client.pull_annotations(since_dt).await?;

    if annotations.is_empty() {
        println!("No new annotations.");
        return Ok(());
    }

    let count = duck.insert_annotations(&annotations)?;
    println!("Pulled and stored {count} annotations.");
    Ok(())
}

pub(crate) async fn cmd_sync_push(data_dir: &std::path::Path, url: &str) -> anyhow::Result<()> {
    let duck = open_duck(data_dir)?;
    duck.create_drrp_tables()?;

    let entries = duck.get_unpushed_polished()?;
    if entries.is_empty() {
        println!("Nothing to push.");
        return Ok(());
    }

    let client = fractalaw_sync::SyncClient::new(url.to_string());
    let accepted = client.push_polished(&entries).await?;

    // Mark each entry as pushed.
    for entry in &entries {
        duck.mark_pushed(&entry.law_name, &entry.provision)?;
    }

    println!(
        "Pushed {} polished entries ({} accepted by server).",
        entries.len(),
        accepted
    );
    Ok(())
}

pub(crate) async fn cmd_sync_publish(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    laws: Option<String>,
    family: Option<String>,
    all: bool,
    changed: bool,
) -> anyhow::Result<()> {
    let store = open_duck(data_dir)?;
    store.ensure_taxa_hash_columns()?;
    store.ensure_fitness_columns()?;

    // Resolve law names: --family, --laws, --changed, or --all (must be explicit).
    let law_names: Vec<String> = if let Some(ref fam) = family {
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws", fam, names.len());
        names
    } else if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if changed {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE taxa_hash IS NOT NULL \
               AND (published_hash IS NULL OR taxa_hash != published_hash) \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing {} laws with changed taxa (--changed)",
            names.len()
        );
        names
    } else if all {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE duty_holder IS NOT NULL AND len(duty_holder) > 0 \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!("Publishing ALL {} laws with taxa data", names.len());
        names
    } else {
        anyhow::bail!(
            "Specify --family, --laws, --changed, or --all to select laws to publish.\n\
             Example: fractalaw sync publish --family \"OH&S: Occupational / Personal Safety\" --tenant dev"
        );
    };

    if law_names.is_empty() {
        println!("No laws with taxa data to publish.");
        return Ok(());
    }

    println!(
        "Publishing taxa for {} laws to zenoh (tenant: {})...",
        law_names.len(),
        zenoh.tenant,
    );

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    // Wait for at least one peer to connect before publishing.
    // Peer-mode scouting and inbound connections need time to establish.
    print!("Waiting for zenoh peer...");
    let peers = sync
        .wait_for_peers(std::time::Duration::from_secs(15))
        .await;
    if peers == 0 {
        println!(" no peers connected (timeout). Publishing anyway, but data may not be received.");
    } else {
        println!(" {peers} peer(s) connected.");
    }

    // Put actor dictionary — fires any active sertantai subscribers
    if let Ok(dict_yaml) = std::fs::read("crates/fractalaw-core/data/actor-dictionary.yaml") {
        sync.publish_dictionary(&dict_yaml)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish actor dictionary: {e}"))?;
        println!("Published actor dictionary ({} bytes)", dict_yaml.len());
    }

    let mut published = 0usize;
    for law_name in &law_names {
        let sql = format!(
            "SELECT name, duty_holder, rights_holder, responsibility_holder, power_holder, \
                    duty_type, role, role_gvt, \
                    duties, rights, responsibilities, powers, \
                    fitness_person, fitness_process, fitness_place, \
                    fitness_plant, fitness_property, fitness_sector, fitness, \
                    significance_rating, significance_score, \
                    significance_high_count, significance_medium_count, \
                    significance_low_count, significance_total_obligations, \
                    significance_parts \
             FROM legislation WHERE name = '{}'",
            law_name.replace('\'', "''")
        );
        let batches = store.query_arrow(&sql)?;
        if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
            eprintln!("  {law_name}: no data, skipping");
            continue;
        }

        sync.publish_taxa(law_name, &batches)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish {law_name}: {e}"))?;

        // Mark published_hash = taxa_hash so --changed skips this law next time.
        store.execute(&format!(
            "UPDATE legislation SET published_hash = taxa_hash WHERE name = '{}'",
            law_name.replace('\'', "''")
        ))?;

        published += 1;
    }

    println!("Published {published}/{} laws.", law_names.len());
    Ok(())
}

pub(crate) async fn cmd_sync_publish_provisions(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    laws: Option<String>,
    family: Option<String>,
    all: bool,
    changed: bool,
    pending: bool,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    let store = open_duck(data_dir)?;
    store.ensure_taxa_hash_columns()?;
    store.ensure_provisions_published_column()?;

    let lance = crate::open_provision_store(data_dir, pg_url).await?;

    // Resolve law names.
    let law_names: Vec<String> = if pending {
        store.ensure_enrichment_queue_columns()?;
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE enrichment_pending = false \
               AND enrichment_added_at IS NOT NULL \
               AND (provisions_published_at IS NULL \
                    OR provisions_published_at < updated_at) \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing provisions for {} recently enriched laws (--pending)",
            names.len()
        );
        names
    } else if let Some(ref fam) = family {
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws", fam, names.len());
        names
    } else if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if changed {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE taxa_hash IS NOT NULL \
               AND (provisions_published_at IS NULL \
                    OR provisions_published_at < updated_at) \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing provisions for {} laws with changed taxa (--changed)",
            names.len()
        );
        names
    } else if all {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE duty_holder IS NOT NULL AND len(duty_holder) > 0 \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing provisions for ALL {} laws with taxa data",
            names.len()
        );
        names
    } else {
        anyhow::bail!(
            "Specify --family, --laws, --changed, or --all to select laws to publish.\n\
             Example: fractalaw sync publish --provisions --family \"OH&S: Occupational / Personal Safety\" --tenant dev"
        );
    };

    if law_names.is_empty() {
        println!("No laws with taxa data to publish.");
        return Ok(());
    }

    println!(
        "Publishing provision-level taxa for {} laws to zenoh (tenant: {})...",
        law_names.len(),
        zenoh.tenant,
    );

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    print!("Waiting for zenoh peer...");
    let peers = sync
        .wait_for_peers(std::time::Duration::from_secs(15))
        .await;
    if peers == 0 {
        println!(" no peers connected (timeout). Publishing anyway, but data may not be received.");
    } else {
        println!(" {peers} peer(s) connected.");
    }

    // Put actor dictionary — fires any active sertantai subscribers
    if let Ok(dict_yaml) = std::fs::read("crates/fractalaw-core/data/actor-dictionary.yaml") {
        sync.publish_dictionary(&dict_yaml)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish actor dictionary: {e}"))?;
        println!("Published actor dictionary ({} bytes)", dict_yaml.len());
    }

    let mut published = 0usize;
    let mut total_provisions = 0usize;
    for law_name in &law_names {
        let batches = lance.query_provision_taxa(law_name).await
            .map_err(|e| anyhow::anyhow!("query provisions for {law_name}: {e}"))?;
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        if rows == 0 {
            eprintln!("  {law_name}: no enriched provisions, skipping");
            continue;
        }

        sync.publish_provision_taxa(law_name, &batches)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish provisions for {law_name}: {e}"))?;

        // Mark provisions as published.
        store.execute(&format!(
            "UPDATE legislation SET provisions_published_at = CURRENT_TIMESTAMP WHERE name = '{}'",
            law_name.replace('\'', "''")
        ))?;

        println!("  {law_name}: {rows} provisions");
        total_provisions += rows;
        published += 1;
    }

    println!(
        "Published {total_provisions} provisions across {published}/{} laws.",
        law_names.len()
    );
    Ok(())
}

pub(crate) async fn cmd_sync_pull_lat(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    law_names: &[String],
    timeout_secs: u64,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    let lance = crate::open_provision_store(data_dir, pg_url).await?;

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    let timeout = std::time::Duration::from_secs(timeout_secs);

    println!(
        "Pulling LAT for {} laws from sertantai (tenant: {}, timeout: {timeout_secs}s)...",
        law_names.len(),
        zenoh.tenant,
    );

    let mut total_pulled = 0usize;
    let mut total_rows = 0usize;

    for law_name in law_names {
        eprint!("  {law_name}: ");

        let batches = sync
            .query_lat(law_name, timeout)
            .await
            .map_err(|e| anyhow::anyhow!("query failed for {law_name}: {e}"))?;

        if batches.is_empty() {
            eprintln!("no data (sertantai did not respond)");
            continue;
        }

        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        lance
            .upsert_lat(batches)
            .await
            .map_err(|e| anyhow::anyhow!("upsert failed for {law_name}: {e}"))?;

        eprintln!("{rows} provisions");
        total_pulled += 1;
        total_rows += rows;
    }

    println!(
        "\nPulled {total_pulled}/{} laws, {total_rows} total provisions.",
        law_names.len()
    );

    Ok(())
}

pub(crate) async fn cmd_sync_watch(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    timeout_secs: u64,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    let lance = crate::open_provision_store(data_dir, pg_url).await?;
    let duck = open_duck(data_dir)?;
    duck.ensure_taxa_hash_columns()?;
    duck.ensure_provisions_published_column()?;
    duck.ensure_enrichment_queue_columns()?;

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    let subscriber = sync
        .subscribe_events()
        .await
        .map_err(|e| anyhow::anyhow!("failed to subscribe to events: {e}"))?;

    let timeout = std::time::Duration::from_secs(timeout_secs);

    // Serve actor dictionary as a queryable (stays alive for the watch session)
    let _dict_handle = if let Ok(dict_yaml) = std::fs::read("crates/fractalaw-core/data/actor-dictionary.yaml") {
        println!(
            "Serving actor dictionary via queryable ({} bytes)",
            dict_yaml.len()
        );
        Some(
            sync.serve_dictionary(dict_yaml)
                .await
                .map_err(|e| anyhow::anyhow!("failed to serve actor dictionary: {e}"))?,
        )
    } else {
        eprintln!(
            "Warning: crates/fractalaw-core/data/actor-dictionary.yaml not found, dictionary queryable not started"
        );
        None
    };

    // Serve pipeline status as a queryable
    let status_key = fractalaw_sync::zenoh_sync::keys::status(sync.tenant());
    let status_queryable = sync
        .session()
        .declare_queryable(&status_key)
        .await
        .map_err(|e| anyhow::anyhow!("failed to declare status queryable: {e}"))?;
    println!("Serving pipeline status via queryable ({status_key})");

    duck.ensure_pipeline_status_columns()?;

    println!(
        "Watching for sync events (tenant: {}, timeout: {timeout_secs}s per pull)...",
        zenoh.tenant
    );
    println!("Pipeline: ensure LRT → pull LAT → ack → queue for enrichment");
    println!("Press Ctrl+C to stop.\n");

    let mut total_events = 0usize;
    let mut total_lrt_pulls = 0usize;
    let mut total_lat_pulls = 0usize;
    let mut total_rows = 0usize;
    let mut total_enriched = 0usize;
    let mut total_deletions = 0usize;

    loop {
        tokio::select! {
            sample = subscriber.recv_async() => {
                let sample = match sample {
                    Ok(s) => s,
                    Err(_) => break,
                };

                let bytes = sample.payload().to_bytes();
                let event = match fractalaw_sync::SyncEvent::from_payload(&bytes) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("  [warn] failed to parse event: {e}");
                        continue;
                    }
                };

                total_events += 1;
                let law_name = &event.metadata.law_name;

                // Skip events we don't act on (e.g. amendments).
                if event.table != "lat" && event.table != "lrt" {
                    eprintln!(
                        "  [skip] {}.{} for {}",
                        event.table, event.action, law_name
                    );
                    continue;
                }

                // Handle LAT deletion — purge local text + annotations, keep taxa/fitness.
                if event.action == "lat_deleted" {
                    eprint!("  {law_name}: lat_deleted");
                    let lat_count = lance.delete_law_lat(law_name).await.unwrap_or(0);
                    let ann_count = lance.delete_law_annotations(law_name).await.unwrap_or(0);
                    eprintln!(" → deleted {lat_count} provisions, {ann_count} annotations");
                    total_deletions += 1;
                    continue;
                }

                eprint!("  {law_name}:");

                // Step 1: Ensure LRT exists in DuckDB.
                let lrt_exists = duck
                    .query_arrow(&format!(
                        "SELECT 1 FROM legislation WHERE name = '{}'",
                        law_name.replace('\'', "''")
                    ))
                    .map(|b| b.iter().any(|b| b.num_rows() > 0))
                    .unwrap_or(false);

                if !lrt_exists {
                    eprint!(" pull LRT");
                    match sync.query_lrt(law_name, timeout).await {
                        Ok(batches) if batches.is_empty() => {
                            eprintln!(" → no LRT data");
                        }
                        Ok(batches) => match duck.upsert_legislation(&batches) {
                            Ok(n) => {
                                eprint!(" → {n} row(s)");
                                total_lrt_pulls += 1;
                            }
                            Err(e) => {
                                eprintln!(" → LRT upsert error: {e}");
                            }
                        },
                        Err(e) => {
                            eprintln!(" → LRT query error: {e}");
                        }
                    }
                }

                // Step 2: Pull LAT from sertantai → LanceDB.
                eprint!(" → pull LAT");
                let lat_rows: usize = match sync.query_lat(law_name, timeout).await {
                    Ok(batches) if batches.is_empty() => {
                        eprintln!(" → no LAT data");
                        continue;
                    }
                    Ok(batches) => {
                        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                        match lance.upsert_lat(batches).await {
                            Ok(_) => {
                                eprint!(" → {rows} provisions");
                                total_lat_pulls += 1;
                                total_rows += rows;
                                rows
                            }
                            Err(e) => {
                                eprintln!(" → LAT upsert error: {e}");
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(" → LAT query error: {e}");
                        continue;
                    }
                };

                // Step 3: Ack to sertantai + queue for enrichment.
                //
                // No enrichment or publish here — that happens in batch via
                // `taxa enrich --pending` and `sync publish --pending`.
                if let Err(e) = sync.publish_ack(law_name, lat_rows).await {
                    eprint!(" → ack error: {e}");
                }

                // Mark law as pending enrichment in DuckDB.
                let escaped = law_name.replace('\'', "''");
                let _ = duck.execute(&format!(
                    "UPDATE legislation \
                     SET enrichment_pending = true, \
                         enrichment_added_at = CURRENT_TIMESTAMP \
                     WHERE name = '{escaped}'"
                ));
                total_enriched += 1;
                eprintln!(" → queued for enrichment");
            }
            query = status_queryable.recv_async() => {
                let query = match query {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                let reply_key = query.key_expr().as_str().to_string();

                // Parse law names from:
                // 1. Selector params: ?laws=UK_ukpga_1974_37,UK_uksi_1999_3242
                // 2. Payload body: JSON array ["UK_ukpga_1974_37", ...]
                let law_names: Vec<String> = {
                    let mut names = Vec::new();

                    // Try selector params first (?laws=...)
                    let selector = query.selector();
                    let params = selector.parameters().as_str();
                    for param in params.split('&') {
                        if let Some(val) = param.strip_prefix("laws=") {
                            let decoded = val.replace("%2C", ",").replace("%20", " ");
                            names.extend(
                                decoded.split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty()),
                            );
                        }
                    }

                    // Fallback to payload body (JSON array)
                    if names.is_empty() {
                        if let Some(p) = query.payload() {
                            if let Ok(parsed) = serde_json::from_slice::<Vec<String>>(&p.to_bytes()) {
                                names = parsed;
                            }
                        }
                    }

                    names
                };

                // Query DuckDB for status
                let response = build_status_response(&duck, &law_names);
                let payload = serde_json::to_string(&response).unwrap_or_else(|_| "[]".to_string());

                if let Err(e) = query.reply(&reply_key, payload.into_bytes()).await {
                    tracing::warn!(error = %e, "failed to reply to status query");
                } else {
                    eprintln!("  [status] replied with {} law statuses", response.len());
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
        }
    }

    println!(
        "Done. {total_events} events, {total_lrt_pulls} LRT pulls, \
         {total_lat_pulls} LAT pulls ({total_rows} provisions), \
         {total_enriched} queued for enrichment, {total_deletions} deletions."
    );

    Ok(())
}

// ── CRDT commands ──

pub(crate) fn crdt_persist_dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("crdt")
}

pub(crate) fn generate_peer_id() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::process::id().hash(&mut hasher);
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        hostname.hash(&mut hasher);
    }
    hasher.finish()
}

pub(crate) async fn cmd_crdt_status(data_dir: &std::path::Path, zenoh: &ZenohArgs) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    let persisted = sync
        .list_persisted_docs()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Tenant: {}", zenoh.tenant);
    println!("Persist dir: {}", persist_dir.display());
    println!("Persisted documents: {}", persisted.len());
    for doc_id in &persisted {
        let path = persist_dir.join(format!("{doc_id}.loro"));
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("  {doc_id} ({size} bytes)");
    }
    Ok(())
}

pub(crate) async fn cmd_crdt_create(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    doc_id: &str,
) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    sync.create_doc(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    sync.save_snapshot(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Created and saved CRDT document: {doc_id}");
    Ok(())
}

pub(crate) async fn cmd_crdt_inspect(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    doc_id: &str,
) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    sync.open_or_create(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let value = sync
        .get_doc_value(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{value:?}");
    Ok(())
}

pub(crate) async fn cmd_crdt_save(data_dir: &std::path::Path, zenoh: &ZenohArgs) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    // Load all persisted docs first
    let doc_ids = sync
        .list_persisted_docs()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for doc_id in &doc_ids {
        sync.open_or_create(doc_id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    let paths = sync
        .save_all_snapshots()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Saved {} document(s).", paths.len());
    for p in &paths {
        println!("  {}", p.display());
    }
    Ok(())
}

/// Build JSON status response for a list of law names from DuckDB.
fn build_status_response(
    duck: &fractalaw_store::DuckStore,
    law_names: &[String],
) -> Vec<serde_json::Value> {
    let where_clause = if law_names.is_empty() {
        return Vec::new();
    } else {
        let list = law_names
            .iter()
            .map(|n| format!("'{}'", n.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("WHERE name IN ({list})")
    };

    let sql = format!(
        "SELECT name, lat_pulled_at, embedded_at, parsed_at, classified_at, \
         validated_at, adjudicated_at, provisions_published_at, \
         taxa_hash, published_hash \
         FROM legislation {where_clause}"
    );

    let batches = match duck.query_arrow(&sql) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "status query failed");
            return Vec::new();
        }
    };

    let mut results = Vec::new();
    for batch in &batches {
        let name_col = batch.column_by_name("name");
        let lat_col = batch.column_by_name("lat_pulled_at");
        let emb_col = batch.column_by_name("embedded_at");
        let parse_col = batch.column_by_name("parsed_at");
        let cls_col = batch.column_by_name("classified_at");
        let val_col = batch.column_by_name("validated_at");
        let adj_col = batch.column_by_name("adjudicated_at");
        let pub_at_col = batch.column_by_name("provisions_published_at");
        let taxa_col = batch.column_by_name("taxa_hash");
        let pub_col = batch.column_by_name("published_hash");

        for row in 0..batch.num_rows() {
            let name = name_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let has_lat = lat_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_emb = emb_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_parse = parse_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_cls = cls_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_val = val_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let taxa_hash = taxa_col.and_then(|c| get_string_value(c.as_ref(), row));
            let pub_hash = pub_col.and_then(|c| get_string_value(c.as_ref(), row));
            let is_published = taxa_hash.is_some() && pub_hash.is_some() && taxa_hash == pub_hash;

            let stage = if is_published {
                "published"
            } else if has_val {
                "ready_to_publish"
            } else if has_cls {
                "needs_validate"
            } else if has_parse {
                "needs_classify"
            } else if has_emb {
                "needs_parse"
            } else if has_lat {
                "needs_embed"
            } else {
                "needs_lat"
            };

            let ts = |col: Option<&std::sync::Arc<dyn arrow::array::Array>>, r: usize| -> serde_json::Value {
                if let Some(c) = col {
                    if !c.is_null(r) {
                        return get_string_value(c.as_ref(), r)
                            .map(serde_json::Value::String)
                            .unwrap_or(serde_json::Value::Null);
                    }
                }
                serde_json::Value::Null
            };

            results.push(serde_json::json!({
                "law_name": name,
                "stage": stage,
                "lat_pulled_at": ts(lat_col, row),
                "embedded_at": ts(emb_col, row),
                "parsed_at": ts(parse_col, row),
                "classified_at": ts(cls_col, row),
                "validated_at": ts(val_col, row),
                "adjudicated_at": ts(adj_col, row),
                "published_at": ts(pub_at_col, row),
            }));
        }
    }

    // Add missing laws as needs_lat
    for name in law_names {
        if !results.iter().any(|r| r["law_name"] == name.as_str()) {
            results.push(serde_json::json!({
                "law_name": name,
                "stage": "needs_lat",
            }));
        }
    }

    results
}
