---
description: Send code, architecture proposals, or session docs to Gemini for critical review. Use when the user asks to "share with Gemini", "get Gemini's feedback", or "run past Gemini".
---

# Gemini Review

## When This Applies

When the user wants a second opinion from Gemini on:
- Architecture proposals or design decisions
- Code review (scripts, Rust modules, SQL schemas)
- Session docs or plans before implementation
- Reconciliation rules, pipeline design, data model changes

## How It Works

Calls the Gemini API directly via curl, passing the content with a review prompt. Returns Gemini's critique.

## Prerequisites

- `GEMINI_API_KEY` must be set. Source it from `~/.bashrc` if not in the current environment:
  ```bash
  source ~/.bashrc
  ```
- The key is stored in `~/.bashrc` as `export GEMINI_API_KEY="..."` — DO NOT hardcode it in scripts or commit it.

## Usage

### Review a file

```bash
export GEMINI_API_KEY="$(grep GEMINI_API_KEY ~/.bashrc | cut -d'"' -f2)"
CONTENT=$(cat <file_path>)

curl -s "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key=$GEMINI_API_KEY" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg content "$CONTENT" --arg prompt "<REVIEW_PROMPT>" '{
    "contents": [{"parts": [{"text": ($prompt + "\n\n" + $content)}]}],
    "generationConfig": {"temperature": 0.1, "maxOutputTokens": 4096, "thinkingConfig": {"thinkingBudget": 4096}}
  }')" | python3 -c "
import sys, json
resp = json.load(sys.stdin)
try:
    for part in resp['candidates'][0]['content']['parts']:
        if 'text' in part:
            print(part['text'])
except (KeyError, IndexError):
    print(json.dumps(resp, indent=2))
"
```

### Review prompts by type

**Architecture / design review:**
```
Be a harsh systems architect. Review this proposal. Focus on:
1. Correctness — any logical errors or edge cases?
2. Missing pieces — what's not covered?
3. Scalability — will this break at 10x scale?
4. Simplicity — is this over-engineered?
Be concise. Bullet points. Max 800 words.
```

**Code review:**
```
Review this code. Only list issues that will CRASH or produce WRONG results.
Skip style nits. Focus on:
1. Bugs and logic errors
2. Edge cases not handled
3. Performance issues
4. API misuse
Bullet points, max 500 words.
```

**Session doc / plan review:**
```
Review this session plan. Focus on:
1. Are the work items in the right order?
2. Are dependencies correctly identified?
3. What's missing that will block progress?
4. Is the scope right — too broad or too narrow?
Be concise. Max 500 words.
```

**Schema / data model review:**
```
Review this database schema change. Focus on:
1. Will queries that join on this table still work?
2. Are there missing indexes?
3. Is the naming consistent with existing columns?
4. Any data migration issues?
Bullet points, max 400 words.
```

## Persisting reviews

**ALWAYS persist the review.** Two outputs:

1. **Raw unedited Gemini response** → `data/code-review/<topic>.md`. No editing, no summarising. The human reads this directly.
2. **Action summary** → append to the active session doc under a "Gemini review feedback (YYYY-MM-DD)" section. This is Claude's interpretation of what's actionable.

```
data/code-review/
├── significance-rating-model.md
├── gemini-cascade-architecture-review.md
├── ...
```

## Guidelines

- **Set thinking budget.** Use `"thinkingBudget": 4096` for complex reviews, `2048` for simple ones. This gives Gemini time to reason before answering.
- **Be specific in the review prompt.** "Review this" produces generic feedback. "Review the reconciliation rules — are the confidence thresholds correct?" produces actionable feedback.
- **Cap output tokens.** `maxOutputTokens: 4096` is usually enough. Longer responses add noise not signal.
- **Temperature 0.1** for reviews — you want consistent, analytical output, not creative.
- **Parse the response.** The Python one-liner extracts the text from Gemini's response format, handling the thinking/response split.
- **Ask Gemini to be harsh.** It defaults to being polite and encouraging. "Be a harsh critic" or "Be brutal" produces more useful feedback.
- **Filter the noise.** Gemini often flags things that are already handled or makes assumptions about the codebase. Read critically — not every critique is valid.

## Model selection

- **gemini-2.5-flash** — default for reviews. Fast, cheap, good enough for architecture and code critique.
- **gemini-2.5-pro** — use for complex multi-file reviews or when flash gives shallow feedback. Replace `gemini-2.5-flash` with `gemini-2.5-pro` in the URL.

## Troubleshooting

- **403 Permission denied**: API key not set or invalid. Check `echo $GEMINI_API_KEY`.
- **Truncated response**: Increase `maxOutputTokens`. Gemini 2.5 Flash is a "thinking" model — without `thinkingConfig`, it may use 400+ tokens thinking and only 100 for actual output. Set `thinkingBudget` to cap thinking and `maxOutputTokens` for headroom.
- **Generic/unhelpful feedback**: Make the review prompt more specific. Include context about what you're worried about.
- **Response too long / noisy**: Add "Max N words" or "Bullet points only" to the prompt.
- **JSON in response has markdown fences**: Gemini sometimes wraps JSON in ` ```json ` fences. Strip with: `text.split("```json")[1].split("```")[0].strip()`.

## Python SDK (for scripts, not this skill)

If calling Gemini from Python scripts (not via this skill's curl approach):
- **Package**: `google-genai` (the new SDK), NOT `google-generativeai` (deprecated)
- **Timeout**: Set `config={"http_options": {"timeout": 30_000}}` (milliseconds)
- The SDK handles thinkingBudget transparently; the REST API needs explicit `generationConfig`
