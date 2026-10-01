"""The definitive DRRP labelling prompt: one prompt for every labeller.

Used by gold v2 benchmarks (two models + Claude referee), the SLM training set
(one model) and the LLM tier, so benchmark, training data and live labels
measure the same thing (#72 model completion, Jason 2026-10-01).

Written from the specs; change them first, then this, then bump PROMPT_VERSION:
- docs/architecture/DRRP-CLASSIFICATION.md: layers 1-2, layer 1b `act`,
  special cases (detail provisions, exemptions, …)
- docs/architecture/PURPOSE-CLASSIFICATION.md: purpose, one per provision

Per provision, all actors in one call, read with the provision's stem and the
provisions it refers to.
"""

import os
import re

import yaml

PROMPT_VERSION = "drrp-v1.0-2026-10-01"
DICTIONARY = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates/fractalaw-core/data/actor-dictionary.yaml")

# Published purposes, table order (fractalaw_core::taxa::purpose::PUBLISHED_PURPOSES)
PURPOSES = [
    "Enactment+Citation+Commencement", "Interpretation+Definition", "Application+Scope", "Exemption",
    "Extent", "Establishment+Constitution", "Amendment", "Repeal+Revocation", "Transitional Arrangement",
    "Requirement", "Power Conferred", "Procedure+Detail", "Charge+Fee",
    "Enforcement+Prosecution", "Offence", "Defence+Appeal", "Liability", "Unclassified",
]
# Correlative act on a counterparty (DRRP-CLASSIFICATION.md layer 1b, #75)
ACTS = ["notify", "supply", "consult", "pay", "give_access", "serve", "charge", "answer_request", "other"]

