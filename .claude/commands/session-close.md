Close the current session and add a YAML frontmatter learning block.

1. **Identify the active session**: The user will specify which session doc to close, or it will be obvious from the current conversation context. Session docs live in `.claude/sessions/` (any subdirectory).

2. **Check for incomplete work**: Read the session file and find any unchecked items (`⬜`). If there are incomplete items:
   - List them clearly
   - Ask the user: "These items are still open. Do you want to: (a) close anyway and defer them, (b) mark them done, or (c) keep the session open?"
   - **DO NOT proceed until the user confirms**

3. **Replace the skeleton frontmatter**: The session file already has a `---` fenced YAML block at the top (created by session-start). **REPLACE** this skeleton block with the full closing frontmatter. Use the Edit tool to replace from the opening `---` through the closing `---` (inclusive). All content below the frontmatter (heading, sections, work items, results) remains untouched. Use this schema:

```yaml
---
session: <session title>
status: closed
opened: <YYYY-MM-DD>
closed: <YYYY-MM-DD>
outcome: <success | partial | failed | deferred>

summary: >
  2-3 sentence summary of what was accomplished and the key result.

decisions:
  - what: <what was decided>
    why: <reasoning — what evidence or constraint drove it>
    result: <outcome — quantitative if possible>

metrics:
  <metric_name>: { <key>: <value>, ... }

lessons:
  - title: <one-line lesson>
    detail: <context and explanation>
    tag: <infrastructure | models | methodology | data | architecture | tooling>

artifacts:
  - <path to file created or modified>

depends_on:
  - <session filename without path>

enables:
  - <what future work this unblocks>
---
```

4. **Populate from the session content**:
   - **decisions**: Extract from architecture decisions, Gemini reviews, or explicit choices documented in the session
   - **metrics**: Extract any accuracy numbers, counts, benchmarks, timings
   - **lessons**: Focus on what was surprising, what failed, what the user corrected, what worked unexpectedly well. These should be useful to a future AI or human encountering the same situation
   - **artifacts**: List scripts, configs, data files created during the session
   - **depends_on / enables**: Trace the session graph from context

5. **Update the session heading**: Change `(ACTIVE)` or `(SUSPENDED)` to `(CLOSED)` in the `# Session:` line.

6. **Mark deferred items**: Any incomplete work items should be changed from `⬜` to `⏸️` with a note like `(deferred — reason)`.

7. **Write directly**: Do not present the draft to the user for review — write the frontmatter straight to the file. The user can review the diff in git if needed.

8. **Rebuild the session index**: After writing the frontmatter, rebuild the SQLite index so the new session is queryable:
   ```bash
   /usr/bin/python3 scripts/maintenance/session_index.py --root /var/home/jason/fractalaw
   ```
   This is idempotent — it drops and recreates all tables from the markdown source. The `--archive` flag can be added to also move sessions closed >30 days ago to `archive/`.

## Guidelines

- **Lessons are the most valuable section.** Capture what would save someone (human or AI) time next time. "Kaggle loses /workspace/ on kernel restart" is a good lesson. "We used LoRA" is not — that's a decision, not a lesson.
- **Be specific in metrics.** `accuracy: 77.4%` not `accuracy: good`.
- **Tag lessons consistently.** Use the tags: infrastructure, models, methodology, data, architecture, tooling.
- **Don't invent lessons.** Only capture what actually happened in the session. If there were no surprises, fewer lessons is fine.
- **Keep summary under 3 sentences.** The detail is in the sections below.
