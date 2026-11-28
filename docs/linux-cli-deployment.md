# Current deployment and agent operations

The device runs the local/mock CLI, not the planned web agent. PR #40 source cleanup does not deploy or change host configuration. [State](../PROJECT_STATE.md) | [Roadmap](agent-roadmap.md).

## Installed layout

Installed release: e7e8fc9839090ac4cee4e6dc3140a6002124d655. Binaries live in `~/.local/lib/atomos/releases/<commit>/bin`, selected by `~/.local/lib/atomos/current`, with launchers in `~/.local/bin`. Default installation has no replay/provider enabled. Reuse existing private memory; never seed fixtures into owner data or infer permission to unlock credentials.

Existing tmux can retain a CLI across SSH disconnects. Ctrl-b then d detaches; `/quit` exits and may end the last pane. This is not durable tasks, auto-start or crash reconciliation.

## Development and release

Prepared Linux needs Rust/Cargo, compiler, pkg-config, SQLite development libraries, Python, Node and measurement tools. No speech engines, desktop stack or extra resident service. Provision dependencies explicitly, then run offline regressions and scale/recovery on the reviewed commit. Node auth tests use synthetic fixtures, not accounts or a decoder executable.

Keep source separate from installation. Fetch the reviewed commit, build a versioned release, verify hashes and smoke-test synthetic data before switching the current symlink. Retain a recoverable deployment and SSH access. Merge/deployment are separately owner-authorized. Do not copy decrypted credentials or replace issued identity to bypass accounting.

## Agent deployment requirements

Define service identity, administrative mediation, egress enforcement, resource/storage limits and task recovery. Verify LAN address/subnet, IPv4/IPv6 firewall exposure, trusted HTTPS and owner login. No public forwarding/proxy/tunnel. Startup/reconnect must not duplicate ambiguous effects.

Identify backing filesystem before durability testing; `/tmp` may be tmpfs. Use private TMPDIR on persistent disk and label RAM-backed runs separately. Bound/redact logs; keep credentials outside memory. Verify capacity/free space before capture/learning quotas. This cleanup installs no service, privileges or capture configuration.
