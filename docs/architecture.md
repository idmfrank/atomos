# AtomOS architecture

**Current development direction:** [Local agent and LAN browser interface](local-agent-direction.md) supersedes the CLI-only target below. A shared Rust runtime should own tasks, permissions and verification; a LAN-restricted authenticated web adapter provides interaction. These are proposed components, not deployed wiring.

Status: persistent local agent and LAN web target adopted 2026-11-27. The existing text runtime is local/mock; M2.5a admission and decoder profiles remain isolated synthetic components. M2.5b is merged at `e3ebf24`; post-merge CI and full scale on the identical final PR tree passed. [Exact provenance](m2-text-outcomes.md#verified-candidate-evidence). Voice is removed from active scope. [Roadmap](agent-roadmap.md) | [Current state](../PROJECT_STATE.md) | [README](../README.md) | [Threat model](threat-model.md) | [Resources](resource-budget.md) | [Inference](inference-architecture.md) | [AMG](amg-architecture.md)

## System boundary

The active target is one always-running local agent on Linux, accessed by one owner through an authenticated HTTPS interface restricted to the LAN. Shared Rust interfaces own task state, local permission checks, memory, tool execution, verification and optional inference. The CLI remains for maintenance. Browser lifetime is independent of task lifetime. [Active stages](agent-roadmap.md).

Interaction and outcome events feed bounded learning queues. Consolidation produces graph assertions with provenance and confidence, never instructions or silently trusted facts. Raw observations rotate separately from durable graph knowledge. Scoped own-host network metadata is planned; packet content requires configured exclusions, retention and capture privileges. No whole-LAN visibility is assumed.

The web process is unprivileged; administrative effects pass through a reviewed executor. External-effect policy covers subprocesses, remote storage and network use; unrestricted root execution cannot be claimed as confined by an application-only check. No web service, executor, isolation, observer or local model is implemented by this cleanup. Installed behavior remains the local/mock CLI.

## Runtime boundaries and proposed interfaces

| Component | Responsibility and status | Boundary |
| --- | --- | --- |
| Service/web adapter | Planned authenticated LAN interface and durable task interaction | Browser disconnect does not cancel or duplicate tasks; new implementation required |
| Learning pipeline | Planned observation ingestion, consolidation and retention | Separate evidence from graph assertions; provenance, bounded storage and derived deletion |
| Executor | Planned local execution and mediated administration | Policy checks and verification; no unrestricted egress bypass |
| CLI adapter | Terminal input/output, command parsing, exit/status and cancellation signals | Thin adapter; one owner and active session; [session contract](cli-session-contract.md) merged PR #38 and deployed local/mock session |
| Conversation core | Turn/task ownership and bounded working context | Reject stale delivery; no full transcript persistence by default |
| Memory | Identity, explicit facts, correction/forget/export and local retrieval | Typed APIs; short transactions; separate current administration CLI |
| Local resolver | Deterministic supported commands and fact queries | No inference charge; expose unsupported/ambiguous states honestly |
| Context compiler | Bounded relevant context and trust/provenance labels | Private AMG provider disclosure stays separately gated |
| Admission and broker | Account/request identity, reservation, outcomes and settlement | One durable admission before response read; no detachable public permit |
| Provider adapter | Future authorized text transport with bounded lifetime | Separate security/spending/time/credential gate; no new live authorization |

Keep existing Rust crate/interface boundaries where useful; choose privilege/process separation for the agent from explicit security and resource requirements. Existing SSH authenticates host access, not provider disclosure or multi-user isolation. Optional existing `tmux` keeps a process running across disconnects but adds no AtomOS durable session recovery or automatic credential unlock.

## CLI conversation lifecycle

1. Wait for a submitted terminal line without idle remote calls.
2. Validate turn/task/input and resolve supported local commands or facts.
3. For unresolved conversation, build bounded in-session context; use mock/synthetic responses until an approved provider adapter exists.
4. Validate a fresh admission context, persist reservation and decode the selected text outcome. Preserve accounting and privacy policy regardless of delivery style.
5. Display only output permitted by the reviewed outcome contract. Current synthetic parsing buffers until terminal validation/settlement; incremental terminal streaming is not implemented by this roadmap.
6. On cancellation, EOF, disconnect or errors, stop stale delivery and expose local status. Define signal/exit behavior in the CLI contract. A disconnect/socket close does not prove upstream generation or usage stopped.
7. Persist explicit memory commits and accounting outcomes. Restart restores identity and committed facts, not necessarily volatile working turns. No automatic retry, transcript archive, reminders or background consolidation.

## Verified Linux deployment profile

The historical initial [owner-local CLI deployment](linux-cli-deployment.md) was verified at PR #34 main `5869e205167a3dde3a6a77392af294101e6a46c3`. One owner account holds clean source, tested versioned binaries and a private generic identity store; one local/mock foreground CLI runs in tmux. No service autostart, browser/audio stack, extra inference model or production provider path is introduced. Build/test tools are separate from running CLI dependencies.

All 173 host tests, 18,000 decoder samples and SSD-backed 1K/10K/100K memory recovery passed. At 100K, selective context p95 0.3974 ms and incremental CLI RSS 1,448 KiB meet existing targets. Keep the existing Rust/SQLite architecture and Linux foundation; no rewrite or enlarged resource allocation is justified. The ordinary CLI remains local/mock and read-only against AMG; explicit mutations use its separate administration binary. Private/provider gates remain unchanged.

## M2.5b contract boundary

The [offline text-outcome contract](m2-text-outcomes.md) specifies explicit extended atomic replay with Local/Completed/Refused/Failed final results. It reuses existing decoder profiles, immutable request identity, fresh context and v1 accounting records; preserves the strict reference; and adds outcome/cancellation/persistence-fault tests. The implementation and required offline evidence passed on PR #36, merged with successful post-merge CI. No CLI command/session wiring, transport, incremental display or private memory access is added by the contract.

## CLI session boundary

The [session contract](cli-session-contract.md) defines an opt-in synchronous terminal adapter while preserving legacy launch modes. Help/status/clear/quit commands remain local; memory mutations stay in atomos-memory. Retain at most four completed delivered pairs/16 KiB of local-only text; do not feed that history into the current-question digest or claim it is provider-bound context. Explicit replay is isolated from AMG. A read-only atomic status snapshot distinguishes pending/halted/unhealthy without exposing a permit or response data. Default Ctrl-C terminates the process and may leave a stale replay lock; graceful transport cancellation remains a later gate. The shared Rust session and thin adapter now implement this boundary on PR #38; full offline verification passed, and the owner-authorized default-feature deployment is verified. Optional replay is not installed; provider integration remains disabled.

## State, persistence and concurrency

M2.5a [atomic admission](m2-atomic-admission.md) owns a private bounded ledger and one outstanding reservation. Unknown outcomes remain charged and block further synthetic admission; revocation/overrun suspension are durable. The implementation is not connected to conversational AMG, OAuth or a network. Real crash locks require manual investigation; clean reopen tests do not qualify automatic crash recovery.

One conversation owner manages current turn/task and bounded working context. Serialize memory writes through typed interfaces; never hold an AMG transaction across inference. SQLite + FTS5 and existing recovery formats remain the reference. Keep secrets out of memory and accounting journals. Terminal/session buffers and redirected output can retain text; do not add transcript/secret logging.

Persistent service operation, local tools, web access and learning are planned under the active agent roadmap. Concurrent conversations remain outside the initial one-owner/one-task scope.

## Voice removed from product scope

The owner removed voice from the roadmap. Audio buffers, voice coordinator, native adapter harness and CLI event protocol are removed from current source. Shared turn epochs, cancellation and text interfaces remain. Legacy AMG grant schema and retirement/migration integrity logic remain solely to preserve existing-store compatibility; grant commands and voice recall API are removed. No owner store is rewritten.

## Historical first vertical slice (M1)

The original voice-first vertical slice included mock responses, interruptible playback, event timestamps and physical microphone/speaker evaluation. These acoustic exit criteria are removed from active scope. Agent acceptance is defined in [the roadmap](agent-roadmap.md); historical audio qualification was incomplete.

## Preserved implementation evidence

[M1.4b post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45755478646): 35 Rust + 9 Python tests, smoke checks and no-store text benchmark passed. At that historical revision, the Rust core provided mock/local inference, turn epochs, bounded PCM, RMS endpointing and IPC. Optional warm Sherpa-ONNX/Kokoro adapter defaults to half-duplex; real speech engines and AEC remain unvalidated.

AMG v1 implements persistent local identity, scoped entities and typed assertions/edges, recorded-time revision history, transactional correction/deletion and FTS5. The explicit administration CLI owns mutation. M1.4b adds MemoryReader and a SQLite read-only connection to the text and voice host. Exact lookup only in conversation; follow-up retains a subject key, never a cached value. Final-turn validation precedes lookup; no database operation on microphone callback. Voice demo allowlist is not authentication. No implicit store creation/migration or remote inference. See [AMG foundation](m1-amg-foundation.md) and [conversational integration](m1-amg-conversation.md).

M1.4c–f subsequently implemented scoped graph/FTS context, revision-specific demo voice grants, schema v3 deletion and bounded streaming logical recovery; see [AMG optimization](m1-amg-optimization.md). Private authenticated memory, encrypted recovery and aggregate quota remain gates. M2.1–M2.4c add separate offline broker, authorization, decoder profiles and owner-run auth/qualification utilities; no integrated live broker or private AMG egress. Voice features are now removed; persistent task/service and learning work follows the active agent roadmap. The fixed-budget governor journal remains separate from AMG. See [AMG architecture](amg-architecture.md) for target requirements.

