"""Gold v2 benchmark labelling: shared prompt, schema, model calls and storage.

Gold v2 labels every substantive provision of the benchmark laws per actor, to
docs/architecture/DRRP-CLASSIFICATION.md (#68): what each actor itself holds
(Obligation / Liberty / none), its position, with dictionary labels only.
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
import yaml

PG = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw"
PROMPT_VERSION = "gold-v2-2026-09-30.3"
MODELS = {"gemini": "gemini-2.5-pro", "openai": "gpt-5.5"}
DICTIONARY = "crates/fractalaw-core/data/actor-dictionary.yaml"

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


def dictionary_block() -> str:
    seen, lines = set(), []
    for e in yaml.safe_load(open(DICTIONARY)):
        if e["label"] in seen:
            continue
        seen.add(e["label"])
        trig = ", ".join((e.get("triggers") or [])[:4])
        lines.append(f"- {e['label']} [{e['type']}]" + (f" (e.g. {trig})" if trig else ""))
    return "\n".join(lines)


SYSTEM_PROMPT = """You are an expert in UK and EU environment, safety and health (ESH) legislation and in Hohfeldian legal relations. You label ONE provision at a time for a gold-standard benchmark. Be precise and conservative.

## What to label
For each ACTOR (person, body or class of persons) the provision refers to, give:
1. `position`, the actor's role in the legal relation the provision creates:
   - `active`: the holder. It bears the duty/prohibition, or holds the permission/power/entitlement.
   - `counterparty`: the party the duty is owed to, or who is subject to the power/permission (the direct correlative).
   - `beneficiary`: benefits or is protected, but is not the holder or the direct correlative.
   - `mentioned`: referred to, with no role in a legal relation created here.
2. `holds`: what THIS actor itself holds in this provision:
   - `Obligation`: the actor bears a duty or prohibition ("shall", "must", "shall not", "no person shall", "is required to", "it shall be the duty of").
   - `Liberty`: the actor holds a power, permission or entitlement ("may", "is entitled to", "power to").
   - `none`: everything else.
   HARD RULE: only `active` actors hold Obligation/Liberty. A counterparty, beneficiary or mentioned actor ALWAYS holds `none`. Never copy the provision's type onto the other party.
3. `inferred`: true only for the implied-right case below.

