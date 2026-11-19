## QQ-FB-11. Two sources of truth

**Your point.** There is a risk of two sources of truth if the classification captured in IMR differs from the one ultimately used for reporting and decision-making.

**Response.** Agreed that it is a risk. The IMR gap analysis has addressed it: its action QQ-01 recommends a stated two-tool boundary. Power BI is for corporate reporting and decisions; IMR (EcoOnline) is for capture, case workflow and site-level views. All reporting already reads the reporting layer (the data pipeline that feeds Power BI), never IMR directly. IMR holds the ratings as entered, the pipeline derives the one classification every dashboard uses, and errors are corrected at source in IMR. So there is one source of truth for reporting.

The panel works within that boundary:

| | Where |
|---|---|
| The panel's list of cases to sentence (triage flags, AI model candidates, the audit sample) | The reporting layer (an Excel workbook) |
| The details of each case | The IMR record, opened by the panel |
| The panel's classification, with control state | Recorded back in the reporting layer, joined to the IMR record by its report key |
| The triage rating | Stays in IMR, as the triager's view; kept alongside for comparison, not reported as the classification |

The panel's classification is the classification of record in the reporting layer. IMR needs no new field. The proposal (§4.1) now says so.
