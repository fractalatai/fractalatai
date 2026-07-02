#!/usr/bin/env /usr/bin/python3
"""Compute spaCy dependency parsing features for provision_actors.

Reads provision text from legislation_text, parses with spaCy, extracts
7 dep features per (provision, actor) pair, and writes to provision_actors.

Usage:
    /usr/bin/python3 scripts/compute_dep_features.py
    /usr/bin/python3 scripts/compute_dep_features.py --laws UK_ukpga_1974_37
    /usr/bin/python3 scripts/compute_dep_features.py --model en_core_web_trf
"""

import argparse
import psycopg2
import spacy

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"


def actor_search_terms(label):
    """Generate search terms from canonical actor label."""
    parts = label.split(":")
    last = parts[-1].strip().lower()
    terms = [last]
    if len(last.split()) > 1:
        terms.append(last.split()[-1])
    return terms


def find_actor_in_doc(doc, label):
    """Find actor token in parsed doc via phrase matching."""
    terms = actor_search_terms(label)
    text_lower = doc.text.lower()
    for term in terms:
        idx = text_lower.find(term)
        if idx >= 0:
            span = doc.char_span(idx, idx + len(term), alignment_mode="expand")
            if span and len(span) > 0:
                return span[0]
    return None


def extract_dep_features(doc, actor_label):
    """Extract 7 dep features for an actor in a parsed doc."""
    is_subject = 0.0
    is_object = 0.0
    is_agent = 0.0
    is_attr = 0.0
    voice_passive = 0.0
    has_modal = 0.0
    verb_dist = 0.5

    root = None
    for token in doc:
        if token.dep_ == "ROOT":
            root = token
            voice_passive = (
                1.0
                if any(c.dep_ == "auxpass" for c in token.children)
                else 0.0
            )
            has_modal = (
                1.0
                if any(
                    c.dep_ == "aux" and c.text.lower() in ("shall", "must", "may", "should")
                    for c in token.children
                )
                else 0.0
            )
            break

    if not root:
        return (is_subject, is_object, is_agent, is_attr, voice_passive, has_modal, verb_dist)

    actor_token = find_actor_in_doc(doc, actor_label)

    if actor_token:
        head = actor_token
        while head.head != head and head.head != root:
            head = head.head
        if head.head == root:
            if head.dep_ in ("nsubj", "nsubjpass"):
                is_subject = 1.0
            elif head.dep_ in ("agent",):
                is_agent = 1.0
            elif head.dep_ in ("dobj", "pobj"):
                is_object = 1.0
            elif head.dep_ in ("attr",):
                is_attr = 1.0

        h = actor_token
        dist = 0
        while h != root and dist < 20:
            h = h.head
            dist += 1
        if h == root:
            verb_dist = min(dist / 10.0, 1.0)

    return (is_subject, is_object, is_agent, is_attr, voice_passive, has_modal, verb_dist)


def main():
    parser = argparse.ArgumentParser(description="Compute dep parsing features")
    parser.add_argument("--laws", help="Comma-separated law names (default: all)")
    parser.add_argument("--model", default="en_core_web_md", help="spaCy model name")
    args = parser.parse_args()

    print(f"Loading spaCy model: {args.model}")
    nlp = spacy.load(args.model)

    conn = psycopg2.connect(PG)
    cur = conn.cursor()

    # Get provisions that have actors needing dep features
    where = ""
    if args.laws:
        law_list = [l.strip() for l in args.laws.split(",")]
        placeholders = ",".join(["%s"] * len(law_list))
        where = f"AND lt.law_name IN ({placeholders})"
        cur.execute(
            f"SELECT DISTINCT lt.section_id, lt.text "
            f"FROM legislation_text lt "
            f"JOIN provision_actors pa ON lt.section_id = pa.section_id "
            f"WHERE lt.text IS NOT NULL {where}",
            law_list,
        )
    else:
        cur.execute(
            "SELECT DISTINCT lt.section_id, lt.text "
            "FROM legislation_text lt "
            "JOIN provision_actors pa ON lt.section_id = pa.section_id "
            "WHERE lt.text IS NOT NULL "
            "AND pa.dep_is_subject IS NULL"
        )

    provisions = cur.fetchall()
    print(f"Processing {len(provisions)} provisions")

    # Get actors per provision
    if args.laws:
        cur.execute(
            f"SELECT pa.section_id, pa.actor_label "
            f"FROM provision_actors pa "
            f"JOIN legislation_text lt ON pa.section_id = lt.section_id "
            f"WHERE 1=1 {where}",
            law_list if args.laws else [],
        )
    else:
        cur.execute(
            "SELECT pa.section_id, pa.actor_label "
            "FROM provision_actors pa "
            "WHERE pa.dep_is_subject IS NULL"
        )

    actors_by_sid = {}
    for sid, label in cur.fetchall():
        actors_by_sid.setdefault(sid, []).append(label)

    # Process in batches
    total_updated = 0
    texts = [(sid, text[:500]) for sid, text in provisions]

    for i, (sid, text) in enumerate(texts):
        if sid not in actors_by_sid:
            continue

        doc = nlp(text)

        for actor_label in actors_by_sid[sid]:
            feats = extract_dep_features(doc, actor_label)
            cur.execute(
                "UPDATE provision_actors SET "
                "dep_is_subject = %s, dep_is_object = %s, dep_is_agent = %s, "
                "dep_is_attr = %s, dep_voice_passive = %s, dep_has_modal = %s, "
                "dep_verb_distance = %s "
                "WHERE section_id = %s AND actor_label = %s",
                (*feats, sid, actor_label),
            )
            total_updated += 1

        if (i + 1) % 500 == 0:
            conn.commit()
            print(f"  {i+1}/{len(texts)} provisions, {total_updated} actors updated")

    conn.commit()
    print(f"\nDone. {total_updated} actors updated across {len(texts)} provisions.")

    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
