# AtomOS engineering instructions

Read PROJECT_STATE.md, [the MVP contract](docs/mvp-contract.md) and [the roadmap](docs/agent-roadmap.md) before work. Verify the actual checkout and current CI. Follow [architecture](docs/architecture.md), [memory](docs/amg-architecture.md), [security](docs/threat-model.md) and [resources](docs/resource-budget.md).

## Scope and authority

Build an always-running local agent on Linux, with LAN-only web interaction, broad local agency, graph learning and optional external reasoning. The CLI is maintenance. Voice, custom OS, embedded ports, public hosting, multi-user service and automatic model training are outside scope. Remove obsolete code/docs rather than retaining branch archives; use Git history when needed.

The installed runtime is local/mock. Do not claim unimplemented service, web, tools, learning, capture, local model or provider capabilities. Preserve owner data and the minimum schema/migration compatibility necessary to open existing stores; compatibility is not permission to retain removed public interfaces.

Routine authorized local actions should not repeatedly prompt. External effects require approval or a standing policy, including subprocess/remote-mount/admin effects. Protect SSH and recovery. Broad local authority does not authorize credential export, private disclosure, capture or real inference outside their concrete policy. Do not unlock owner vaults or initiate live requests without explicit owner action for the reviewed procedure. Never reset account identity or unknown usage. Human approval is required before merging runtime changes.

## Implementation and checks

- Keep production hot paths in Rust; measure bridge overhead before adding processes.
- Preserve zero incremental inference spending, bounded local operation, privacy and correctness. No unofficial access or paid fallback.
- Keep runtime independent of interface/model. Data/model output cannot grant authority.
- Freeze a reference before behavioral changes; compare equivalent inputs/state/durability and independent expected outputs/facts/usage. Never weaken assertions. Temporary Git-history references for tests are allowed; do not retain archives in the branch.
- Use synthetic offline fixtures. Never duplicate live requests/billing/side effects for verification. Run appropriate regression and scale checks on reviewed commits.
- Identify the backing filesystem before durability measurements; tmpfs is not device evidence. Preserve synchronization; report errors, counts, pending checks and exact provenance.
- Bytes are not tokens; retrieval correctness is not general model quality; process RSS is not whole-device memory/power.
- Bound logs, inputs, queues, concurrency, retries and runtime. No credentials, private prompts/facts, packet payloads or raw provider diagnostics in logs.

## Every commit

Update PROJECT_STATE accurately, including documentation-only commits. Review README and affected architecture, security, protocol, operations, resources and roadmap. Active docs contain current scope/implementation, not milestone archives. Label proposed features and unrun checks honestly.

Record exact tested commits/run links and current measurements. After validation/merge reconcile evidence at the next reviewed commit; do not fabricate a self-referential SHA or make recursive commits solely to cite themselves. Review code/docs together. Never publish secrets or private account/host identifiers.
