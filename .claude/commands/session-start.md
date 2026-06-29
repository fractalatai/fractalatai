Open a new session or resume a PENDING/SUSPENDED session.

## New session

1. **Agree the session title and scope** with the user. The title should be short and descriptive — "Reconciliation Engine", "Dependency Parsing Features", "Local SLM Tier". Not a task description.

2. **Check for related sessions**: Read existing session files in `.claude/sessions/cascade/` to understand what's already been done. Look for:
   - PENDING sessions on the same topic (resume instead of creating new)
   - CLOSED sessions that this work depends on (note in Dependencies)
   - Avoid duplicating work already captured elsewhere

3. **Create the session file** at `.claude/sessions/cascade/MM-DD-YY-<slug>.md` using today's date and a kebab-case slug from the title. Use the Write tool.

4. **Structure the document** following this template:

```markdown
# Session: <Title> (ACTIVE)

## Problem

1-3 sentences. What's broken, missing, or needed? Why now? Include a concrete example if possible.

## Work

Numbered checklist of items to complete. Use ⬜ for pending:
1. ⬜ First item
2. ⬜ Second item

## Dependencies

- ✅ or ⬜ for each prerequisite — what must exist before this work can proceed
```

5. **Keep it lean at creation**. The session doc grows during the session as decisions, results, and findings are added. Don't front-load with speculative sections. The following sections get **added as work progresses**, not at creation:
   - Architecture decisions (when a choice is made)
   - Results / metrics (when measured)
   - Gemini review feedback (when requested)
   - Comparison tables (when data exists)

## Resume a PENDING or SUSPENDED session

1. **Read the existing session file** thoroughly — understand what was done, what's outstanding, what's deferred.
2. **Change the status** in the heading from `(PENDING)` or `(SUSPENDED)` to `(ACTIVE)`.
3. **Review the work items** — confirm which are still relevant. Remove or update stale items.
4. **Check dependencies** — have they been resolved since the session was paused?
5. **Brief the user** on the current state: what's done, what's next, any blockers resolved.

## Conventions

- **File naming**: `MM-DD-YY-<slug>.md` — date is when the session opened, not when work started
- **Statuses**: `ACTIVE` (current work), `PENDING` (scoped but not started), `SUSPENDED` (started, paused with reason), `CLOSED` (done, has YAML frontmatter)
- **Work items**: `⬜` pending, `✅` done, `⏸️` deferred, `❌` abandoned
- **One session = one coherent piece of work**. If scope expands, split into a new session and link via depends_on/enables
- **Session docs are living documents** — update them as work progresses, don't batch updates at the end
- **Tick off items as you complete them**, not in bulk at session close

## What NOT to put in session docs

- Raw command output or logs (summarise the finding)
- Code snippets longer than 10 lines (reference the file path instead)
- Speculative future work beyond the current scope (create a PENDING session for that)
- Duplicate content from CLAUDE.md or skills
