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

PROMPT_VERSION = "drrp-v1.4-2026-10-06"
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
3. `inferred`: true only for the implied-right case and for a holder taken from an APPLYING PROVISION (both below).
4. `act`: for a `counterparty` of an OBLIGATION (an active actor's, or a passive duty with no holder named, e.g. "notice shall be given to the operator"), what it is owed: one of `notify` (told of an event or decision), `supply` (given information, a copy or a thing), `consult`, `pay`, `give_access` (access or inspection), `serve` (served with a notice), `charge` (a charge levied on, or withheld from, it), `answer_request` (the duty answers its request), `other`. Otherwise `null`.

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
- **Parliamentary procedure** ("subject to annulment in pursuance of a resolution of either House", "may not be made unless a draft has been laid before and approved by …") → relation `no`. But a duty to LAY a report, direction or copy before Parliament or an Assembly ("The Secretary of State shall lay a copy of the report before Parliament") is relation `yes`: an Obligation of the government actor (active).
- **Exception: a deadline that compels a government actor to act** ("The first … regulations must come into force no later than 1 April 2018") → relation `yes`, Obligation of the regulation maker (active).
- **Passive and thing-subject duties are Obligations.** "records shall be kept", "equipment must be provided", "the register shall be available" with no named doer → relation `yes`, raw_type `Obligation`. Their holder comes only from the stem, a referenced provision or an applying provision (rules below); otherwise no actor is active. Never guess a holder.
- **Stems and list items.** You are given the provision's ancestors (its stem) as CONTEXT. Label the actors of the provision read together with its stem. If the provision completes a duty or power sentence begun in the stem (e.g. stem "It shall be the duty of each enforcing authority—", item "(a) to secure that the registers are available…"), the stem's holder is `active` in this provision too. Don't label a legal relation that exists only in the stem and not in this provision's own text.
- **Implied access rights.** Where a GOVERNMENT actor's Obligation is to make something available for inspection/copying by, or to supply copies on request/payment to, a governed party named in the provision (e.g. "the public", "any person"), that governed party is `active`, holds `Liberty`, `inferred: true`. Enforcement or notice-service provisions never qualify.
- **Holder named in a referenced provision.** You are also given the text of provisions this one refers to (REFERENCED PROVISIONS). When a provision that CREATES a relation identifies its holder only there (e.g. "Regulations under subsection (2) may prescribe…", where subsection (2) says "The Scottish Ministers may by regulations…"; or "A power under this section may be exercised by force", where the section confers the power on an authorised officer), that holder is `active`. Only use a holder that the referenced text actually names. This never turns a detail provision into a relation.
- **Holder named in an applying provision.** You are also given APPLYING PROVISIONS: provisions of the same law that put a named party under a duty to comply with, or to ensure compliance with, requirements that include this provision. Examples: "Every employer shall ensure that every workplace … complies with any requirement of these Regulations"; "A contractor carrying out construction work must comply with the requirements of this Part"; "a manufacturer must ensure that it has been designed and manufactured in accordance with the essential health and safety requirements"; "no person shall keep the material unless he complies with paragraphs (3) to (6)". When THIS provision creates an Obligation but names no one who must act (passive or thing-subject, e.g. "Every enclosed workplace shall be ventilated…"), each party that an applying provision puts under that duty is `active`, holds `Obligation`, `inferred: true`, and raw_type is `null`. Supervisory form: "the principal contractor must take all reasonable steps to ensure that contractors … comply with the duties under these Regulations" → BOTH the principal contractor and the contractors. Only use parties the applying text names; ignore an applying provision that names no party or whose scope doesn't truly include this provision. If this provision names its own holder (in its text or stem), use that and ignore the applying provisions. This never turns a detail provision or a no-relation provision into a relation.
- **Headless stems.** "X shall—" or "X may—" whose substance is in its items: the stem itself is relation `yes`, X `active` holding Obligation/Liberty, purpose `Requirement`/`Power Conferred`. Never `Procedure+Detail` just because the object is in the items.
- **Transitional powers and duties.** A transitional provision that itself confers a time-limited power or imposes a time-limited duty ("until 1 January 2020 a Member State may continue to authorise…") is relation `yes` with that Liberty/Obligation, purpose `Transitional Arrangement`. Pure savings and continuity ("as if this Act had not passed", "shall continue to have effect") stay relation `no`.
- **Member State, country: actor or place.** `EU: Member State` is an actor only when it bears the duty/power ("Member States shall ensure…") or receives the act. When it only names a jurisdiction or location ("the laws in force in the Member State", "placed on the market in a Member State"), it is a place: do not list it. Countries ("third countries"), the environment, animals, property and industries are never actors.
- **Content lists of schemes, regulations and notices.** "A scheme under this section must— (a) …", "Regulations may— (a) …": each item completes the stem's Obligation or Liberty, so relation `yes` with the same type. The holder is the scheme or regulation maker (resolved from the stem or referenced provisions), otherwise raw_type with no active actor.
- **Class-definition and criterion items.** Under ANY duty or power (not only content lists), an item that only states an eligibility criterion or defines which class of persons or things the duty covers (e.g. "(i) the person's residence is in Scotland", "(b) persons who are not employees") → relation `no`. The Obligation/Liberty stays on the provision above it.
- **Trigger-condition actors.** A party named only in a condition or trigger ("having regard to the risks to end-users", "where the vessel presents a risk to persons") is `mentioned`, never `beneficiary`: a beneficiary needs the duty's own protective purpose.
- **Enforcing-authority designations and functions lists.** "X shall be (responsible as) the enforcing authority for …" and lists of a body's functions that impose no duty in their own text → relation `no`, purpose `Establishment+Constitution`.
- **Money provided by Parliament.** "There shall be paid out of money provided by Parliament …" is an authority to spend → relation `no`, purpose `Charge+Fee`.
- **Time-limited disapplications** ("these Regulations shall not apply until …") → purpose `Exemption`.
- **A power to exempt** ("may by certificate exempt …") → relation `yes`, that actor `active` with Liberty, purpose `Exemption`. **Conditions on another provision's power** ("shall not grant any such exemption unless satisfied …", "shall not consent unless— (a) …") → detail: relation `no`, `Procedure+Detail`.
- **Functions of a governed party** ("safety representatives shall have the following functions— (a) …") → relation `yes`, that party `active` with Liberty, purpose `Power Conferred`. (Only the functions of a BODY are `Establishment+Constitution`.)
- **Content list or detail.** "The notice/report/register … must— (a) …": a content list (relation `yes`) when it completes a duty created in this same provision; a detail provision (relation `no`, `Procedure+Detail`) when it details something required elsewhere ("the notice referred to in paragraph (1) must contain"), enforcement notices included.
- **Timing items** ("at suitable intervals", "within 8 weeks") and **discharge details** ("shall comply with his duty under paragraph (1) by …") → relation `no`, `Procedure+Detail`.
- **Mixed provisions.** If a provision creates any duty or power of its own (e.g. an exemption plus a fallback duty), relation `yes`, and its purpose follows that operative part (`Requirement`/`Power Conferred`).
- **Withheld conduct** ("ensure that workers do not eat …", "shall not be permitted to remain"): the parties whose conduct is permitted or withheld are `counterparty`, act `other`.
- **Savings of old law or status** ("continues to have effect as if …", reliance on old documents as evidence) → relation `no`, purpose `Transitional Arrangement` (ahead of the machinery order). Pure deeming with no time element stays `Interpretation+Definition`, including electronic-delivery rules ("has effect as a delivery only if …").
- **Designation inside a definition** ("as may be designated by the Secretary of State by order" within a definition) → relation `no`, `Interpretation+Definition`. A Gazette notification inside a citation/commencement provision → relation `no`, `Enactment+Citation+Commencement`.
- **"Means at their disposal."** "Ensure the nominees have adequate time and means" → the nominees are `mentioned`; a counterparty with `supply` needs the text to say the party is provided with something.
- **Appeals.** A provision about an appeal or review of a decision has purpose `Defence+Appeal`; relation `yes` only if it creates its own duty or power.
- A relation-`no` class-definition or criterion item has purpose `Application+Scope`.
- **Instruments are never actors.** A scheme, regulations, an order, a notice or a licence is not an actor. "The scheme may specify…" is a Liberty of the scheme maker when the stem or referenced provisions name it; otherwise raw_type with no active actor.
- **Applications to a court or tribunal.** "On the application of X, the court may…" → the court (`Gvt: Judiciary`) is `active` with Liberty. X is `mentioned`: its application is a condition, not a liberty of X.
- **Statutory defences.** "It is a defence for an accused … to prove that …" → relation `no`; the accused is `mentioned`.
- **Commencement and citation.** A short title or a list of commencement dates → relation `no`. A POWER to commence ("on such day as the Secretary of State may by order appoint") is a Liberty held by that actor (`active`).
- **Procedural time limits.** "Proceedings may be commenced within 6 months …" → relation `no`.
- **Review clauses.** "The Secretary of State must review these Regulations and publish a report" → relation `yes`, Obligation of the Secretary of State.
- **Amending text** ("in section 5, for 'X' substitute 'Y'", inserted text) → relation `no`; actors `mentioned`.
- **Legal fiction.** "shall be treated as", "shall be deemed" → not an Obligation.
- A provision can have several active holders, e.g. an authority's Obligation and the public's implied Liberty.
- **One entry per label, strongest role.** If two different persons fit the same label, or one actor has two roles in the provision (e.g. receives a notification AND may shorten the period), list the label once, with its strongest role (active > counterparty > beneficiary > mentioned).

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
- Consistency: `Requirement` goes with relation `yes` (Obligation); `Power Conferred` with relation `yes` (Liberty); machinery and `Procedure+Detail` with relation `no`, except (1) a transitional provision that itself confers a time-limited power or duty (relation `yes`, purpose `Transitional Arrangement`) (2) a power to commence ("on such day as the Secretary of State may by order appoint": relation `yes`, that actor `active` with Liberty, purpose `Enactment+Citation+Commencement`), and (3) a power to exempt (relation `yes`, Liberty, purpose `Exemption`). Enforcement, fees and sanctions may go either way (an inspector's power is a Liberty; an offence creates no relation).

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


def dictionary_entries(yaml_text: str | None = None) -> dict[str, dict]:
    """label → {type, triggers, patterns} for the actor dictionary (or a given YAML text, e.g. an old commit)."""
    out = {}
    for e in yaml.safe_load(yaml_text if yaml_text is not None else open(DICTIONARY)):
        d = out.setdefault(e["label"], {"type": e["type"], "triggers": [], "patterns": []})
        d["triggers"] += e.get("triggers") or []
        d["patterns"] += e.get("regex_patterns") or []
    return out


def dictionary_version() -> str:
    """Short hash of the dictionary block the prompt carries."""
    import hashlib
    return hashlib.sha256(dictionary_block().encode()).hexdigest()[:8]


def label_version() -> str:
    """PROMPT_VERSION plus a hash of the actor dictionary block: the dictionary is part of the prompt,
    so labels made with a different dictionary are a different version (e.g. drrp-v1.1-2026-10-05+dict.1a2b3c4d)."""
    return f"{PROMPT_VERSION}+dict.{dictionary_version()}"


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


# Applying provisions (#60): a named party's duty to comply with, or ensure compliance with, requirements
# that cover other provisions ("Every employer shall ensure that every workplace … complies with any
# requirement of these Regulations"). They name the holder of passive/thing-subject duties they cover.
_SCOPE = (r"(?:these\s+Regulations|this\s+(?:Order|Act|Part|Schedule)|Part\s+\d+[A-Z]*\b|Schedules?\s+\d+[A-Z]*\b"
          r"|(?:regulations?|sections?|articles?)\s+\d+[A-Z]*\b(?:\(\w{1,4}\))*(?:\s*(?:,|to|and|or)\s*\d+[A-Z]*\b(?:\(\w{1,4}\))*)*"
          r"|(?:sub-?)?paragraphs?\s+\(\w{1,4}\)(?:\s*(?:,|to|and|or)\s*\(\w{1,4}\))*)")
_REQ = r"(?:any|the|all|each\s+of\s+the|such)?\s*(?:requirements?|provisions?|duties)\s+(?:of|in|imposed\s+by|under|contained\s+in)\s+"
_APPLY = re.compile(
    r"\b(?:shall|must)\b.{0,200}?(?:"
    rf"\bcompl(?:y|ies)\s+with\s+(?:{_REQ})?(?P<s1>{_SCOPE})"                       # comply / ensure X complies with …
    rf"|\b(?:ensure|secure)\s+that\s+{_REQ}(?P<s2>{_SCOPE}).{{0,80}}?\b(?:are|is)\s+complied\s+with"  # … are complied with
    # the scope must be this law's: not "regulation 48 of the Construction and Use Regulations",
    # "Article 21 of RAMS" or "Part 1 of Schedule 1 to the 2011 Order"
    r")(?!(?:\(\w{1,4}\))*(?:\s*\(.{0,200}?\))?\s+(?:of|to)\s+(?!these\s+Regulations\b|this\s+(?:Act|Order|Part|regulation|section|article)\b))", re.I | re.S)
# Product regimes: "a manufacturer must ensure that it has been designed and manufactured in accordance with the
# essential (health and safety) requirements" applies every provision that sets those requirements
_ESR_TERM = r"essential\s+(?:health\s+and\s+safety\s+)?requirements"
_ESR = re.compile(r"\b(?:shall|must)\b.{0,200}?\b(?:ensure|secure)\b.{0,200}?(?:in\s+accordance\s+with|satisf(?:y|ies)|meets?|compl(?:y|ies)\s+with)"
                  rf"\s+(?:all\s+)?(?:the\s+)?(?:relevant\s+|applicable\s+)?{_ESR_TERM}", re.I | re.S)
_ESR_MENTION = re.compile(_ESR_TERM, re.I)
# Mentions of compliance that put no one under a duty to comply
_NOT_APPLYING = re.compile(
    r"\boffence\b|\bguilty\b|\bfail(?:s|ed|ure)?\s+to\s+comply|\bcontravene|\bin\s+order\s+to\s+comply|\benabl"
    r"|\bpresumed\b|\btreated\s+as\b|\bdeemed\s+to\s+compl|\bopinion\b|\bsatisfied\b|\bneed\s+not\b|\bshall\s+not\s+apply\b"
    r"|\bnot\s+compl|\bnon-?complian|\bregard\s+to\b|\bas\s+if\b|\bwarn|\bstate\s+that\b|\bwhen\s+enforcing\b"
    r"|\bevidence\b|\bshowing\b|\bnotice\b|\bwhether\b|\bdoes\s+not\b|\bMember\s+States\b"
    r"|\b(?:taken|made|necessary|practicable|measures)\s+(?:\w+\s+){0,3}to\s+comply\b", re.I)
_NUM = re.compile(r"(\d+)([A-Z]*)((?:\(\w{1,4}\))*)")


def _base(local: str) -> tuple[str, str]:
    """('reg', '6') from 'reg.6(1)(a)'; ('sch', '2') from 'sch.2.reg.3'."""
    kind, _, rest = local.partition(".")
    m = _NUM.match(rest.removeprefix("Article "))
    return kind, (m.group(1) + m.group(2)) if m else ""


def _covers(scope: str, applier_local: str, applier_part: str | None):
    """Predicate over (local id, part) for one scope expression."""
    s = re.sub(r"\s+", " ", scope.strip())
    low = s.lower()
    if re.match(r"(?:sub-?)?paragraph", low):
        # paragraphs of the applier's own provision: "paragraphs (3) to (6)" of reg.6 → reg.6(3) … reg.6(6)
        head = applier_local.split("(", 1)[0]
        labels = re.findall(r"\((\w{1,4})\)", s)
        names, nums = set(labels), set()
        for a, b in re.findall(r"\((\d+)\)\s*to\s*\((\d+)\)", s):
            nums.update(str(n) for n in range(int(a), int(b) + 1))
        wanted = names | nums
        return lambda local, part: local.startswith(head + "(") and local[len(head) + 1:].split(")", 1)[0] in wanted
    if low in ("these regulations", "this order", "this act"):
        return lambda local, part: not local.startswith("sch.")
    if low == "this part":
        return lambda local, part: applier_part is not None and part == applier_part and not local.startswith("sch.")
    if low == "this schedule":
        return lambda local, part: False  # schedules aren't labelled on their own scope here
    if low.startswith("part"):
        n = s.split()[1]
        return lambda local, part: part == n and not local.startswith("sch.")
    if low.startswith("schedule"):
        n = s.split()[1]
        return lambda local, part: local.startswith("sch.") and _base(local)[1] == n
    # regulations/sections/articles: numbers, ranges ("5 to 27") and sub-refs ("14(4)")
    named, ranged, prefixes = set(), set(), []  # "regulation 8" ≠ 8A; "regulations 5 to 27" includes 8A
    toks = re.findall(r"\d+[A-Z]*(?:\(\w{1,4}\))*|\bto\b", s)
    i = 0
    while i < len(toks):
        t = toks[i]
        if t != "to" and i + 2 < len(toks) and toks[i + 1] == "to" and toks[i + 2] != "to":
            ranged.update(range(int(_NUM.match(t).group(1)), int(_NUM.match(toks[i + 2]).group(1)) + 1))
            i += 3
            continue
        if t != "to":
            m = _NUM.match(t)
            prefixes.append(t) if m.group(3) else named.add(m.group(1) + m.group(2))
        i += 1

    def pred(local, part):
        if local.startswith("sch.") or "." not in local:
            return False
        rest = local.split(".", 1)[1].removeprefix("Article ")
        base = _base(local)[1]
        digits = re.match(r"\d+", base)
        return (base in named or (digits is not None and int(digits.group()) in ranged)
                or any(rest.startswith(p) for p in prefixes))
    return pred


def applying_index(texts: dict[str, str], parts: dict[str, str | None]) -> dict[str, list]:
    """Per law, the applying provisions: [(section_id, rendered text, covers(local, part))]."""
    esr: dict[str, set[str]] = {}  # per law, provisions that set essential requirements
    for sid, text in texts.items():
        if text and _ESR_MENTION.search(text):
            law, local = sid.split(":", 1)
            esr.setdefault(law, set()).add(local)
    keys = None
    index: dict[str, list] = {}
    for sid, text in texts.items():
        if not text or not ("compl" in text.lower() or _ESR_MENTION.search(text)):
            continue
        law, local = sid.split(":", 1)
        stems = [texts[a] for a in reversed(ancestors(sid)) if texts.get(a)]
        full = " ".join(stems + [text])
        if _NOT_APPLYING.search(full):
            continue
        own_from = len(full) - len(text)  # a match must end in this provision's own text, not its stem's
        covers = [_covers(m.group("s1") or m.group("s2"), local, parts.get(sid))
                  for m in _APPLY.finditer(full) if m.end() > own_from]
        m = _ESR.search(full)
        if m and m.end() > own_from:
            covers.append(lambda loc, part, ls=esr.get(law, set()): loc in ls)
        if not covers:
            continue
        rendered = " … ".join([t[:400] for t in stems] + [text])
        if "—" in text or "–" in text or text.rstrip().endswith(":"):
            keys = keys or sorted(texts)
            items = [texts[k] for k in keys if k.startswith(sid + "(") and k.count("(") == sid.count("(") + 1 and texts[k]]
            rendered += " " + " ".join(t[:200] for t in items[:6])
        index.setdefault(law, []).append((sid, rendered, lambda loc, part, cs=covers: any(c(loc, part) for c in cs)))
    return index


def applying(section_id: str, parts: dict[str, str | None], index: dict[str, list]) -> list[tuple[str, str]]:
    """Applying provisions (same law) whose scope covers this provision, excluding its own stem and items."""
    law, local = section_id.split(":", 1)
    own = {section_id, *ancestors(section_id)}
    out = []
    for sid, rendered, covers in index.get(law, []):
        if sid in own or section_id in ancestors(sid) or sid in {s for s, _ in out}:
            continue
        if covers(local, parts.get(section_id)):
            out.append((sid, rendered))
    return out[:4]


def user_prompt(section_id: str, text: str, stems: list[tuple[str, str]], refs: list[tuple[str, str]] = (),
                apps: list[tuple[str, str]] = ()) -> str:
    ctx = "\n".join(f"[{sid}] {t[:1500]}" for sid, t in reversed(stems)) or "(none)"
    ref = "\n".join(f"[{sid}] {t[:1500]}" for sid, t in refs) or "(none)"
    app = "\n".join(f"[{sid}] {t[:1200]}" for sid, t in apps) or "(none)"
    return (f"STEM CONTEXT (ancestors, outermost first):\n{ctx}\n\nREFERENCED PROVISIONS (same law):\n{ref}\n\n"
            f"APPLYING PROVISIONS (same law; a named party's duty to comply with requirements that include this provision):\n{app}\n\n"
            f"PROVISION TO LABEL [{section_id}]:\n{text[:6000]}")
