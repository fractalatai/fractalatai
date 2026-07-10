# Fitness Graph: Applicability Propagation

## Problem

Applicability is declared once and applies to many provisions. "This Part applies to any public authority" scopes every section within that Part. The extraction pipeline finds the declaration (Phase 1 polarity, Phase 2 mentions), but the propagation — applying that scope to all downstream provisions — requires understanding the structural graph of legislation.

## What We Have

### Structural hierarchy (Postgres)

Every provision has `hierarchy_path`, `part`, `chapter`, `depth`:

```
part.5/chapter.1/heading.125/provision.125  →  s.125 (MCZ duties)
part.5/chapter.1/heading.125/provision.126  →  s.126 (MCZ decisions)
part.5/chapter.1/heading.129/provision.129  →  s.129 (MCZ byelaws)
```

This gives us the tree: Part → Chapter → Heading Group → Provision. Scope declared at Part level propagates down to all provisions within.

### Inter-law edges (DuckDB)

`law_edges` table: 1M+ edges between laws.

| Edge type | Count | Relevance to fitness |
|---|---|---|
| amends / amended_by | 954K | An SI amending an Act may inherit or modify the Act's scope |
| rescinds / rescinded_by | 67K | Rescinded provisions lose applicability |
| enacts / enacted_by | 13K | Enacted SIs inherit the parent Act's scope context |

### Cross-references (fitness.rs)

`CROSS_REF_RE` already detects intra-law references: "regulation 6(4)", "Schedule 5", "paragraph (2)". These are edges within a single law.

## The Open Question: Nodes and Edges for Fitness

The existing graph serves DRRP (tracking amendments between laws). The fitness graph is different:

### Nodes

For fitness propagation, the nodes are **scope units** — the structural containers within a law that carry applicability:

- **Law** — top-level scope ("This Act applies to England and Wales")
- **Part** — mid-level scope ("This Part applies to any public authority")
- **Chapter** — sub-Part scope (less common for applicability)
- **Section/Regulation** — provision-level scope narrowing or extending
- **Schedule** — often carries its own scope, referenced from the main body

These already exist as values in `hierarchy_path`, `part`, `chapter`. They're not separate table rows — they're derived from the provision hierarchy.

### Edges

Three types of edges carry fitness information:

1. **Structural inheritance** (implicit, from hierarchy)
   - Part → sections within Part
   - Direction: downward (parent scope applies to children)
   - Example: "This Part applies to employers" → all sections in that Part inherit "applies to employers"

2. **Cross-reference override** (explicit, from text)
   - Section A → Section B ("Subsection (3) does not apply where...")
   - Direction: follows the reference
   - Can narrow (DisappliesTo) or extend (ExtendsTo) the inherited scope
   - Already detected by `CROSS_REF_RE` in fitness.rs

3. **Schedule linkage** (explicit, from text)
   - Section → Schedule ("Schedule 5 has effect for the purposes of this section")
   - Schedules often carry their own applicability distinct from the main body
   - Already detected by `CROSS_REF_RE`

### Inter-law fitness edges

When an SI amends an Act, does the SI's scope apply to the amended provisions? This is the hardest case:

- **Amending SI**: "The Conservation of Habitats and Species Regulations 2017" amends WCA 1981. The Regulations' scope doesn't override the Act's scope — the amended provisions retain the Act's applicability.
- **Commencement SI**: "This section comes into force on 1 October 2025". The commencement order's scope IS the temporal applicability of the target provision.
- **Extension SI**: "These Regulations extend the application of [Act] to [territory]". The SI creates a territorial scope edge to the Act.

For v1, inter-law fitness edges are limited to commencement and extension orders. Full amendment scope inheritance is deferred.

## Propagation Algorithm

```
1. For each law:
   a. Find all provisions with polarity (Phase 1 output)
   b. Group by scope_unit (the Part/Chapter/law the mention scopes)
   c. Build the scope tree:
      - Law-level mentions → root node
      - Part-level mentions → Part nodes
      - Section-level mentions → leaf overrides

2. Walk the tree top-down:
   a. Start with law-level AppliesTo mentions
   b. At each Part node, merge Part-level mentions:
      - AppliesTo adds to inherited scope
      - DisappliesTo subtracts from inherited scope
   c. At each provision, merge section-level mentions:
      - Cross-reference overrides apply here

3. Each provision ends up with:
   - Inherited scope (from law/Part)
   - Local overrides (from its own text)
   - Net applicability = inherited + local AppliesTo - local DisappliesTo
```

## Implementation Notes

- The hierarchy data is already in Postgres (`hierarchy_path`, `part`, `chapter`)
- No new graph database needed — this is a tree walk with cross-reference edges
- Cross-references within a law are already detected by `CROSS_REF_RE`
- The `scope_unit` field on mentions (from Phase 2 Layer 1) tells us what structural level the mention scopes
- Inter-law edges (amendment scope inheritance) are a separate, harder problem

## Scope for v1

- Structural hierarchy propagation within a single law (Part → sections)
- DisappliesTo as scope subtraction at any level
- Cross-reference overrides within a law (Section A disapplies Section B)
- Commencement date propagation from commencement orders

## Not in v1

- Full inter-law scope inheritance via amendment chains
- Schedule-to-main-body scope resolution (treat schedules as standalone scope units for now)
- "Incorporated by reference" — where an entirely different Act's scope is pulled in
