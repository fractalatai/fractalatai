# Feedback folder: rules for agents

Read [README.md](README.md) for the approach. These rules apply to any work on feedback items.

- **No verbatim client text in this folder.** The repository is public. Quote feedback only from the gitignored file in `docs/sif/research/<source>-feedback/`, and only in your replies, never in tracked files. Paraphrase neutrally in `register.md`.
- **No personal names.** Name sources by role ("QQ colleague"). Use "QQ", never the client's full name.
- **No GitHub issues for feedback.** They would be public.
- **IDs are permanent.** Never renumber or reuse one. Split a point by adding a new ID and noting it in the old item's `response`.
- **Every change links back.** A commit an item drives ends with `Feedback: <ID>[, <ID>]`. Add its hash to the item's `changes` only after the commit has returned it.
- **Keep the status current.** Update `status` and `response` in the same commit as the change.
- **Decisions are Jason's.** For `design` items, present the options and a recommendation; do not set `accepted` or `declined` without his decision. The AI model is Spongl Ltd's, so there is no panel review of model values.
- **Re-index after edits:** `/usr/bin/python3 scripts/maintenance/session_index.py --root /var/home/jason/fractalaw`.
