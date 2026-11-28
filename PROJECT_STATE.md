# AtomOS current engineering state

Updated 2026-11-27. Active work: durable synthetic task/action runtime on main base 5edefff40248c91427270ce0422bf60e9998f82a (PR #42 merged). [Post-merge CI 46101698882](https://github.com/idmfrank/atomos/actions/runs/46101698882) passed. No merge/deployment, host configuration, credentials, private data or real inference is performed by this slice.

## Task foundation

New Rust library atomos-tasks uses the existing pinned rusqlite dependency in a separate private owner/policy-bound SQLite store. Implements durable submit/idempotency, FIFO/single active task, bounded admission, prepared intent/effect/verification records, cancellation, explicit post-open recovery and metadata cursor inspection. Only built-in synthetic counter operations exist; no worker loop, listener, planner, command callback, real permissions or external approval API. Task completion proves synthetic action results, not arbitrary natural-language goal success. [Implementation](docs/task-runtime.md).

Limits: 8 KiB goals, 16 newly admitted pending tasks plus the existing active task, 32 dispatches/600,000 active ms per task, 60,000 ms fixture effect deadline, metadata pages of 64 events/32 actions, required 64 KiB to 256 MiB main-DB cap. Recovery conservatively counts interrupted downtime and never drops acknowledged records. Terminal retention/free-space/aggregate quota, caller clock authenticity, worker cancellation and service isolation remain pending. Goals are private durable application state; Debug/events omit goal/key text. Storage failure poisons the runtime; no state reset.

Seventeen new task tests passed, including actual child death at eight boundaries, rollback, exclusive lease contention, full-database failure, limits, redaction and durable failure reasons. [Hosted PR CI 46104110281](https://github.com/idmfrank/atomos/actions/runs/46104110281) passed on synthetic merge a85271e8c0dd1779d211a7642076b5b7af1757cb of runtime source f611f3dbeb37e7933eb934e932f6aa83c47bb690. It verified Rust 1.77 task all-targets and core Rust 1.75, all 191 distinct tests, 18,000 decoder samples and accounting/session/task measurements. Ignored child fixtures and repeated compiler/feature runs are not extra tests. No deployment inventory or private-environment measurements are public evidence.

Documentation review includes README, roadmap, architecture, contract, security, resources, operations and engineering instructions. AMG, reasoning and auth APIs are unchanged; no private graph, auth vault or inference accounting access is added. Next: bounded worker/service loop and thin authenticated LAN adapter with synthetic actions; real executor and permission boundary require their own slice.

## Publication privacy

Deployment inventory, installed-version fingerprints and private environment measurements are excluded from public documentation and PR text. This documentation/checker change removes current public references, adds a publication rule and checks common accidental disclosures before CI proceeds. It does not erase earlier Git/PR revisions or external copies. No history rewrite, deployment or runtime change. Current-tree link/fence checks, publication-checker self-tests and source scan, and whitespace passed. The private audit found one older PR description needing sanitization, which was updated; returned issue/review metadata contained no remaining matching content. No private inventory is included in the checker itself. This privacy-only PR has its own CI pending.


## Product and implementation

One owner, continuous Linux service, LAN browser interaction, broad local agency and graph-based learning. Optional external reasoning supplements local capability with zero incremental inference spending. External effects require approval or standing policy. [Roadmap](docs/agent-roadmap.md).

Implemented: local/mock text CLI, bounded sessions, SQLite/FTS5 typed graph with retrieval/correction/deletion/recovery, offline accounting/decoder/broker foundations, durable synthetic tasks and owner auth/model discovery. Service, web, autonomous executor, learning/capture, local models and production inference remain unimplemented.

Cleanup removes voice/audio components and public interfaces, obsolete one-shot inference tools and ledger writers, and archived documentation. Auth is auth/catalog only. Existing accounting files are untouched; a missing host cannot be initialized in a nonempty credential directory. Required SQLite schema/migration/integrity compatibility remains without removed voice APIs.

## Current deployment

Deployment versions and inventory are maintained privately. Source changes do not authorize deployment, host configuration, credential access, capture or privilege changes. [Operations](docs/linux-cli-deployment.md).

## Validation

Runtime source f611f3dbeb37e7933eb934e932f6aa83c47bb690 passed hosted CI as linked above and [full graph scale/recovery 46104029156](https://github.com/idmfrank/atomos/actions/runs/46104029156). The graph run passed 1K/10K/100K: each size returned 300/300 expected facts and 100 complete packets, with zero parity mismatches, recovery-verification errors and remote calls. This is synthetic hosted-runner evidence, not deployment or model qualification.

The hosted task measurement asserted 100 completed, 100 cancelled, counter 100 and 700 events, zero errors/calls; acknowledgement p95 0.995897 ms, process RSS 4,416 KiB, apparent fixture store 86,528 bytes and example 432,112 bytes. These characterize the published fixture on hosted CI only, not a deployed system. Current follow-up changes publication policy/checks/docs only; Rust/runtime/dependency sources are unchanged. Current-tree publication, metadata-rejection (without matched-text echo), link/fence and whitespace checks passed. Follow-up CI is pending; full MVP gates, real worker and device qualification remain pending.

All 15 active docs and local paths/anchors/fences passed before publication review. Existing CLI/core/memory sources remain unchanged; lockfile adds only the task workspace package with existing registry versions. No private graph, vault or inference ledger access.


## Constraints and next work

Deployment inventory is private. Verify free/usable capacity, filesystem and headroom privately before quotas/model selection. No new model-quality, token/power-saving or isolation claim is established.

Next: implement the bounded task worker/service loop and LAN adapter; then verified local execution/external boundary, graph learning/retention, measured local reasoning and optional external integration. [Architecture](docs/architecture.md), [memory](docs/amg-architecture.md), [inference](docs/inference-architecture.md), [security](docs/threat-model.md), [resources](docs/resource-budget.md).
