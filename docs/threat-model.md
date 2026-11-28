# Security and authority

Current boundaries and target controls, not production security certification. [State](../PROJECT_STATE.md) | [Roadmap](agent-roadmap.md).

[The MVP contract](mvp-contract.md) defines approved task scope, bound external approval, privileged confinement, protected recovery and acceptance gates. No standing external grant is activated by writing it.

## Owner authority

Standing authority covers local files, processes, services and administration on the dedicated device. Routine authorized work should not prompt repeatedly. Mediate administration through a narrow executor; web must not run as root. Protect SSH, cancel and recovery. Content/model output cannot expand permissions.

External effects include outbound disclosure/requests, package downloads, Git pushes, messages, purchases, remote mounts/databases and cloud changes. Require destination/effect/data/cost approval or bounded standing policy. Approved SSH and configured authenticated LAN browser traffic need no per-packet prompts. Validate subprocess, DNS, redirect and privilege bypasses. Unrestricted root bypassing checks is not confinement.

## LAN browser target

Explicit LAN address/subnet restrictions verified for IPv4/IPv6; no public forwarding, tunnel or reverse proxy. HTTPS with trusted local certificate arrangement, owner login, protected sessions/cookies, CSRF and Host/Origin checks. Authorize streams/mutations; render output as untrusted text. No secrets in URLs/logs. Reconnect must not duplicate actions.

## Learning and observation target

Separate untrusted evidence, inferred assertions and confirmed knowledge. Preserve provenance/confidence/derivation and correction. Bound capture buffers/queues/CPU/disk/privileges, expose drops and pause on pressure. Configure packet scope/exclusions; no credentials, TLS interception or assumed whole-LAN visibility. Other-device/network changes require external authorization.

Deletion accounts for derived knowledge, indexes, caches and backups. Restore cannot silently resurrect forgotten facts. Memory does not grant execution/export authority. Private observations stay local unless approved for disclosure.

## Implemented boundaries and limits

Bounded local/mock CLI; local-only history. The separate [synthetic task store](task-runtime.md) has owner/policy binding, an OS-released exclusive SQLite lease, bounded state inspection, explicit recovery before mutation and fail-closed persistence errors. It grants no real tool or external authority. The worker supplies monotonic-anchored time, bounded commands, quiescent shutdown and fixed-recipe continuation checks; underlying Runtime caller timestamps remain rollback-checked but not authenticated. AMG owner scope, typed queries, private directories, explicit writes and migration/integrity checks. Offline admission conservatively retains unknown usage. Private encrypted auth vault with owner interaction; no live adapter or obsolete one-shot command.

Trusted filesystem/process ownership is assumed. Same-UID/root tampering, rollback and physical access are not solved. Private-memory encryption/key recovery and aggregate quota remain unqualified. Web, executor/isolation, learning/capture and provider controls need implementation/adversarial validation. No credentials, private prompts/facts, payloads or raw provider errors in logs.

## Tests

Denied/duplicate/stale actions, poisoned observations, unsafe paths/mounts, egress/privilege bypass, browser protections, cancellation, crash reconciliation, disk pressure, corruption, deletion and recovery. Use disposable hosts/synthetic data; no private vault or live inference in CI.
