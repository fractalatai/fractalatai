"""Gold v2 benchmark labelling: shared prompt, schema, model calls and storage.

Gold v2 labels every substantive provision of the benchmark laws with the
definitive prompt (scripts/drrp_prompt.py): per actor, what it holds, its
position and the correlative act; per provision, relation, raw type and purpose.
Two independent models label (Gemini, OpenAI); agreement is gold, and a Claude
agent referees disagreements.
"""

import json
import os
import re
import time
import urllib.error
import urllib.request

import psycopg

PG = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw"
# PROMPT_VERSION comes from scripts/drrp_prompt.py (was gold-v2-2026-09-30.3)
# Model tags; an OpenAI tag may carry a reasoning effort as "<model>:<effort>" (Jason, 2026-09-30:
# Gemini Flash + GPT-5.5 low, after the pilot comparison).
MODELS = {"gemini": "gemini-3.8-flash", "openai": "gpt-5.5:low"}

SCHEMA_DDL = """
CREATE TABLE IF NOT EXISTS gold_v2_raw (
    section_id text NOT NULL, law_name text NOT NULL, text_md5 text NOT NULL,
    model text NOT NULL, prompt_version text NOT NULL, response jsonb, error text,
    created_at timestamptz DEFAULT now(),
    PRIMARY KEY (section_id, text_md5, model, prompt_version));
ALTER TABLE gold_v2_raw ADD COLUMN IF NOT EXISTS usage jsonb;
CREATE TABLE IF NOT EXISTS gold_v2 (
    section_id text NOT NULL, law_name text NOT NULL, text_md5 text NOT NULL,
    actor_label text NOT NULL, position text NOT NULL, holds text NOT NULL,
    inferred boolean DEFAULT false, source text NOT NULL, note text,
    prompt_version text NOT NULL, created_at timestamptz DEFAULT now(),
    PRIMARY KEY (section_id, actor_label));
CREATE TABLE IF NOT EXISTS gold_v2_provision (
    section_id text PRIMARY KEY, law_name text NOT NULL, text_md5 text NOT NULL,
    relation text NOT NULL, raw_type text, source text NOT NULL, note text,
    prompt_version text NOT NULL, created_at timestamptz DEFAULT now());
"""


# The prompt, schema and context builders live in the one definitive module
# (scripts/drrp_prompt.py), shared with the SLM training set and the LLM tier.
import sys as _sys  # noqa: E402

_sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
from drrp_prompt import (  # noqa: E402,F401
    PROMPT_VERSION, RESPONSE_SCHEMA, ancestors, dictionary_block, gemini_schema, references,
    system_prompt, user_prompt,
)


def _post(url: str, body: dict, headers: dict, timeout: int = 180) -> dict:
    req = urllib.request.Request(url, data=json.dumps(body).encode(), headers={"Content-Type": "application/json", **headers})
    for attempt in range(5):
        try:
            return json.load(urllib.request.urlopen(req, timeout=timeout))
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt < 4:
                time.sleep(2 ** attempt * 5)
                continue
            raise RuntimeError(f"HTTP {e.code}: {e.read()[:300]!r}") from e
        except (urllib.error.URLError, TimeoutError):
            if attempt < 4:
                time.sleep(2 ** attempt * 5)
                continue
            raise


def call_gemini(system: str, user: str) -> dict:
    schema = gemini_schema()
    body = {
        "systemInstruction": {"parts": [{"text": system}]},
        "contents": [{"role": "user", "parts": [{"text": user}]}],
        "generationConfig": {"temperature": 0, "responseMimeType": "application/json", "responseSchema": schema},
    }
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{MODELS['gemini']}:generateContent?key={os.environ['GEMINI_API_KEY']}"
    r = _post(url, body, {})
    out = json.loads(r["candidates"][0]["content"]["parts"][0]["text"])
    out["_usage"] = r.get("usageMetadata")
    return out


def call_openai(system: str, user: str) -> dict:
    model, _, effort = MODELS["openai"].partition(":")
    body = {
        "model": model,
        "instructions": system,
        "input": user,
        "text": {"format": {"type": "json_schema", "name": "provision_labels", "schema": RESPONSE_SCHEMA, "strict": True}},
    }
    if effort:  # minimal | low | medium | high
        body["reasoning"] = {"effort": effort}
    r = _post("https://api.openai.com/v1/responses", body, {"Authorization": f"Bearer {os.environ['OPENAI_API_KEY']}"})
    text = next(c["text"] for o in r["output"] if o.get("type") == "message" for c in o["content"] if c.get("type") == "output_text")
    out = json.loads(text)
    out["_usage"] = r.get("usage")
    return out


CALLERS = {"gemini": call_gemini, "openai": call_openai}


def connect():
    conn = psycopg.connect(PG)
    conn.execute(SCHEMA_DDL)
    conn.commit()
    return conn


def norm_label(label: str) -> str:
    return re.sub(r"\s+", " ", label.strip())
