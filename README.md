# acceptarium

> BRAINSTORM STAGE

acceptarium
:   (*Latin*) allotment-holding
:   (*Medieval*) receipt book

CLI tooling to facilitate scanning receipts, extracting useful data, archiving the assets, and importing the results into [Plain Text Accounting][pta] systems.

----

## Overview

1. Scan receipts.
2. Archive scanned assets using [Git Annex][gitannex] (or potentially pluggable backends? LFS? WebDAV?).
3. Extract data via OCR using local LLM tooling (or pluggable remote tooling).
4. Process data into structured transaction info.
5. Export data as transaction(s) via CVS (or possibly directly to journal for [HLedger][hledger], [Ledger CLI][ledgercli], [Beancount][beancount], etc.).

## Dependencies

* [Git Annex][gitannex]

[beancount]: https://beancount.io/
[gitannex]: https://git-annex.branchable.com/
[hledger]: https://hledger.org/
[ledgercli]: https://ledger-cli.org/
[pta]: https://plaintextaccounting.org/
