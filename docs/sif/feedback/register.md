---
register: SIF feedback register
updated: 2026-11-19
sources:
  - id: QQ-FB-2026-11-19
    from: QQ colleague
    received: 2026-11-19
    on: docs/sif/proposal.md v0.1
    verbatim: docs/sif/research/qq-feedback/2026-11-19-colleague.md  # gitignored
items:
  - id: QQ-FB-01
    source: QQ-FB-2026-11-19
    kind: confirm
    touches: [docs/sif/proposal.md]
    point: >
      Reads the problem as inconsistent individual judgement of severity, likelihood and
      credibility, making PSIF data hard to trend and compare. Reads the solution as five
      parts: triage as a broad filter; the AI model as a second reading and extra
      candidates; a calibrated panel decides; potential assessed apart from controls;
      events treated apart from precursors and exposures. Asks whether that is right.
    status: open
    response:
    changes: []
  - id: QQ-FB-02
    source: QQ-FB-2026-11-19
    kind: confirm
    touches: [docs/sif/proposal.md]
    point: >
      Supports the direction, above all moving the final decision from individual
      triagers to a calibrated panel using consistent criteria.
    status: open
    response:
    changes: []
  - id: QQ-FB-03
    source: QQ-FB-2026-11-19
    kind: design
    touches: [docs/sif/proposal.md, docs/sif/sif-definition.md]
    point: >
      At most three core categories: SIF (actual outcome); HiPo (no SIF, but could
      credibly have caused a life-ending, life-threatening or life-altering injury);
      SIF precursor or exposure (a credible SIF scenario exists from a condition,
      control weakness or work activity, but no event has occurred).
    status: open
    response:
    changes: []
  - id: QQ-FB-04
    source: QQ-FB-2026-11-19
    kind: design
    touches: [docs/sif/proposal.md, docs/sif/sif-definition.md]
    point: >
      Asks whether SIFP or PSIF terminology helps or hinders, and suggests HiPo (high
      potential event) as the operational term with the same method behind it.
    status: open
    response:
    changes: []
  - id: QQ-FB-05
    source: QQ-FB-2026-11-19
    kind: design
    touches: [docs/sif/proposal.md, docs/sif/sif-definition.md]
    point: >
      Agrees with the energy principle, but non-safety users think in hazards and
      consequences. Suggests a plain front-end question ("Could this realistically have
      resulted in a life-ending, life-threatening or life-altering injury?") with the
      energy method kept behind the scenes.
    status: open
    response:
    changes: []
  - id: QQ-FB-06
    source: QQ-FB-2026-11-19
    kind: design
    touches: [docs/sif/proposal.md]
    point: >
      Define the audiences (safety professionals analysing trends, senior leaders) and
      the decisions each must make before finalising, to set how much complexity is needed.
    status: open
    response:
    changes: []
  - id: QQ-FB-07
    source: QQ-FB-2026-11-19
    kind: gap
    touches: [docs/sif/sif-scenario-index.md, docs/sif/proposal.md]
    point: >
      The same subjectivity (likelihood, credibility, dependence on controls) exists in
      risk assessment, not only in PSIF rating.
    status: open
    response:
    changes: []
  - id: QQ-FB-08
    source: QQ-FB-2026-11-19
    kind: gap
    touches: [docs/sif/sif-scenario-index.md, docs/sif/proposal.md]
    point: >
      A golden thread across risk assessment, HiPo and SIF identification, investigation
      categorisation, assurance and control verification, and organisational learning.
    status: open
    response:
    changes: []
  - id: QQ-FB-09
    source: QQ-FB-2026-11-19
    kind: gap
    touches: [docs/sif/sif-controls-precursors.md, docs/sif/proposal.md]
    point: >
      Assurance should give leading indicators on SIF-capable work and critical
      controls: whether they are present and performing, not only classifying events
      afterwards.
    status: open
    response:
    changes: []
  - id: QQ-FB-10
    source: QQ-FB-2026-11-19
    kind: gap
    touches: [docs/sif/proposal.md]
    point: >
      If candidates progress only when a triager puts them forward, some events are
      never considered.
    status: open
    response:
    changes: []
  - id: QQ-FB-11
    source: QQ-FB-2026-11-19
    kind: gap
    touches: [docs/sif/proposal.md]
    point: >
      Risk of two sources of truth if the classification held in IMR differs from the
      one used for reporting and decisions.
    status: open
    response:
    changes: []
---

# SIF feedback register

The register holds feedback on the SIF papers and the AI model, one item per point, in the frontmatter above. It is the source of truth. See [README.md](README.md) for the approach, the rules and how to query it.
