#[allow(warnings)]
mod bindings;
mod ipc;

use bindings::fractal::app::{ai_inference, audit_log, data_mutate, data_query};
use bindings::Guest;
use serde::Deserialize;

struct DrrpPolisher;

// ── Prompt templates ──

const SYSTEM_PROMPT: &str = "\
You are a legal provision extractor specialising in UK ESH (environment, safety, health) legislation.

Given a provision's DRRP taxa classification (from regex) and the full provision text, refine the \
extraction with AI precision.

The regex classification is rough — it identifies duty types, actors, and a cleaned clause, but may \
be imprecise. Your job is to produce an accurate, corrected version.

Respond ONLY with a JSON object. No markdown fences, no explanation, just raw JSON:
{
  \"holder\": \"the entity that holds this duty/right/responsibility/power\",
  \"ai_clause\": \"the exact provision text, quoted verbatim from the source\",
  \"qualifier\": \"any qualifying phrase (e.g. 'so far as is reasonably practicable')\" or null,
  \"clause_ref\": \"the specific clause reference (e.g. 's.2(1)')\"
}

Rules:
- Quote the provision text EXACTLY from the source — do not paraphrase
- Include the full clause, not a truncated version
- The holder should be the specific entity (e.g. 'every employer', not just 'Org: Employer')
- The clause_ref should be a standard legal citation (e.g. 's.2(1)', 'reg.3(1)(a)')
- If the regex extraction is correct, return the same values with improved precision
- If you cannot identify a clear provision, set holder to \"unknown\" and ai_clause to the most relevant sentence";

fn build_user_prompt(prov: &ProvisionRow) -> String {
    let mut prompt = format!(
        "Law: {law}\n\
         Provision: {provision}\n",
        law = prov.law_name,
        provision = prov.provision,
    );

    if !prov.drrp_types.is_empty() {
        prompt.push_str(&format!("DRRP types: {}\n", prov.drrp_types.join(", ")));
    }

    if let Some(ref family) = prov.duty_family {
        prompt.push_str(&format!("Duty family: {family}\n"));
    }

    if !prov.governed_actors.is_empty() {
        prompt.push_str(&format!(
            "Governed actors: {}\n",
            prov.governed_actors.join(", ")
        ));
    }
    if !prov.government_actors.is_empty() {
        prompt.push_str(&format!(
            "Government actors: {}\n",
            prov.government_actors.join(", ")
        ));
    }

    if let Some(ref clause) = prov.clause_refined {
        prompt.push_str(&format!("\nRegex-refined clause:\n{clause}\n"));
    }

    prompt.push_str(&format!("\nFull section text:\n{}", prov.text));
    prompt
}

// ── Types ──

/// A provision row from LanceDB with taxa data and source text.
#[derive(Deserialize)]
struct ProvisionRow {
    section_id: String,
    law_name: String,
    provision: String,
    text: String,
    #[serde(default)]
    drrp_types: Vec<String>,
    #[serde(default)]
    governed_actors: Vec<String>,
    #[serde(default)]
    government_actors: Vec<String>,
    #[serde(default)]
    duty_family: Option<String>,
    #[serde(default)]
    clause_refined: Option<String>,
}

/// AI-refined output from the polisher.
#[derive(Deserialize)]
struct PolishedOutput {
    holder: String,
    ai_clause: String,
    qualifier: Option<String>,
    clause_ref: String,
}

// ── Helpers ──

fn audit(event_type: &str, detail: &str) {
    audit_log::record_event(&audit_log::AuditEntry {
        event_type: event_type.to_string(),
        resource: "drrp-polisher".to_string(),
        detail: detail.to_string(),
    });
}

fn query_i64(sql: &str) -> Result<i64, String> {
    let ipc = data_query::query(sql)
        .map_err(|e| format!("query failed: {} (code {})", e.message, e.code))?;
    ipc::extract_i64(&ipc).ok_or_else(|| "failed to parse i64 from IPC result".to_string())
}

fn query_string(sql: &str) -> Result<String, String> {
    let ipc = data_query::query(sql)
        .map_err(|e| format!("query failed: {} (code {})", e.message, e.code))?;
    ipc::extract_string(&ipc).ok_or_else(|| "failed to parse string from IPC result".to_string())
}

fn execute(sql: &str) -> Result<u64, String> {
    data_mutate::execute(sql)
        .map_err(|e| format!("execute failed: {} (code {})", e.message, e.code))
}

fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

// ── Guest entry point ──

