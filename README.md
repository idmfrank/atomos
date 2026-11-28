# AtomOS

An always-running local agent with a LAN browser interface, persistent graph memory, broad control of its dedicated Linux device and optional external reasoning.

AtomOS will learn from owner interactions, verified task results and scoped system/network observations. Linux supplies the operating system. Shared Rust interfaces own tasks, memory, permissions, execution and accounting; model reasoning is an optional resource. External effects require approval or an approved standing policy. Target zero incremental inference spending with no paid fallback.

[Roadmap](docs/agent-roadmap.md) | [Architecture](docs/architecture.md) | [Current state](PROJECT_STATE.md) | [Memory](docs/amg-architecture.md) | [Security](docs/threat-model.md)

## Current implementation

- Local/mock text CLI and bounded sessions; the CLI remains a maintenance interface.
- SQLite/FTS5 memory graph with owner scope, typed facts/relationships, source IDs, correction, deletion, scoped retrieval, integrity checks and logical recovery.
- Offline inference broker, strict/extended response decoding, durable request-bound admission, usage settlement and conservative handling of unknown usage.
- Explicit [owner-run authentication utility](docs/owner-auth.md) with encrypted credentials and model discovery. No production inference connection.

The persistent task service, LAN web interface, autonomous executor, learning pipeline, packet observation and local model are not yet implemented. Mock responses are not evidence of intelligence. Source cleanup removes voice runtime/adapters and obsolete one-shot inference tools; voice communication is outside the project. Existing data formats remain readable without rewriting owner stores.

## Product contract

- One owner, one dedicated Linux device and initially one active task.
- Browser access within the local network only, with HTTPS, owner authentication, explicit address/subnet restrictions and browser request protections.
- Continuous availability independent of browser connections, bounded event-driven learning and safe restart reconciliation.
- Standing local administrative authority, mediated privileges, verified outcomes, cancellation and protected SSH/recovery.
- Approval or a bounded standing policy before external effects/disclosures, including inference and tool-initiated network activity.
- Provenance-aware graph learning, inspect/correct/forget controls, rotating observations and deletion of derived knowledge.
- Local rules, retrieval and useful benchmark-selected local models before external reasoning. No idle remote inference or automatic model training.

Use the existing host. The owner reports 2 TB storage; verify capacity, free space and filesystem before quotas. More disk does not justify indefinite packet/transcript retention. Learn first from interactions and verified outcomes, then host events and own connection metadata. Packet content needs configured scope, exclusions, privileges and retention; do not assume whole-LAN visibility.

## Build and offline checks

On prepared Linux with Rust, SQLite development libraries, pkg-config, Python and Node:

```bash
cargo fetch --locked
bash scripts/test.sh
bash scripts/test-responses.sh
node --test scripts/auth/protocol.test.mjs
bash scripts/test-atomic-admission.sh
bash scripts/test-session.sh
```

Provisioning is explicit. Tests do not need real credentials, inference or capture privileges. See [resources](docs/resource-budget.md) and [operations](docs/linux-cli-deployment.md). Evidence for the final candidate belongs in PROJECT_STATE.

## Maintenance CLI

```bash
cargo run --locked --offline -p atomos-cli --bin atomos -- --session
```

Use `/help`, `/status`, `/clear`, `/quit`. Default is local/mock. Explicit memory changes use atomos-memory; conversation memory is read-only. [Text contract](docs/cli-session-contract.md).

## Delivery

Follow the [roadmap](docs/agent-roadmap.md): cleanup, durable tasks/service contract, LAN browser, verified local execution, continuous graph learning, measured local reasoning, optional external augmentation and sustained acceptance.

Voice, custom kernels, public hosting, multi-user operation, unbounded surveillance and automatic model training are outside scope. Repository history holds prior implementations/evidence; active documentation describes the current project only.

## Current validation

Current main `ed73c6b` passed [post-merge offline CI](https://github.com/idmfrank/atomos/actions/runs/46100497944): 174 distinct tests, 18,000 decoder samples, core Rust 1.75 and accounting/session benchmarks. [Full 1K/10K/100K graph scale/recovery](https://github.com/idmfrank/atomos/actions/runs/46100262290) passed with no parity/recovery errors or remote calls. [Exact provenance and scope](PROJECT_STATE.md#validation). This evidence update changes documentation only; planned service/model/learning capabilities remain unqualified.