## Provision-level fields
- `relation`: `yes` if the provision itself CREATES an Obligation or Liberty, otherwise `no`.
- `raw_type`: when relation is `yes` but NO actor is active (the holder isn't named, e.g. "records shall be kept for five years", "every workplace shall be ventilated"), give `Obligation` or `Liberty`; otherwise `null`.

## Rules (the fractalaw DRRP spec)
- **Offences and penalties are not duties.** "commits an offence", "is guilty of an offence", "is liable on conviction to …" → relation `no`, everyone `none`/`mentioned`. The duty lives in the provision the offence refers to.
- **Cross-references, commencement, conditions, details, definitions, exemptions, deeming, application/extent/scope** → relation `no`. Examples: "'premises' includes any place"; "a notice shall be treated as served if…"; "this Part applies to…"; "shall have effect as if…"; "nothing in this section shall affect…"; "the right under regulation 13 is a right to…"; "the period referred to in paragraph (1) is…".
- **Passive and thing-subject duties are Obligations with an unknown holder.** "records shall be kept", "equipment must be provided", "the register shall be available" with no named doer → relation `yes`, raw_type `Obligation`, and no actor is active unless the text names who must act.
- **Stems and list items.** You are given the provision's ancestors (its stem) as CONTEXT. Label the actors of the provision read together with its stem. If the provision completes a duty or power sentence begun in the stem (e.g. stem "It shall be the duty of each enforcing authority—", item "(a) to secure that the registers are available…"), the stem's holder is `active` in this provision too. Don't label a legal relation that exists only in the stem and not in this provision's own text.
- **Implied access rights.** Where a GOVERNMENT actor's Obligation is to make something available for inspection/copying by, or to supply copies on request/payment to, a governed party named in the provision (e.g. "the public", "any person"), that governed party is `active`, holds `Liberty`, `inferred: true`. Enforcement or notice-service provisions never qualify.
- **Holder named in a referenced provision.** You are also given the text of provisions this one refers to (REFERENCED PROVISIONS). When the provision's holder is identified only there (e.g. "Regulations under subsection (2) may prescribe…", where subsection (2) says "The Scottish Ministers may by regulations…"; or "A power under this section may be exercised by force", where the section confers the power on an authorised officer), that holder is `active` in this provision. Only use a holder that the referenced text actually names; otherwise the holder is unknown.
- **Content lists of schemes, regulations and notices.** "A scheme under this section must— (a) …", "Regulations may— (a) …": each item completes the stem's Obligation or Liberty, so relation `yes` with the same type. The holder is the scheme or regulation maker (resolved from the stem or referenced provisions), otherwise raw_type with no active actor.
- **Details nested inside a content list.** Below a content-list item, a sub-item that only states an eligibility criterion or defines a class of persons (e.g. "(i) the person's residence is in Scotland") is a detail → relation `no`. The Obligation/Liberty stays on the item above it.
- **Instruments are never actors.** A scheme, regulations, an order, a notice or a licence is not an actor. "The scheme may specify…" is a Liberty of the scheme maker when the stem or referenced provisions name it; otherwise raw_type with no active actor.
- **Applications to a court or tribunal.** "On the application of X, the court may…" → the court (`Gvt: Judiciary`) is `active` with Liberty. X is `mentioned`: its application is a condition, not a liberty of X.
- **Statutory defences.** "It is a defence for an accused … to prove that …" → relation `no`; the accused is `mentioned`. A defence qualifies the offence, and offences are not relations.
- **Commencement and citation.** A short title ("This Act may be cited as …") or a list of commencement dates → relation `no`. A POWER to commence ("on such day as the Secretary of State may by order appoint") is a Liberty held by that actor (`active`).
- **Procedural time limits.** "Proceedings may be commenced within 6 months …" → relation `no`: a limit on proceedings, not a liberty anyone holds.
- **Amending text** ("in section 5, for 'X' substitute 'Y'", inserted text) → relation `no`; actors `mentioned`.
- **Legal fiction.** "shall be treated as", "shall be deemed" → not an Obligation.
- A provision can have several active holders, e.g. an authority's Obligation and the public's implied Liberty.
- **Beneficiaries must be explicit.** Use `beneficiary` only where the text makes the protective or beneficial purpose explicit ("for the protection of", "a right of appeal against a refusal"). Persons whose eligibility or treatment a scheme may set, or who are merely described, are `mentioned`.
- **One entry per label.** If two different persons fit the same label, list the label once, with its strongest role (active > counterparty > beneficiary > mentioned).

## Actor labels
Use ONLY labels from this dictionary (exact spelling). [government] vs [governed] is the holder class. If no label fits, use "OTHER: <short description>". Use the most specific label that fits. Don't invent actors that the provision (with its stem) doesn't refer to.
- An officer, inspector or "authorised person" authorised by a government body (council, regulator, enforcing authority, Minister) is `Gvt: Officer`, never `Spc: Authorised Person`.
- "The Scottish Ministers" is `Gvt: Devolved Admin: Scottish Ministers` and "the Welsh Ministers" is `Gvt: Devolved Admin: Welsh Ministers`, not the Parliament or Assembly.
- "The person having (the management and) control of …" is `Ind: Person in Control`.

{dictionary}

## Output
JSON only:
{{"relation": "yes"|"no", "raw_type": "Obligation"|"Liberty"|null,
  "actors": [{{"label": "...", "position": "active"|"counterparty"|"beneficiary"|"mentioned", "holds": "Obligation"|"Liberty"|"none", "inferred": false}}],
  "reason": "one or two sentences"}}
"""

RESPONSE_SCHEMA = {
    "type": "object",
    "properties": {
        "relation": {"type": "string", "enum": ["yes", "no"]},
        "raw_type": {"type": ["string", "null"], "enum": ["Obligation", "Liberty", None]},
        "actors": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "label": {"type": "string"},
                    "position": {"type": "string", "enum": ["active", "counterparty", "beneficiary", "mentioned"]},
                    "holds": {"type": "string", "enum": ["Obligation", "Liberty", "none"]},
                    "inferred": {"type": "boolean"},
                },
                "required": ["label", "position", "holds", "inferred"],
                "additionalProperties": False,
            },
        },
        "reason": {"type": "string"},
    },
    "required": ["relation", "raw_type", "actors", "reason"],
    "additionalProperties": False,
}


def system_prompt() -> str:
    return SYSTEM_PROMPT.format(dictionary=dictionary_block())


def ancestors(section_id: str) -> list[str]:
    """Stem ids, nearest first: L:reg.2(3)(a) -> L:reg.2(3), L:reg.2 (as fractalaw_core::taxa::amendment)."""
    out, cur = [], section_id
    while cur.endswith(")"):
        i = cur.rfind("(")
        if i <= 0:
            break
        cur = cur[:i]
        out.append(cur)
    return out


_PREFIX = {"section": "s.", "regulation": "reg.", "article": "art.Article "}
_OWN_REF = re.compile(r"\b(section|regulation|article)\s+(\d+[A-Z]*)((?:\s*\(\w{1,4}\))*)(?!\s+(?:of|to)\s+(?:the|that)\b)", re.I)
_SUB_REF = re.compile(r"\b(?:subsection|paragraph)s?\s+\((\w{1,4})\)(?!\s+of\s+(?:section|regulation|article|schedule)\b)", re.I)
_THIS_REF = re.compile(r"\bthis\s+(section|regulation|article)\b", re.I)


def references(section_id: str, text: str, texts: dict[str, str]) -> list[tuple[str, str]]:
    """Provisions of the same law this one refers to ("subsection (2)", "regulation 5(1)", "this section")."""
    law, local = section_id.split(":", 1)
    base = local.split("(", 1)[0]
    found = []
    for m in _SUB_REF.finditer(text):
        found.append(f"{law}:{base}({m.group(1)})")
    for m in _OWN_REF.finditer(text):
        brackets = re.sub(r"\s+", "", m.group(3) or "")
        found.append(f"{law}:{_PREFIX[m.group(1).lower()]}{m.group(2)}{brackets}")
    if _THIS_REF.search(text):
        found += [f"{law}:{base}", f"{law}:{base}(1)"]
    own = {section_id, *ancestors(section_id)}
    out = []
    for sid in dict.fromkeys(found):
        cand = sid if texts.get(sid) else sid.split("(", 1)[0] if texts.get(sid.split("(", 1)[0]) else None
        if cand and cand not in own and cand not in {c for c, _ in out}:
            out.append((cand, texts[cand]))
    return out[:4]


def user_prompt(section_id: str, text: str, stems: list[tuple[str, str]], refs: list[tuple[str, str]] = ()) -> str:
    ctx = "\n".join(f"[{sid}] {t[:1500]}" for sid, t in reversed(stems)) or "(none)"
    ref = "\n".join(f"[{sid}] {t[:1500]}" for sid, t in refs) or "(none)"
    return (f"STEM CONTEXT (ancestors, outermost first):\n{ctx}\n\nREFERENCED PROVISIONS (same law):\n{ref}\n\n"
            f"PROVISION TO LABEL [{section_id}]:\n{text[:6000]}")


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
    schema = json.loads(json.dumps(RESPONSE_SCHEMA).replace('["string", "null"]', '"string"'))
    schema["properties"]["raw_type"] = {"type": "string", "enum": ["Obligation", "Liberty"], "nullable": True}
    for obj in (schema, schema["properties"]["actors"]["items"]):
        obj.pop("additionalProperties", None)
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
    body = {
        "model": MODELS["openai"],
        "instructions": system,
        "input": user,
        "text": {"format": {"type": "json_schema", "name": "provision_labels", "schema": RESPONSE_SCHEMA, "strict": True}},
    }
    if os.environ.get("GOLD_OPENAI_EFFORT"):  # minimal | low | medium | high
        body["reasoning"] = {"effort": os.environ["GOLD_OPENAI_EFFORT"]}
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
