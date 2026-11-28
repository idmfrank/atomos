# AtomOS current engineering state

Updated 2026-11-27. Active work: durable synthetic task/action runtime on main base 5edefff40248c91427270ce0422bf60e9998f82a (PR #42 merged). [Post-merge CI 46101698882](https://github.com/idmfrank/atomos/actions/runs/46101698882) passed. No merge/deployment, host configuration, credentials, private data or real inference is performed by this slice.

## Task foundation

New Rust library atomos-tasks uses the existing pinned rusqlite dependency in a separate private owner/policy-bound SQLite store. Implements durable submit/idempotency, FIFO/single active task, bounded admission, prepared intent/effect/verification records, cancellation, explicit post-open recovery and metadata cursor inspection. Only built-in synthetic counter operations exist; no worker loop, listener, planner, command callback, real permissions or external approval API. Task completion proves synthetic action results, not arbitrary natural-language goal success. [Implementation](docs/task-runtime.md).

Limits: 8 KiB goals, 16 newly admitted pending tasks plus the existing active task, 32 dispatches/600,000 active ms per task, 60,000 ms fixture effect deadline, metadata pages of 64 events/32 actions, required 64 KiB to 256 MiB main-DB cap. Recovery conservatively counts interrupted downtime and never drops acknowledged records. Terminal retention/free-space/aggregate quota, caller clock authenticity, worker cancellation and service isolation remain pending. Goals are private durable application state; Debug/events omit goal/key text. Storage failure poisons the runtime; no state reset.

Seventeen new tests passed locally, including a parent that actually kills a child at eight boundaries, SQLite rollback, full-database fail-closed behavior, no duplicate effects, cancellation, time/capacity limits, cursor privacy and private-store validation. Ignored child fixture is not an extra test. Local compiler check found existing rusqlite C-string syntax incompatible with Rust 1.75; new crate explicitly declares 1.77 and CI verifies that minimum separately. Core 1.75 remains unchanged. Local full retained workspace/session/decoder/auth checks passed, along with the final 17 task tests and separate Rust 1.77 task / Rust 1.75 core checks. Final WSL ext4 synthetic task measurement passed: 100 completed, 100 cancelled, counter 100, 700 events, zero errors/calls; cancellation acknowledgement p95 10.005317 ms, peak process RSS 4,560 KiB, apparent store 86,528 bytes and release example 432,104 bytes. These measured an uncommitted candidate based on the stated main base, not an executable at that base SHA. The benchmark header now reports worktree changes explicitly. Windows-mounted 9p could not satisfy private-file checks and was rejected, not counted as a durability result. Local log: task-final-validation.log outside the repository. All 15 active Markdown files and 80 local paths/anchors/fences passed, as did whitespace and unchanged-runtime/lockfile checks. Candidate CI/full-scale checks are pending at authoring. No whole-device/A07/model-quality claim.

Documentation review includes README, roadmap, architecture, contract, security, resources, operations and engineering instructions. AMG, reasoning and auth APIs are unchanged; no private graph, auth vault or inference accounting access is added. Next: bounded worker/service loop and thin authenticated LAN adapter with synthetic actions; real executor and permission boundary require their own slice.

## Product and implementation

One owner, continuous Linux service, LAN browser interaction, broad local agency and graph-based learning. Optional external reasoning supplements local capability with zero incremental inference spending. External effects require approval or standing policy. [Roadmap](docs/agent-roadmap.md).

Implemented: local/mock text CLI, bounded sessions, SQLite/FTS5 typed graph with retrieval/correction/deletion/recovery, offline accounting/decoder/broker foundations, durable synthetic tasks and owner auth/model discovery. Service, web, autonomous executor, learning/capture, local models and production inference remain unimplemented.

Cleanup removes voice/audio components and public interfaces, obsolete one-shot inference tools and ledger writers, and archived documentation. Auth is auth/catalog only. Existing accounting files are untouched; a missing host cannot be initialized in a nonempty credential directory. Required SQLite schema/migration/integrity compatibility remains without removed voice APIs.

## Current deployment

Installed local/mock CLI release: e7e8fc9839090ac4cee4e6dc3140a6002124d655. Cleanup is not deployed; no host packages/configuration, credentials, private facts, capture settings or privileges changed. [Operations](docs/linux-cli-deployment.md).

## Validation

Current base main passed CI as linked above. Local candidate validation covers 191 distinct tests (154 Rust, 17 Python, 20 Node): the existing 174 plus 17 task tests. Repeated compiler/feature runs and both child-only crash helpers are not extra tests. The final task/failure-reason revision passed all-targets under Rust 1.77 and the ext4 benchmark; existing CLI/core/memory runtime sources remain byte-identical to base. The lockfile adds only the workspace task package and reuses the pinned SQLite dependency. No new registry package versions.

CI adds task durability measurement and a separate 1.77 task all-target check; core 1.75 checks remain. Current-candidate PR CI and full 1K/10K/100K graph scale/recovery are pending. No new worker/service, capture, model, actual-device or full MVP gate result is claimed.


## Constraints and next work

Recorded hardware: i7-6700, 16 GB RAM; owner reports 2 TB storage. Verify free/usable capacity, filesystem and headroom before quotas/model selection. No new model-quality, token/power-saving or isolation claim is established.

Next: implement the bounded task worker/service loop and LAN adapter; then verified local execution/external boundary, graph learning/retention, measured local reasoning and optional external integration. [Architecture](docs/architecture.md), [memory](docs/amg-architecture.md), [inference](docs/inference-architecture.md), [security](docs/threat-model.md), [resources](docs/resource-budget.md).
