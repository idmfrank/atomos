# AtomOS current engineering state

Updated 2026-11-27. Active work: persistent local-agent scope and removal of obsolete source/docs. PR #40 is open on codex/local-agent-roadmap; no merge or deployment is performed here.

## Product and implementation

One owner, continuous Linux service, LAN browser interaction, broad local agency and graph-based learning. Optional external reasoning supplements local capability with zero incremental inference spending. External effects require approval or standing policy. [Roadmap](docs/agent-roadmap.md).

Implemented: local/mock text CLI, bounded sessions, SQLite/FTS5 typed graph with retrieval/correction/deletion/recovery, offline accounting/decoder/broker foundations and owner auth/model discovery. Service, web, autonomous executor, learning/capture, local models and production inference remain unimplemented.

Cleanup removes voice/audio components and public interfaces, obsolete one-shot inference tools and ledger writers, and archived documentation. Auth is auth/catalog only. Existing accounting files are untouched; a missing host cannot be initialized in a nonempty credential directory. Required SQLite schema/migration/integrity compatibility remains without removed voice APIs.

## Current deployment

Installed local/mock CLI release: e7e8fc9839090ac4cee4e6dc3140a6002124d655. Cleanup is not deployed; no host packages/configuration, credentials, private facts, capture settings or privileges changed. [Operations](docs/linux-cli-deployment.md).

## Validation

Final cleanup passed locally in WSL: default workspace/CLI smoke, memory process persistence/recovery, feature-enabled session workspace tests, seven conversation/memory process tests, nine session process tests with exact text-reference parity, 18,000 decoder samples with zero errors, retained Node auth/storage tests including removed-command and unrecognized-state guards, and Rust 1.75 core all-targets with atomic-admission. Node syntax checks passed. All 13 current Markdown files passed link/anchor/fence checks; whitespace passed. Local synthetic log: current-scope-validation.log outside the repository. GitHub CI and full 1K/10K/100K scale/recovery for the expanded candidate are pending. New service/model/learning checks await implementation.

## Constraints and next work

Recorded hardware: i7-6700, 16 GB RAM; owner reports 2 TB storage. Verify free/usable capacity, filesystem and headroom before quotas/model selection. No new model-quality, token/power-saving or isolation claim is established.

Next: durable task/service and LAN web contract; then web implementation, verified local execution/external boundary, graph learning/retention, measured local reasoning and optional external integration. [Architecture](docs/architecture.md), [memory](docs/amg-architecture.md), [inference](docs/inference-architecture.md), [security](docs/threat-model.md), [resources](docs/resource-budget.md).
