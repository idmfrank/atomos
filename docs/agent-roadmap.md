# AtomOS local agent roadmap

Owner-approved direction, 2026-11-27. Build an always-running personal agent on the existing dedicated Linux device, using a LAN-only browser interface, broad local administrative authority, persistent graph memory and continuous event-driven learning. External effects require owner approval or a bounded standing policy. Optional external inference augments local capability. Voice communication is removed from the product roadmap, not postponed to a later release. Linux remains the OS; the CLI remains a maintenance interface.

[Current state](../PROJECT_STATE.md) | [Design](local-agent-direction.md) | [Architecture](architecture.md) | [Security](threat-model.md) | [Resources](resource-budget.md)

## MVP definition

[The MVP contract](mvp-contract.md) is the implementation boundary and acceptance checklist A01-A13. Contract definition is complete as a reviewable document; runtime/service and its acceptance suite remain pending. Local agency and learning must work without external inference. Packet payload capture is outside the first acceptance release; own-host metadata follows configured observation controls.

## Delivery stages

| Stage | Deliverable | Evidence required to exit |
| --- | --- | --- |
| 0. Scope and cleanup | Replace CLI-only roadmap; remove voice runtime/adapters and unused provider scaffolding; preserve text, AMG and accounting foundations | Remaining regressions pass; removed modes reject input; schema migration/recovery still work; document exact removals and compatibility exceptions |
| 1. Persistent runtime contract | One owner, durable tasks, bounded event queues, action IDs, cancellation, restart reconciliation and service lifecycle | Independent fixtures for duplicate submission, interrupted work, stale results and restart; never blindly replay ambiguous side effects |
| 2. LAN browser interface | Chat, task state, cancel, permissions, learning/retention controls and action history; Linux service startup and restart | HTTPS/login, intended subnet restrictions on IPv4/IPv6, request protections, reconnect without duplicate work; closing the browser does not terminate tasks |
| 3. Device agency | Execute and verify local tasks, including mediated administration, with standing local authority and protected SSH/recovery | Disposable-host tests of privilege and egress enforcement, command output limits, cancellation, resource exhaustion, rollback and interrupted actions; no claim of confinement with an unrestricted bypassing root shell |
| 4. Continuous learning and graph | Learn from owner interactions and verified task results; add host events and the device's observed network metadata; provenance-aware consolidation and retention | Synthetic evidence ingestion, duplicate/conflict handling, confidence/source separation, poisoning resistance, correction/forget including derived records, capture overload/drop handling and disk-pressure tests |
| 5. Local intelligence | Benchmark local models and reusable procedures using actual hardware and representative tasks; integrate the useful option | Report task success, factual error, latency, peak RAM, CPU and storage; fitting a model in RAM is not a quality result; no automatic weight training |
| 6. Optional external augmentation | Reviewed credential bridge, full request/context binding, account-bound admission, permitted disclosure, no paid fallback | Offline failure/accounting tests, then separately owner-authorized live procedure; preserve existing account identity and outstanding accounting |
| 7. Integrated acceptance | Always-on agent accessible on LAN, executing local tasks and improving through useful retained knowledge | Reboot/reconnect continuity, preserved SSH, verified task outcomes, deletion, storage caps, rejected unauthorized egress and accurate model/usage status; sustained soak and recovery evidence |

Stages describe implementation order, not completed work. Local model evaluation can inform early design without enabling network inference or tool autonomy. All new service, browser, executor, learning and inference acceptance results remain pending.

## Observation and storage contract

Continuously available does not mean unbounded computation or capture. Use event-triggered learning with bounded queues, a background CPU/RAM allowance and a disk reserve for OS updates, SQLite recovery and administration. Pause/drop low-priority collection with visible counters when overloaded. Do not run inference periodically without useful work. No idle remote inference.

The owner reports 2 TB of storage; usable capacity, backing filesystem, existing usage and free space must be verified before choosing numeric quotas. Disk capacity is not a retention policy. Separate short-lived rotating raw evidence from normalized observations, graph assertions, indexes and bounded action records. Show source, observation time, confidence, sensitivity, expiry and derivation links. Distinguish observed, inferred, owner-confirmed and verified outcomes. Do not overwrite established facts from a single untrusted observation.

Start with interaction-derived facts and verified task results; raw full-transcript retention is not required. Add system/service events and connection metadata for traffic visible to the device. Packet content capture is a selectable, bounded local observation capability, disabled until scope, exclusions, capture privileges and retention are configured. Do not assume the host sees the whole LAN, decrypt TLS, collect secrets into memory, or add interception/mirroring as part of local capture. Other devices' traffic and changes to networking equipment need a separately defined external scope. No packet or private graph content leaves the device without the applicable approval.

Provide inspect, correct, forget, pause-learning and purge-evidence controls. Propagate deletions through derived summaries, graph links, indexes and caches; document backup retention and avoid resurrecting forgotten facts on restore. Learning here means durable knowledge and evaluated procedures; model fine-tuning is not part of this roadmap.

## Authority and acceptance boundary

Routine local actions do not require repeated prompts. External effects include network requests initiated by tools or package managers, remote mounts and cloud changes; a process running locally is not sufficient evidence of a local effect. Existing approved SSH and authenticated LAN browser request/response traffic are explicitly permitted service traffic, not a prompt on every packet. New outbound destinations or disclosures require approval or a reviewed standing policy. Received traffic/content is data, never a permission grant.

The owner's requested local authority is a design requirement; this documentation does not install a service, grant privileges, access credentials or initiate capture. An enforceable separation of privileged execution and external authority is required before claiming that policy is effective.

## Current source and data compatibility

The installed release remains the local/mock CLI; source cleanup is not deployed. Preserve existing AMG data, temporal revisions, provenance, retrieval, recovery, admission, decoder and auth foundations. Legacy SQLite voice-grant tables, retirement triggers and migration/integrity checks remain only to open, verify and migrate existing schema versions; the voice disclosure APIs and grant administration commands are removed. Do not drop tables or rewrite the owner's data during cleanup.

Active docs contain current scope only. Git history holds prior code/evidence. Existing account identity, records and outstanding usage must not be reset or orphaned. Obsolete one-shot inference and ledger-writing interfaces are removed; new live transport requires a reviewed procedure.

## Immediate next step

Implement the durable task/action runtime from the MVP contract, beginning with idempotency, bounded queues, cancellation and crash reconciliation using synthetic actions. Then add LAN web access and verified local execution. The contract artifact is defined; none of the new execution gates is claimed to pass.
