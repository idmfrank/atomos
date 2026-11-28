# AtomOS

An always-running local agent with a LAN browser interface, persistent graph memory, broad control of its dedicated Linux device and optional external reasoning.

AtomOS will learn from owner interactions, verified task results and scoped system/network observations. Linux supplies the operating system. Shared Rust interfaces own tasks, memory, permissions, execution and accounting; model reasoning is an optional resource. External effects require approval or an approved standing policy. Target zero incremental inference spending with no paid fallback.

[MVP contract](docs/mvp-contract.md) | [Roadmap](docs/agent-roadmap.md) | [Architecture](docs/architecture.md) | [Current state](PROJECT_STATE.md) | [Memory](docs/amg-architecture.md) | [Security](docs/threat-model.md)

## Current implementation

- Local/mock text CLI and bounded sessions; the CLI remains a maintenance interface.
- [Durable tasks and worker](docs/worker-service.md): persistent identities, bounded queue, intent/effect/verification, cancellation and recovery, with a synthetic actor and supervised local control process. No real tools or listener.
- SQLite/FTS5 memory graph with owner scope, typed facts/relationships, source IDs, correction, deletion, scoped retrieval, integrity checks and logical recovery.
- Offline inference broker, strict/extended response decoding, durable request-bound admission, usage settlement and conservative handling of unknown usage.
- Explicit [owner-run authentication utility](docs/owner-auth.md) with encrypted credentials and model discovery. No production inference connection.

The task library/worker is separate from the text CLI. The LAN web interface, autonomous executor, learning pipeline, packet observation and local model are not yet implemented. The task service executes a fixed counter fixture, not natural-language goals. Mock responses are not evidence of intelligence. Source cleanup removes voice runtime/adapters and obsolete one-shot inference tools; voice communication is outside the project. Existing data formats remain readable without rewriting owner stores.

## Product contract

- One owner, one dedicated Linux device and initially one active task.
- Browser access within the local network only, with HTTPS, owner authentication, explicit address/subnet restrictions and browser request protections.
- Continuous availability independent of browser connections, bounded event-driven learning and safe restart reconciliation.
- Standing local administrative authority, mediated privileges, verified outcomes, cancellation and protected SSH/recovery.
- Approval or a bounded standing policy before external effects/disclosures, including inference and tool-initiated network activity.
- Provenance-aware graph learning, inspect/correct/forget controls, rotating observations and deletion of derived knowledge.
- Local rules, retrieval and useful benchmark-selected local models before external reasoning. No idle remote inference or automatic model training.

Use the existing host. Determine capacity, free space and filesystem privately before configuring quotas. Do not publish deployment inventory. More disk does not justify indefinite packet/transcript retention. Learn first from interactions and verified outcomes, then host events and own connection metadata. Packet content needs configured scope, exclusions, privileges and retention; do not assume whole-LAN visibility.

## Build and offline checks

On prepared Linux with current Rust (new task library minimum 1.77), SQLite development libraries, pkg-config, Python and Node:

```bash
cargo fetch --locked
bash scripts/test.sh
bash scripts/test-responses.sh
node --test scripts/auth/protocol.test.mjs
bash scripts/test-atomic-admission.sh
bash scripts/test-session.sh
bash scripts/test-task-service.sh
```

Provisioning is explicit. Tests do not need real credentials, inference or capture privileges. See [resources](docs/resource-budget.md) and [operations](docs/linux-cli-deployment.md). Evidence for the final candidate belongs in PROJECT_STATE.

## Maintenance CLI

```bash
cargo run --locked --offline -p atomos-cli --bin atomos -- --session
```

Use `/help`, `/status`, `/clear`, `/quit`. Default is local/mock. Explicit memory changes use atomos-memory; conversation memory is read-only. [Text contract](docs/cli-session-contract.md).

## Delivery

The [MVP contract](docs/mvp-contract.md) defines supported journeys, local/external authority, durable tasks, retention, initial work bounds and acceptance gates A01-A13. It is a specification; these agent capabilities and acceptance tests are not implemented. The first MVP must work locally without a provider connection.

Follow the [roadmap](docs/agent-roadmap.md): cleanup, durable tasks and synthetic worker/service, LAN browser, verified local execution, continuous graph learning, measured local reasoning, optional external augmentation and sustained acceptance.

Voice, custom kernels, public hosting, multi-user operation, unbounded surveillance and automatic model training are outside scope. Repository history holds prior implementations/evidence; active documentation describes the current project only.

## Current validation

The worker/service implementation adds bounded-channel, worker lifecycle and process coverage; the expected distinct suite is 210 tests. Remote offline regression/process checks passed, including signal/EOF/forced-death recovery, output backpressure, duplicate prevention, 18,000 decoder samples and supervisor-template syntax. Candidate hosted CI, minimum-compiler gates and full graph scale/recovery are pending. Only hosted runner measurements may be published. No real-worker tool, model, LAN or deployment qualification is implied. [Current scope and progress](PROJECT_STATE.md).