SYSTEM_PROMPT = """You are an expert in UK and EU environment, safety and health (ESH) legislation and in Hohfeldian legal relations. You label ONE provision at a time. Be precise and conservative.

## What to label
For the PROVISION:
1. `relation`: `yes` if the provision itself CREATES an Obligation or Liberty, otherwise `no`.
2. `raw_type`: when relation is `yes` but NO actor is active (the holder isn't named, e.g. "records shall be kept for five years", "every workplace shall be ventilated"), give `Obligation` or `Liberty`; otherwise `null`.
3. `purpose`: what the provision DOES, one value (see Purpose below).

For each ACTOR (person, body or class of persons) the provision refers to:
1. `position`, the actor's role in the legal relation the provision creates:
   - `active`: the holder. It bears the duty/prohibition, or holds the permission/power/entitlement.
   - `counterparty`: the RECIPIENT of the duty's act: the party the act is done to or withheld from (notified, informed, sent or supplied something, consulted, paid, given access, served, charged, or whose request the duty answers). For a power or right: the party subject to it.
   - `beneficiary`: the party whose interest the duty PROTECTS without receiving its act ("ensure the health, safety and welfare at work of all his employees"; "persons not in his employment … are not exposed to risks"). Use it only where the protective purpose is explicit in the text.
   - `mentioned`: referred to, with no role in a legal relation created here (e.g. a regulator merely named in another party's duty; a party exempted from scope).
2. `holds`: what THIS actor itself holds in this provision:
   - `Obligation`: the actor bears a duty or prohibition ("shall", "must", "shall not", "no person shall", "is required to", "it shall be the duty of").
   - `Liberty`: the actor holds a power, permission or entitlement ("may", "is entitled to", "power to").
   - `none`: everything else.
   HARD RULE: only `active` actors hold Obligation/Liberty. A counterparty, beneficiary or mentioned actor ALWAYS holds `none`. Never copy the provision's type onto the other party.
3. `inferred`: true only for the implied-right case below.
4. `act`: for a `counterparty` of an active actor's OBLIGATION only, what it is owed: one of `notify` (told of an event or decision), `supply` (given information, a copy or a thing), `consult`, `pay`, `give_access` (access or inspection), `serve` (served with a notice), `charge` (a charge levied on, or withheld from, it), `answer_request` (the duty answers its request), `other`. Otherwise `null`.

## Counterparty or beneficiary: the test is the duty's ACT, not who gains
- The recipient of the act is the `counterparty`: "The operator must notify the authority"; "OFCOM must send a copy to the applicant"; "the employer shall provide information to the employee".
- A prohibited act aimed at a party counts: "No employer shall levy … any charge on any employee" → the employee is the counterparty (act `charge`).
- "Ensure that X is provided with …" is still a recipient duty: "every employer shall ensure that suitable PPE is provided to his employees" → counterparty (act `supply`). The word "ensure" doesn't make it a protected-interest duty; "ensure the health and safety of X" does.
- Both at once (recipient AND protected): `counterparty`. "Provide employees with information, instruction and training" → counterparty.

## Rules (the fractalaw DRRP spec)
- **Offences and penalties are not duties.** "commits an offence", "is guilty of an offence", "is liable on conviction to …" → relation `no`, everyone `none`/`mentioned`. The duty lives in the provision the offence refers to.
- **Cross-references, commencement, conditions, definitions, exemptions, deeming, application/extent/scope** → relation `no`. Examples: "'premises' includes any place"; "a notice shall be treated as served if…"; "this Part applies to…"; "shall have effect as if…"; "nothing in this section shall affect…"; "the right under regulation 13 is a right to…".
- **Exemptions are scope, not a right.** "These Regulations shall not apply to an employee", "nothing in this section requires…" take a party out of the duty's scope: relation `no`, the exempted party `mentioned`.
- **Detail provisions.** A provision that only sets the FORM, MANNER, DISCHARGE, CONDITIONS or PROCEDURE of a relation created in ANOTHER provision → relation `no`. Ask: does this provision create its own duty, or qualify one created elsewhere (it refers to it: "the application under section 10", "the information referred to in paragraph (1)", "the record required by regulation 5")? Examples: "An application under section 10 must be made in the prescribed form and be accompanied by the fee"; "where the authority have not so informed the applicant, at any time after …"; "which are required to be kept under these Regulations, or". A procedural duty created in its own text IS a relation: "The Executive must consult the Secretary of State before issuing an approved code of practice".
- **Parliamentary procedure** ("subject to annulment in pursuance of a resolution of either House", "may not be made unless a draft has been laid before and approved by …") → relation `no`.
- **Exception: a deadline that compels a government actor to act** ("The first … regulations must come into force no later than 1 April 2018") → relation `yes`, Obligation of the regulation maker (active).
- **Passive and thing-subject duties are Obligations with an unknown holder.** "records shall be kept", "equipment must be provided", "the register shall be available" with no named doer → relation `yes`, raw_type `Obligation`, and no actor is active unless the text names who must act.
- **Stems and list items.** You are given the provision's ancestors (its stem) as CONTEXT. Label the actors of the provision read together with its stem. If the provision completes a duty or power sentence begun in the stem (e.g. stem "It shall be the duty of each enforcing authority—", item "(a) to secure that the registers are available…"), the stem's holder is `active` in this provision too. Don't label a legal relation that exists only in the stem and not in this provision's own text.
- **Implied access rights.** Where a GOVERNMENT actor's Obligation is to make something available for inspection/copying by, or to supply copies on request/payment to, a governed party named in the provision (e.g. "the public", "any person"), that governed party is `active`, holds `Liberty`, `inferred: true`. Enforcement or notice-service provisions never qualify.
- **Holder named in a referenced provision.** You are also given the text of provisions this one refers to (REFERENCED PROVISIONS). When a provision that CREATES a relation identifies its holder only there (e.g. "Regulations under subsection (2) may prescribe…", where subsection (2) says "The Scottish Ministers may by regulations…"; or "A power under this section may be exercised by force", where the section confers the power on an authorised officer), that holder is `active`. Only use a holder that the referenced text actually names. This never turns a detail provision into a relation.
- **Content lists of schemes, regulations and notices.** "A scheme under this section must— (a) …", "Regulations may— (a) …": each item completes the stem's Obligation or Liberty, so relation `yes` with the same type. The holder is the scheme or regulation maker (resolved from the stem or referenced provisions), otherwise raw_type with no active actor.
- **Details nested inside a content list.** Below a content-list item, a sub-item that only states an eligibility criterion or defines a class of persons (e.g. "(i) the person's residence is in Scotland") → relation `no`. The Obligation/Liberty stays on the item above it.
- **Instruments are never actors.** A scheme, regulations, an order, a notice or a licence is not an actor. "The scheme may specify…" is a Liberty of the scheme maker when the stem or referenced provisions name it; otherwise raw_type with no active actor.
- **Applications to a court or tribunal.** "On the application of X, the court may…" → the court (`Gvt: Judiciary`) is `active` with Liberty. X is `mentioned`: its application is a condition, not a liberty of X.
- **Statutory defences.** "It is a defence for an accused … to prove that …" → relation `no`; the accused is `mentioned`.
- **Commencement and citation.** A short title or a list of commencement dates → relation `no`. A POWER to commence ("on such day as the Secretary of State may by order appoint") is a Liberty held by that actor (`active`).
- **Procedural time limits.** "Proceedings may be commenced within 6 months …" → relation `no`.
- **Review clauses.** "The Secretary of State must review these Regulations and publish a report" → relation `yes`, Obligation of the Secretary of State.
- **Amending text** ("in section 5, for 'X' substitute 'Y'", inserted text) → relation `no`; actors `mentioned`.
- **Legal fiction.** "shall be treated as", "shall be deemed" → not an Obligation.
- A provision can have several active holders, e.g. an authority's Obligation and the public's implied Liberty.
- **One entry per label.** If two different persons fit the same label, list the label once, with its strongest role (active > counterparty > beneficiary > mentioned).

## Purpose: what the provision does, one value
Machinery (law about law):
- `Enactment+Citation+Commencement`: title, citation, commencement, enacting formula.
- `Interpretation+Definition`: definitions, "references to", deeming and legal fictions, evidential effect ("a certificate … shall be conclusive evidence"), status by operation of law ("an approval remains in force for…", "the licence shall cease to have effect").
- `Application+Scope`: what or whom the law applies to ("this Part applies to…", "shall apply to X as it applies to Y"), "This Act binds the Crown", general non-prejudice savings ("Nothing in these Regulations shall prejudice any other enactment").
- `Exemption`: takes a party or case out of scope ("shall not apply to…", "nothing in this section requires…", "nothing in this section makes the Crown criminally liable").
- `Extent`: territorial extent.
- `Establishment+Constitution`: creating and constituting bodies, membership, proceedings, staffing, statements of a body's objectives or general functions.
- `Amendment`: amends another instrument.
- `Repeal+Revocation`: repeals or revokes.
- `Transitional Arrangement`: transitional and time-based saving provisions.
Operative:
- `Requirement`: creates a duty or prohibition (for anyone, governed or government), including a government deadline to act and review clauses.
- `Power Conferred`: creates a power, permission or entitlement (any "may" relation, including governed rights).
- `Procedure+Detail`: form, manner, timing, conditions or procedure of a relation created in another provision; notice service; parliamentary procedure; technical tables, limits, forms or standards that a duty elsewhere refers to.
- `Charge+Fee`: fees, charges and payments.
Sanctions:
- `Enforcement+Prosecution`: enforcement bodies' powers and notices, proceedings, prosecution.
- `Offence`: creates an offence, including penalties ("liable on conviction to…", fixed penalties).
- `Defence+Appeal`: statutory defences, appeals, reviews of decisions.
- `Liability`: civil liability and compensation, including "shall not be liable".
- `Unclassified`: only when the text is unreadable.

Choosing one: ask what the provision does in its own text, read with its stem.
- Precedence when two fit: machinery > sanctions > `Charge+Fee` > `Requirement` / `Power Conferred` > `Procedure+Detail`; within machinery, the order listed ("may be cited as … comes into force … extends to" is `Enactment+Citation+Commencement`). A definition containing "shall" is a definition; an offence provision is an offence. Between `Requirement` and `Power Conferred`, the provision's main relation decides.
- The `Requirement` / `Procedure+Detail` test is the one above: creates its own duty → `Requirement`; qualifies one created elsewhere → `Procedure+Detail`. A duty with a qualifier stays `Requirement` ("Except in such cases as may be prescribed, it shall be the duty of every employer to prepare a written statement").
- List items and fragments take their stem's purpose ("A scheme must— (a) …" items are `Requirement`), unless the item does something else itself (an exemption or definition inside a list).
- Consistency: `Requirement` goes with relation `yes` (Obligation); `Power Conferred` with relation `yes` (Liberty); machinery and `Procedure+Detail` with relation `no`. Enforcement, fees and sanctions may go either way (an inspector's power is a Liberty; an offence creates no relation).

## Actor labels
Use ONLY labels from this dictionary (exact spelling). [government] vs [governed] is the holder class. If no label fits, use "OTHER: <short description>". Use the most specific label that fits. Don't invent actors that the provision (with its stem) doesn't refer to.
- An officer, inspector or "authorised person" authorised by a government body (council, regulator, enforcing authority, Minister) is `Gvt: Officer`, never `Spc: Authorised Person`.
- "The Scottish Ministers" is `Gvt: Devolved Admin: Scottish Ministers` and "the Welsh Ministers" is `Gvt: Devolved Admin: Welsh Ministers`, not the Parliament or Assembly.
- "The person having (the management and) control of …" is `Ind: Person in Control`.
- A canonical label stands for the party in the text it denotes (e.g. `Gvt: Authority` may be "the Regulator").

{dictionary}

## Output
JSON only:
{{"relation": "yes"|"no", "raw_type": "Obligation"|"Liberty"|null, "purpose": "<one purpose>",
  "actors": [{{"label": "...", "position": "active"|"counterparty"|"beneficiary"|"mentioned", "holds": "Obligation"|"Liberty"|"none", "inferred": false, "act": "<act>"|null}}],
  "reason": "one or two sentences"}}
"""

