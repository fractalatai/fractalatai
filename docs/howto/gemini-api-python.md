# How-to: Call Gemini API from Python

## Setup

```bash
pip install google-genai
```

Get an API key from [Google AI Studio](https://aistudio.google.com/apikey).

## Basic call

```python
import os
from google import genai

client = genai.Client(api_key=os.environ["GEMINI_API_KEY"])

response = client.models.generate_content(
    model="gemini-2.5-flash",
    contents="What is the capital of France?",
    config={"http_options": {"timeout": 30_000}},
)
print(response.text)
```

## Structured JSON output

```python
prompt = """Classify the sentiment of this text.

Text: "The product arrived damaged and customer service was unhelpful."

Respond in JSON only, no markdown:
{"sentiment": "positive|negative|neutral", "confidence": 0.0-1.0, "reason": "..."}"""

response = client.models.generate_content(
    model="gemini-2.5-flash",
    contents=prompt,
    config={"http_options": {"timeout": 30_000}},
)

import json
text = response.text.strip()

# Strip markdown code fences if present
if "```json" in text:
    text = text.split("```json")[1].split("```")[0].strip()
elif "```" in text:
    text = text.split("```")[1].split("```")[0].strip()

result = json.loads(text)
print(result)
```

## Key notes

- **Package**: Use `google-genai` (the new SDK), NOT `google-generativeai` (deprecated)
- **Model**: `gemini-2.5-flash` is fast and cheap. `gemini-2.5-pro` for harder tasks.
- **Timeout**: Set `http_options.timeout` in milliseconds. Default may be too short for complex prompts.
- **Thinking budget**: Gemini 2.5 Flash uses thinking tokens that consume `maxOutputTokens`. If responses get truncated, the thinking is eating the output budget. Fix via REST API:

## REST API (from Rust or any HTTP client)

```python
import requests, json

api_key = os.environ["GEMINI_API_KEY"]
url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={api_key}"

body = {
    "contents": [{"parts": [{"text": "Your prompt here"}]}],
    "generationConfig": {
        "temperature": 0.1,
        "maxOutputTokens": 2048,
        "thinkingConfig": {"thinkingBudget": 256}
    }
}

resp = requests.post(url, json=body, timeout=30)
data = resp.json()

# Extract the generated text
text = data["candidates"][0]["content"]["parts"][0]["text"]
print(text)
```

### thinkingBudget

Gemini 2.5 Flash is a "thinking" model. Without `thinkingConfig`, it may use 400+ tokens thinking and only 100 tokens for actual output. Set `thinkingBudget: 256` to cap thinking and `maxOutputTokens: 2048` for headroom.

The Python SDK handles this transparently. The REST API needs explicit config.
