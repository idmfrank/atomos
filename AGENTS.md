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

## Public repository publication boundary

Everything on GitHub is public, including commit history, PR titles/descriptions/comments, issues, Actions logs and artifacts. Publish project requirements, generic examples and hosted-CI fixture results only. Never publish target hostnames/domains/IPs, login/account identities, private paths, hardware/CPU/RAM/disk inventory, installed versions/release fingerprints, topology, private deployment logs or measurements that identify the deployment. Keep actual inventory and manifests outside the repository. Use placeholders for operations instructions. Do not copy private development-machine measurements into public evidence; use explicitly identified hosted CI results.

Before every commit/push/PR update, scan changed source/docs, metadata and any logs/artifacts to be published. The publication checker catches common accidental patterns but is not a complete privacy classifier; manual review is mandatory. Sanitize existing public descriptions when an exposure is found. Removing current text does not erase Git history, PR edit histories, caches or clones; never claim complete erasure or rewrite shared history without explicit owner authorization.
