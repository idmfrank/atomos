# Agent architecture

One always-running local agent on existing Linux, with one owner using authenticated HTTPS restricted to the LAN. CLI is maintenance. [Roadmap](agent-roadmap.md) | [State](../PROJECT_STATE.md).

| Component | Responsibility | Status |
| --- | --- | --- |
| Task/service runtime | Durable goals, action IDs, bounded queues, cancellation, restart reconciliation | Planned |
| LAN web | Chat, progress, cancel, approvals, knowledge inspection, reconnect | Planned |
| Executor | Local policy checks, bounded tools, mediated administration, verification/recovery | Planned |
| Learning | Scoped observations, consolidation, provenance, retention/derived deletion | Planned |
| Memory | SQLite/FTS5 identity, typed facts/relationships, source/revisions, retrieval/recovery | Foundations implemented; automatic learning planned |
| Local reasoning | Commands, reusable procedures, useful local model | Commands implemented; autonomous procedures/models planned |
| External reasoning | Complete context binding, admission, approved transport, settlement | Offline foundations implemented; production integration planned |
| CLI | Local/mock text, bounded session, memory administration | Implemented |

## Task lifecycle

Submitting work assigns a durable identity. Reconnect reads existing state without resubmission; browser closure does not cancel a task. Explicit cancel prevents stale delivery and requests bounded shutdown. Restart reconciles interrupted work before retrying ambiguous effects. These are service requirements, not current CLI guarantees.

Keep web handling unprivileged. Separate administrative execution and external authority: unrestricted root cannot be claimed as confined by application checks. Protect SSH and recovery resources. Test bypass/failure cases on disposable hosts before enabling administration.

## Learning and persistence

Interactions/verified outcomes feed bounded learning queues. Host/network observations follow configured scope. Separate rotating raw evidence from graph assertions, procedures and task metadata. Keep source, time, confidence, sensitivity, retention and derivation links. Data/model output never grants authority.

Serialize writes through typed interfaces; short transactions must not cross inference or external actions. Keep SQLite/FTS5 until equivalent measurements justify replacement. Retain necessary migration/integrity compatibility for existing stores without removed disclosure interfaces.

## Boundaries

Only approved external paths may disclose context or invoke inference. Subprocesses, package managers and remote mounts also need enforcement. Credentials stay outside memory/task output/accounting. No public service, idle remote inference or automatic model training. [Security](threat-model.md), [inference](inference-architecture.md).
