"""Shared mapping from natural-language gold benchmark labels to canonical pipeline labels.

Used by benchmark_report.py and train_position_classifier.py to match
gold actors against provision_actors.
"""

LABEL_ALIASES = {
    # Core actors
    "employer": "Org: Employer",
    "employee": "Ind: Employee",
    "employees": "Ind: Employee",
    "person": "Ind: Person",
    "any person": "Ind: Person",
    "any other person": "Ind: Person",
    "persons": "Ind: Person",
    "relevant persons": "Ind: Person",
    "individual": "Ind: Person",
    "responsible person": "Ind: Responsible Person",
    "self-employed": "Ind: Self-Employed",
    "worker": "Ind: Worker",
    "exposed workers": "Ind: Worker",

    # Government
    "inspector": "Spc: Inspector",
    "hse": "Gvt: Agency: Health and Safety Executive",
    "health and safety executive": "Gvt: Agency: Health and Safety Executive",
    "executive": "Gvt: Agency: Health and Safety Executive",
    "the executive": "Gvt: Agency: Health and Safety Executive",
    "secretary of state": "Gvt: Minister",
    "scottish ministers": "Gvt: Minister",
    "welsh ministers": "Gvt: Minister",
    "local authority": "Gvt: Authority: Local",
    "enforcing authority": "Gvt: Authority: Enforcement",
    "authority": "Gvt: Authority",
    "competent authority": "Gvt: Authority",
    "relevant authority": "Gvt: Authority",
    "hazardous substances authority": "Gvt: Authority",
    "market surveillance authority": "Gvt: Authority",
    "office for nuclear regulation": "Gvt: Agency: Office for Nuclear Regulation",
    "nda": "Gvt: Agency: Nuclear Decommissioning Authority",
    "judiciary": "Gvt: Judiciary",
    "court": "Gvt: Judiciary",

    # EU
    "commission": "EU: Commission",
    "member states": "EU: Member State",
    "member state": "EU: Member State",

    # Organisations
    "occupier": "Ind: Occupier",
    "manufacturer": "Org: Manufacturer",
    "supplier": "Org: Supplier",
    "designer": "Org: Designer",
    "importer": "Org: Importer",
    "installer": "Org: Installer",
    "contractor": "Org: Contractor",
    "owner": "Ind: Owner",
    "duty holder": "Org: Duty Holder",
    "company": "Org: Company",
    "body corporate": "Org: Company",
    "undertaking": "Org: Undertaking",
    "responsible undertaking": "Org: Undertaking",
    "economic operator": "Org: Economic Operator",
    "relevant economic operator": "Org: Economic Operator",
    "client": "SC: Client",
    "hirer": "SC: Hirer",

    # Specialist
    "appellant": "Spc: Appellant",
    "applicant": "Spc: Applicant",
    "authorised person": "Spc: Authorised Person",
    "compliance body": "Spc: Compliance Body",
    "conformity assessment body": "Spc: Conformity Assessment Body",
    "professional body": "Spc: Professional Body",
    "scheme administrator": "Spc: Scheme Administrator",
    "participant": "Spc: Participant",
    "accused": "Spc: Accused",

    # Non-actors (gold quality issues — map to flag for cleanup)
    "electrical equipment": "_NOT_ACTOR",
    "civil explosive": "_NOT_ACTOR",
    "these regulations": "_NOT_ACTOR",
    "scheme": "_NOT_ACTOR",
    "northern ireland": "_NOT_ACTOR",
    "united kingdom": "_NOT_ACTOR",
    "parliament": "_NOT_ACTOR",
    "child": "_NOT_ACTOR",
    "public": "Ind: Public",
}


def normalise_label(label):
    """Normalise a gold label to match pipeline canonical form."""
    low = label.strip().lower()
    if low in LABEL_ALIASES:
        return LABEL_ALIASES[low]
    # Already canonical?
    if ":" in label:
        return label.strip()
    return label.strip()
