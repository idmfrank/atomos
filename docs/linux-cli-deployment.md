# Current deployment and agent operations

Current source includes a local/mock CLI, not a web-agent service. Deployment state is private. PR #40 source cleanup does not deploy or change host configuration. [State](../PROJECT_STATE.md) | [Roadmap](agent-roadmap.md).

## Installed layout

Use a private versioned release directory, a selected-current link and owner-local launchers. Resolve actual paths and installed versions from a private deployment manifest; do not publish that manifest or target inventory. Default builds have no replay/provider enabled. Reuse existing private memory; never seed fixtures into owner data or infer permission to unlock credentials.

Existing tmux can retain a CLI across SSH disconnects. Ctrl-b then d detaches; `/quit` exits and may end the last pane. This is not durable tasks, auto-start or crash reconciliation.

## Development and release

Prepared Linux needs current Rust/Cargo (task library minimum 1.77), compiler, pkg-config, SQLite development libraries, Python, Node and measurement tools. No speech engines, desktop stack or extra resident service. Provision dependencies explicitly, then run offline regressions and scale/recovery on the reviewed commit. Node auth tests use synthetic fixtures, not accounts or a decoder executable.

Keep source separate from installation. Fetch the reviewed commit, build a versioned release, verify hashes and smoke-test synthetic data before switching the current symlink. Retain a recoverable deployment and SSH access. Merge/deployment are separately owner-authorized. Do not copy decrypted credentials or replace issued identity to bypass accounting.

## Agent deployment requirements

Define service identity, administrative mediation, egress enforcement, resource/storage limits and task recovery. Verify LAN address/subnet, IPv4/IPv6 firewall exposure, trusted HTTPS and owner login. No public forwarding/proxy/tunnel. Startup/reconnect must not duplicate ambiguous effects.

Identify backing filesystem before durability testing; `/tmp` may be tmpfs. Use private TMPDIR on persistent disk and label RAM-backed runs separately. Bound/redact logs; keep credentials outside memory. Verify capacity/free space before capture/learning quotas. This cleanup installs no service, privileges or capture configuration.
