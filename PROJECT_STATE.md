# AtomOS current engineering state

Updated 2026-11-27. Active work: define the first local-agent MVP contract. Base main is b5ae35268d7474044ea2df46db2cde1cb7c387b6 (PR #41 merged), with successful [post-merge CI 46100924573](https://github.com/idmfrank/atomos/actions/runs/46100924573). This change is documentation only; no runtime, dependency, host, credential or deployment changes.

## MVP contract slice

[The MVP contract](docs/mvp-contract.md) defines supported local journeys, ordinary-language quality, standing local authority versus bound external approvals, durable task/action states, cancellation/crash reconciliation, learning/provenance/retention, mandatory resource configuration and gates A01-A13. The first MVP must function locally without external inference; packet payload capture is a later enhancement. Proposed initial defaults include one active task, 16 waiting/queued tasks, 32 dispatches/10 active minutes per task, bounded I/O/observation queues, 24-hour raw observation rotation and 30-day completed task retention. These are specification targets, not implemented or measured behavior. Disk/CPU/RAM activation limits require device calibration.

Step 1 is complete as a concrete reviewable contract, not an executed acceptance suite. Next is the shared durable task/action runtime with synthetic fixtures for identity, limits, cancellation and crash reconciliation, followed by LAN web and real executor integration. Review updated README, roadmap, architecture, memory, security, resources and engineering instructions; existing inference, owner-auth and operations docs need no behavioral change because this slice does not activate their interfaces. Contract validation passed: 14 active documents and local links/anchors/fences, all thirteen acceptance IDs, ten two-phrase task fixtures, documentation-only scope and whitespace checks. Existing runtime regression results remain unchanged; no runtime tests or device benchmarks were rerun for this specification-only slice.

## Publication privacy

Deployment inventory, installed-version fingerprints and private environment measurements are excluded from public documentation and PR text. This documentation/checker change removes current public references, adds a publication rule and checks common accidental disclosures before CI proceeds. It does not erase earlier Git/PR revisions or external copies. No history rewrite, deployment or runtime change. Current-tree link/fence checks, publication-checker self-tests and source scan, and whitespace passed. The private audit found one older PR description needing sanitization, which was updated; issue/review metadata are being checked separately. No private inventory is included in the checker itself. This privacy-only PR has its own CI pending.


## Product and implementation

One owner, continuous Linux service, LAN browser interaction, broad local agency and graph-based learning. Optional external reasoning supplements local capability with zero incremental inference spending. External effects require approval or standing policy. [Roadmap](docs/agent-roadmap.md).

Implemented: local/mock text CLI, bounded sessions, SQLite/FTS5 typed graph with retrieval/correction/deletion/recovery, offline accounting/decoder/broker foundations and owner auth/model discovery. Service, web, autonomous executor, learning/capture, local models and production inference remain unimplemented.

Cleanup removes voice/audio components and public interfaces, obsolete one-shot inference tools and ledger writers, and archived documentation. Auth is auth/catalog only. Existing accounting files are untouched; a missing host cannot be initialized in a nonempty credential directory. Required SQLite schema/migration/integrity compatibility remains without removed voice APIs.

## Current deployment

Deployment versions and inventory are maintained privately. Source changes do not authorize deployment, host configuration, credential access, capture or privilege changes. [Operations](docs/linux-cli-deployment.md).

## Validation

Current runtime source: `80f66cea92aff4656d817a67bd9992d47396c0db`. [PR CI 46100265296](https://github.com/idmfrank/atomos/actions/runs/46100265296) passed on synthetic merge `fe80c4b4744fcf6dd1f02716244a3b2f5081715e`; [full graph scale/recovery 46100262290](https://github.com/idmfrank/atomos/actions/runs/46100262290) passed on the branch source. The merged main tree exactly matches that validated runtime source. [Post-merge CI 46100497944](https://github.com/idmfrank/atomos/actions/runs/46100497944) passed on main ed73c6b40358ac352c38bee6144c6c207c2b4442. This evidence update changes documentation only; no runtime or dependency changes.

All 174 distinct tests passed: 137 Rust, 17 Python and 20 Node. Repeated feature/compiler runs and the ignored child-only crash fixture are not extra tests. Passed: workspace/CLI smoke, memory persistence/crash recovery, text-reference parity, removed-interface rejection, auth/state guards, 18,000 decoder samples with zero errors, admission/outcome/session accounting and overhead checks, Rust 1.75 core all-targets, and 1K CI graph smoke. Full 1K/10K/100K graph run returned 300/300 expected facts and 100 complete packets at each size, zero parity mismatches, zero recovery-verification errors and zero remote calls. Hosted synthetic results do not qualify new agent/model capabilities or device power.

Local WSL regression and Node syntax checks also passed. All 13 active Markdown files passed local path/anchor/fence checks; whitespace passed. No service/model/learning/security qualification is claimed. Current main post-merge CI passed as linked above. This contract-only change has its own CI pending at authoring; no additional runtime benchmark or live qualification is implied.

## Constraints and next work

Deployment inventory is private. Verify free/usable capacity, filesystem and headroom privately before quotas/model selection. No new model-quality, token/power-saving or isolation claim is established.

Next: implement the durable task/action runtime against the defined MVP contract; then LAN web, verified local execution/external boundary, graph learning/retention, measured local reasoning and optional external integration. [Architecture](docs/architecture.md), [memory](docs/amg-architecture.md), [inference](docs/inference-architecture.md), [security](docs/threat-model.md), [resources](docs/resource-budget.md).
