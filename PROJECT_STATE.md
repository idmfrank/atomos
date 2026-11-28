# AtomOS current engineering state

Updated 2026-11-27. Active work: persistent local-agent scope and removal of obsolete source/docs. PR #40 is merged as main ed73c6b40358ac352c38bee6144c6c207c2b4442. This follow-up updates current validation evidence only; the agent performed no merge or deployment.

## Product and implementation

One owner, continuous Linux service, LAN browser interaction, broad local agency and graph-based learning. Optional external reasoning supplements local capability with zero incremental inference spending. External effects require approval or standing policy. [Roadmap](docs/agent-roadmap.md).

Implemented: local/mock text CLI, bounded sessions, SQLite/FTS5 typed graph with retrieval/correction/deletion/recovery, offline accounting/decoder/broker foundations and owner auth/model discovery. Service, web, autonomous executor, learning/capture, local models and production inference remain unimplemented.

Cleanup removes voice/audio components and public interfaces, obsolete one-shot inference tools and ledger writers, and archived documentation. Auth is auth/catalog only. Existing accounting files are untouched; a missing host cannot be initialized in a nonempty credential directory. Required SQLite schema/migration/integrity compatibility remains without removed voice APIs.

## Current deployment

Installed local/mock CLI release: e7e8fc9839090ac4cee4e6dc3140a6002124d655. Cleanup is not deployed; no host packages/configuration, credentials, private facts, capture settings or privileges changed. [Operations](docs/linux-cli-deployment.md).

## Validation

Current runtime source: `80f66cea92aff4656d817a67bd9992d47396c0db`. [PR CI 46100265296](https://github.com/idmfrank/atomos/actions/runs/46100265296) passed on synthetic merge `fe80c4b4744fcf6dd1f02716244a3b2f5081715e`; [full graph scale/recovery 46100262290](https://github.com/idmfrank/atomos/actions/runs/46100262290) passed on the branch source. The merged main tree exactly matches that validated runtime source. [Post-merge CI 46100497944](https://github.com/idmfrank/atomos/actions/runs/46100497944) passed on main ed73c6b40358ac352c38bee6144c6c207c2b4442. This evidence update changes documentation only; no runtime or dependency changes.

All 174 distinct tests passed: 137 Rust, 17 Python and 20 Node. Repeated feature/compiler runs and the ignored child-only crash fixture are not extra tests. Passed: workspace/CLI smoke, memory persistence/crash recovery, text-reference parity, removed-interface rejection, auth/state guards, 18,000 decoder samples with zero errors, admission/outcome/session accounting and overhead checks, Rust 1.75 core all-targets, and 1K CI graph smoke. Full 1K/10K/100K graph run returned 300/300 expected facts and 100 complete packets at each size, zero parity mismatches, zero recovery-verification errors and zero remote calls. Hosted synthetic results do not qualify new agent/model capabilities or device power.

Local WSL regression and Node syntax checks also passed. All 13 active Markdown files passed local path/anchor/fence checks; whitespace passed. No service/model/learning/security qualification is claimed. The documentation-only follow-up's own CI is pending at authoring; it need not recursively add its own SHA to this file.

## Constraints and next work

Recorded hardware: i7-6700, 16 GB RAM; owner reports 2 TB storage. Verify free/usable capacity, filesystem and headroom before quotas/model selection. No new model-quality, token/power-saving or isolation claim is established.

Next: durable task/service and LAN web contract; then web implementation, verified local execution/external boundary, graph learning/retention, measured local reasoning and optional external integration. [Architecture](docs/architecture.md), [memory](docs/amg-architecture.md), [inference](docs/inference-architecture.md), [security](docs/threat-model.md), [resources](docs/resource-budget.md).
