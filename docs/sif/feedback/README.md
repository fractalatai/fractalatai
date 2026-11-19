# SIF feedback

This folder tracks feedback on the SIF papers (`docs/sif/`) and the AI model, from receipt to response. Each point gets an ID, and each change we make for it is linked back to that ID.

## Where things are

| What | Where | In git? |
|---|---|---|
| Register: one item per point, with status and a one-line response | [register.md](register.md) (YAML frontmatter) | Yes |
| Full response per item: text, tables, references | `responses/<ID>.md` | Yes |
| Compiled response for the source | `response-<source>.md`, built by `scripts/sif/compile_feedback_response.py <source>` | Yes |
| Verbatim feedback, exactly as received | `docs/sif/research/<source>-feedback/<date>-<role>.md` | No: gitignored |
| Links from items to changes | `Feedback:` trailers on commits | Yes |
| Query tables | `.claude/sessions/sessions.db`: `feedback`, `feedback_links` | Rebuilt from the above |

The repository is public. The register therefore holds only neutral paraphrases, with sources named by role (for example "QQ colleague"), never by person. Client wording stays in the gitignored verbatim file. For the same reason, feedback is not raised as GitHub issues.

## Workflow

1. **Receive.** Save the text exactly as it came in a new file under `docs/sif/research/<source>-feedback/`. Its frontmatter records the source role (`from`), the date received (`received`) and what it was about (`about`; not `on`, which YAML reads as true).
2. **Register.** Add a block under `sources:` in `register.md`, and add one item per distinct point. Each item gets:
   - an ID (`<SOURCE>-FB-nn`);
   - a kind;
   - the documents it touches;
   - a paraphrase.
   Then set its status to `open`.
3. **Respond.** Write the full answer in `responses/<ID>.md`, starting with a `## <ID>. <title>` heading, then *Your point* (a paraphrase) and *Response*. Put a one-line summary in `response`, set `response_doc`, and move the status on.
4. **Change.** Every commit an item drives carries a trailer as its last line:
   ```
   Feedback: QQ-FB-03, QQ-FB-04
   ```
   Also list the commit hash under the item's `changes`, so the link survives a history rewrite.
5. **Close.** Set the status to `done`, `answered` or `declined`. When every item has a response, compile the document: `/usr/bin/python3 scripts/sif/compile_feedback_response.py <source>`.
6. **Index.** Run `/usr/bin/python3 scripts/maintenance/session_index.py`. It reloads the register and scans `git log` for trailers.

## Fields

| Field | Values |
|---|---|
| `id` | `<SOURCE>-FB-nn`. Never changed and never reused. |
| `kind` | `confirm`: a question about our intent. `design`: a choice for the model owner. `gap`: something the papers do not cover. |
| `status` | `open`, `accepted` (change agreed, not yet made), `done`, `answered` (no change needed), `declined` (with the reason in `response`) |
| `touches` | Paths of the documents affected |
| `response` | One-line summary, used in the compiled document's summary table |
| `response_doc` | Path to the full response, relative to this folder |
| `changes` | Commit hashes |

Decisions on the AI model's values and design belong to Spongl Ltd. The QQ panel process belongs to QQ.

## Query

```sql
-- What changed for an item
SELECT f.id, f.status, l.commit_hash, l.date, l.subject
FROM feedback f LEFT JOIN feedback_links l ON l.feedback_id = f.id
WHERE f.id = 'QQ-FB-03';

-- Open items by kind
SELECT kind, COUNT(*) FROM feedback WHERE status = 'open' GROUP BY kind;

-- Items with no linked change yet
SELECT f.id, f.kind FROM feedback f
WHERE f.status IN ('accepted', 'done')
  AND NOT EXISTS (SELECT 1 FROM feedback_links l WHERE l.feedback_id = f.id);
```

From the shell: `git log --grep "^Feedback: .*QQ-FB-03"`.
