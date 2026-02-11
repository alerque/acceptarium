# acceptarium

> BRAINSTORM STAGE

acceptarium
:   (*Latin*) allotment-holding
:   (*Medieval*) receipt book

CLI tooling to facilitate scanning receipts, extracting useful data, archiving the assets, and importing the results into [Plain Text Accounting][pta] systems.

----

## Overview

```mermaid
---
config:
  layout: elk
---
flowchart LR
    A["Ingest/Scan"] --> B["ID (Store)"]
    B --> C["Review/Edit"] & D["OCR"]
    D --> E["LLM or Regex Extraction"]
    E --> C
    C --> F["Export"]
    style D stroke-dasharray: 5
    style E stroke-dasharray: 5
```

1. Scan or import scanned receipts, individually or in bulk.
1. Store identifiable scanned assets using [Git Annex][gitannex] or pluggable backends (LFS? WebDAV?).
1. **Optionally** extract data via OCR using local LLM tooling ([Ollama][ollama] or pluggable remote tooling).
1. **Optionally** automatically process data into structured transaction info (via local LLM tooling or pattern matching).
1. Facilitate either review of the data with a chance to edit (for automatically extracted data) or manual entry.
1. Export extracted data as transaction(s) via CVS (or possibly directly to journal for [HLedger][hledger], [Ledger CLI][ledgercli], [Beancount][beancount], etc.).

# Goals

* Automate as many steps as possible to make it easy to handle receipts, invoices, etc. in bulk.
* Use only local-first privacy-preserving tooling by default even where LLMs may be involved.
* Facilitate human review/approval and fully featured editing for any non-deterministic steps like LLM or OCR based meta-data extraction.
* Allow re-processing data from initial assets in the event of improved tooling (better OCR, more journal import rules, etc.).

## Non-goals

* Avoid lock-in to any particular PTA solution (pair with [HLedger][hledger], [Ledger CLI][ledgercli], [Beancount][beancount], or similar journal tools)
* Avoid dictating the entire accounting workflow; people have their own data handling already, we just want to mix in digitized assets.

[beancount]: https://beancount.io/
[gitannex]: https://git-annex.branchable.com/
[hledger]: https://hledger.org/
[ledgercli]: https://ledger-cli.org/
[ollama]: https://ollama.com/
[pta]: https://plaintextaccounting.org/