impl Guest for DrrpPolisher {
    fn run() -> Result<String, String> {
        audit(
            "app-started",
            "DRRP polisher run starting (LanceDB-only mode)",
        );

        // 1. Count provisions needing polishing (have taxa data but no AI refinement).
        let unpolished_count = query_i64(
            "SELECT COUNT(*) FROM legislation_text
             WHERE drrp_types IS NOT NULL AND ai_clause IS NULL",
        )?;

        if unpolished_count == 0 {
            audit("batch-empty", "No provisions need DRRP polishing");
            return Ok("No provisions need DRRP polishing.".to_string());
        }

        audit(
            "batch-start",
            &format!("{unpolished_count} provisions to polish"),
        );

        // 2. Process provisions in batches.
        let batch_size = 50i64;
        let mut total_polished = 0u64;
        let mut total_errors = 0u64;
        let mut total_tokens = 0u32;

        let batches = (unpolished_count + batch_size - 1) / batch_size;
        for batch_idx in 0..batches {
            // Fetch a batch of unpolished provisions as JSON.
            // The host routes legislation_text queries to LanceDB and
            // wraps each row as a JSON string for our IPC parser.
            let offset = batch_idx * batch_size;
            let limit = batch_size.min(unpolished_count - offset);

            for i in 0..limit {
                let row_json = match query_string(&format!(
                    "SELECT to_json(section_id, law_name, provision, text, drrp_types, \
                     governed_actors, government_actors, duty_family, clause_refined) \
                     FROM legislation_text \
                     WHERE drrp_types IS NOT NULL AND ai_clause IS NULL \
                     LIMIT 1 OFFSET {}",
                    offset + i,
                )) {
                    Ok(s) => s,
                    Err(e) => {
                        audit("query-error", &format!("offset {}: {e}", offset + i));
                        total_errors += 1;
                        continue;
                    }
                };

                let prov: ProvisionRow = match serde_json::from_str(&row_json) {
                    Ok(p) => p,
                    Err(e) => {
                        audit(
                            "parse-error",
                            &format!(
                                "offset {}: {e}\nraw: {}",
                                offset + i,
                                &row_json[..row_json.len().min(200)]
                            ),
                        );
                        total_errors += 1;
                        continue;
                    }
                };

                match process_provision(&prov) {
                    Ok(tokens) => {
                        total_polished += 1;
                        total_tokens += tokens;
                    }
                    Err(e) => {
                        audit("provision-error", &format!("{}: {e}", prov.section_id));
                        total_errors += 1;
                    }
                }
            }
        }

        let summary = format!(
            "Polished {total_polished} provisions \
             ({total_errors} errors, {total_tokens} tokens used)."
        );
        audit("batch-complete", &summary);
        Ok(summary)
    }
}

/// Process a single provision: call AI, write results back to LanceDB.
fn process_provision(prov: &ProvisionRow) -> Result<u32, String> {
    let user_prompt = build_user_prompt(prov);

    // Call AI inference (ONNX local-first, falls through to Claude).
    let response = ai_inference::generate(&ai_inference::GenerateRequest {
        system_prompt: Some(SYSTEM_PROMPT.to_string()),
        user_prompt,
        max_tokens: 1024,
        temperature: 0.0,
    })
    .map_err(|e| format!("inference error: {} (code {})", e.message, e.code))?;

    // Parse the AI response.
    let output: PolishedOutput = serde_json::from_str(&response.text).map_err(|e| {
        format!(
            "parse error: {e}\nraw: {}",
            &response.text[..response.text.len().min(200)]
        )
    })?;

    // Write polished results back to LanceDB via UPDATE.
    let qualifier_sql = match &output.qualifier {
        Some(q) => format!("'{}'", sql_escape(q)),
        None => "NULL".to_string(),
    };

    let model = if response.tokens_used == 0 {
        "onnx"
    } else {
        "claude"
    };

    execute(&format!(
        "UPDATE legislation_text SET \
         ai_holder = '{holder}', \
         ai_clause = '{ai_clause}', \
         ai_qualifier = {qualifier}, \
         ai_clause_ref = '{clause_ref}', \
         ai_confidence = {confidence}, \
         ai_model = '{model}', \
         ai_polished_at = CURRENT_TIMESTAMP \
         WHERE section_id = '{section_id}'",
        holder = sql_escape(&output.holder),
        ai_clause = sql_escape(&output.ai_clause),
        qualifier = qualifier_sql,
        clause_ref = sql_escape(&output.clause_ref),
        confidence = response.confidence,
        model = model,
        section_id = sql_escape(&prov.section_id),
    ))?;

    Ok(response.tokens_used)
}

bindings::export!(DrrpPolisher with_types_in bindings);