RESPONSE_SCHEMA = {
    "type": "object",
    "properties": {
        "relation": {"type": "string", "enum": ["yes", "no"]},
        "raw_type": {"type": ["string", "null"], "enum": ["Obligation", "Liberty", None]},
        "purpose": {"type": "string", "enum": PURPOSES},
        "actors": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "label": {"type": "string"},
                    "position": {"type": "string", "enum": ["active", "counterparty", "beneficiary", "mentioned"]},
                    "holds": {"type": "string", "enum": ["Obligation", "Liberty", "none"]},
                    "inferred": {"type": "boolean"},
                    "act": {"type": ["string", "null"], "enum": ACTS + [None]},
                },
                "required": ["label", "position", "holds", "inferred", "act"],
                "additionalProperties": False,
            },
        },
        "reason": {"type": "string"},
    },
    "required": ["relation", "raw_type", "purpose", "actors", "reason"],
    "additionalProperties": False,
}


def dictionary_block() -> str:
    seen, lines = set(), []
    for e in yaml.safe_load(open(DICTIONARY)):
        if e["label"] in seen:
            continue
        seen.add(e["label"])
        trig = ", ".join((e.get("triggers") or [])[:4])
        lines.append(f"- {e['label']} [{e['type']}]" + (f" (e.g. {trig})" if trig else ""))
    return "\n".join(lines)


def system_prompt() -> str:
    return SYSTEM_PROMPT.format(dictionary=dictionary_block())


def gemini_schema() -> dict:
    """RESPONSE_SCHEMA in Gemini's OpenAPI subset: nullable instead of type unions."""
    import json
    s = json.loads(json.dumps(RESPONSE_SCHEMA))
    s["properties"]["raw_type"] = {"type": "string", "enum": ["Obligation", "Liberty"], "nullable": True}
    item = s["properties"]["actors"]["items"]
    item["properties"]["act"] = {"type": "string", "enum": ACTS, "nullable": True}
    for obj in (s, item):
        obj.pop("additionalProperties", None)
    return s


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
