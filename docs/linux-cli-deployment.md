# Linux CLI development and local deployment

Status: verified local/mock deployment, 2026-11-27, at source `5869e205167a3dde3a6a77392af294101e6a46c3` (merged PR #34). [Main CI 46055361823](https://github.com/idmfrank/atomos/actions/runs/46055361823) passed. The source checkout, release binaries, local identity store and terminal session were verified directly on the owner-authorized Linux host. This is deployment of the existing CLI, not implementation of M2.5b or production inference.

[Current state](../PROJECT_STATE.md) | [CLI roadmap](cli-mvp-roadmap.md) | [Architecture](architecture.md) | [Resource evidence](resource-budget.md) | [M2.5b contract](m2-text-outcomes.md)

## Account and software boundary

Use the owner's explicitly selected unprivileged Unix account for source, builds, binaries and data. Administrative access is used only for necessary system dependencies. Verify SSH key selection and the remote account; a saved SSH alias can select a different key/account from a bare hostname. Authorizing an existing public key for the selected account does not require copying private keys, changing sshd configuration or granting additional sudo privileges. Keep hostnames, addresses, account names and credentials out of public deployment records.

The reference host is headless Ubuntu 26.04 LTS, Linux 7.0.0-44, Intel i7-6700 (4 cores / 8 threads), 16 GB RAM and a SATA SSD with ext4-backed owner directories. Reuse existing hardware and OS. No desktop, browser, audio stack, GPU framework, local LLM, npm ecosystem or new resident application service is required.

| Purpose | Required software / verified choice |
| --- | --- |
| Rust compilation | Existing Rust/Cargo/rustdoc 1.95.0 reused; no second compiler installed |
| Native linking | Existing GCC 15.2, make and glibc development files |
| SQLite build discovery | pkg-config and libsqlite3-dev; SQLite 3.46.1 with FTS5 |
| Source and provisioning | Existing Git, curl and CA certificates |
| Regression/measurement | Existing Python 3.14, GNU time, coreutils and strace |
| Offline auth fixture tests | Node 22.22.1 from Ubuntu packages; all 43 fixture tests passed |
| SSH session continuity | tmux 3.6a; an idle terminal session, not a daemon/autostart service |
| Running CLI | Tested Rust binaries, SQLite runtime, libgcc_s, libc, libm and the system loader; compiler/Node/Python are not resident CLI workers |

Only missing tools were added, using `--no-install-recommends`. On the inspected host the requested additions were `pkg-config libsqlite3-dev nodejs tmux`; required dependencies included a matching SQLite runtime patch update. A host lacking a C linker or development files needs those provisioned separately. Do not install duplicate toolchains just to match a historical CI image. This host's 1.95 build is distinct from older 1.100 CI measurements; historical Rust 1.75 core checks are not a claim of whole-workspace compatibility or a fresh 1.75 run on this device.

## Build and verification

Inspect AGENTS, PROJECT_STATE, the working tree and actual reviewed main before changing a checkout. Preserve unrelated edits. Use a dedicated owner checkout such as `~/atomos`; clone the public source only if the target does not already exist. Pin and record the tested commit. Build as the owner, never root.

Use the selected compiler's bin directory and `~/.local/bin` on PATH. The reference host reuses its existing toolchain directory through its login profile; no runtime needs that compiler path. Use up to `nproc` build jobs when memory permits; eight jobs were used here. Do not run CPU-intensive builds concurrently with timing comparisons.

After explicit locked dependency provisioning, existing commands are:

```bash
cargo fetch --locked
bash scripts/test.sh
bash scripts/test-responses.sh
cargo build --release --locked --offline -p atomos-core --features responses-replay --example responses_qualify
node --test scripts/auth/protocol.test.mjs scripts/auth/live-probe.test.mjs
bash scripts/test-atomic-admission.sh
cargo build --workspace --release --locked --offline
bash scripts/benchmark.sh
```

The release decoder executable is a prerequisite for Node replay tests. These scripts use synthetic fixtures, including fake voice workers; preserving their regression coverage does not require speech models or hardware.

All 173 distinct baseline tests passed on the host: 119 Rust, 11 Python and 43 Node. All 18,000 decoder samples passed. Repeated feature executions are not extra distinct tests. The source checkout stayed clean; no runtime, dependency lock or workflow changes were needed.

## Persistent-storage benchmark procedure

Ubuntu's default `/tmp` on this host is tmpfs. The initial default-temp run passed its logical checks but measures RAM-backed storage; it is not the SSD durability/performance baseline. Keep that evidence labeled separately. A successful fsync on tmpfs is not evidence of durable device storage.

For durable admission and AMG scale measurements, create a private temporary parent on the same persistent filesystem as the intended data, inspect its backing filesystem, and set TMPDIR only for the benchmark:

```bash
install -d -m 700 "$HOME/.local/state/atomos/bench-tmp"
findmnt -n -o FSTYPE -T "$HOME/.local/state/atomos/bench-tmp"
TMPDIR="$HOME/.local/state/atomos/bench-tmp" bash scripts/benchmark-admission.sh
TMPDIR="$HOME/.local/state/atomos/bench-tmp" AMG_SAMPLES=100 AMG_RECOVERY=1 bash scripts/benchmark-amg.sh 1000 10000 100000
```

The verified run used ext4 on the SATA SSD; `stat -f` reports its shared ext2/ext3 family name. Tests and benchmark temp stores are separate from the owner's identity store. No fsync, SQLite durability, policy or correctness check was disabled. Controller behavior and actual power-loss recovery are not certified.

Local logs are owner-private under `~/.local/state/atomos/verification-20261127-214716/`: `results.json`, eight stage logs, `ssd-results.json`, `ssd-admission.log` and `ssd-full-amg-scale.log`. These are direct host observations, not GitHub Actions results. [Sanitized measurements](resource-budget.md#verified-linux-cli-host-baseline).

## Installed release layout and smoke checks

The verified setup uses:

- Source: `~/atomos`, on clean main at the tested SHA.
- Immutable release: `~/.local/lib/atomos/releases/TESTED_SHA/bin/{atomos,atomos-memory}`.
- Current release: `~/.local/lib/atomos/current`, an atomically replaced symlink.
- Launchers: `~/.local/bin/atomos` and `~/.local/bin/atomos-memory`, pointing through current.
- Local data: private `~/.local/share/atomos/memory`, initialized once with generic agent name AtomOS and the selected owner scope.
- Local deployment record: `~/.local/share/atomos/deployment.json`, containing tested SHA, binary SHA-256 digests, versions, mode, owner scope and verification location.

The installed binaries' SHA-256 digests match the tested release outputs. The CLI is 479,672 bytes and the memory administration binary 462,280 bytes on this toolchain. The default release profile remains opt-level=s, LTO, one codegen unit and stripping; no machine-specific CPU flags or alternate build profile were introduced.

Initialize only a new store; verify an existing one without overwriting or migrating it. In generic commands, ATOMOS_OWNER must be the exact scope chosen at initialization, not an inferred login name:

```bash
# Set ATOMOS_OWNER to the scope recorded for this deployment.
atomos-memory "$HOME/.local/share/atomos/memory" "$ATOMOS_OWNER" verify
atomos --memory "$HOME/.local/share/atomos/memory" "$ATOMOS_OWNER"
```

The deployed store passed integrity verification; the installed CLI returned its stored identity correctly. A separate installed-binary smoke check confirmed that open-ended questions still produce explicitly labeled mock output. No personal facts were seeded, owner vault opened or inference requested.

## Running over SSH

The reference setup starts one local/mock CLI in a detached `atomos` tmux session under the selected owner. Connect with the authorized account/key, then:

```bash
tmux attach -t atomos
```

Detach with Ctrl-b, then d. The session keeps its process across SSH disconnects; it does not survive a reboot or constitute durable conversation recovery. The CLI waits for input without remote inference. A fresh login verified PATH, owner process, session, clean source, healthy services and text-only boot target. Do not create a duplicate active conversation accidentally.

After a reboot or normal CLI exit, start the verified command again with the existing store/scope. Automatic service startup is not configured. Do not overwrite an existing tmux session, identity store or release path. Keep previous releases when updating so the current symlink can be rolled back deliberately; data compatibility must be checked separately.

## Efficiency decision and next work

The SSD baseline at 100K entities shows exact recall p95 0.0793 ms, selective context p95 0.3974 ms and active CLI peak RSS 4,500 KiB versus 3,052 KiB without a store (1,448 KiB incremental). These fit the existing 5 ms exact / 50 ms context / 8 MiB incremental targets. No kernel rewrite, additional database server, large cache, speech stack or local model is justified by these measurements.

Use local deterministic recall, bounded context and checked admission to reduce unnecessary inference work; do not equate context bytes with provider tokens or infer energy savings. Physical power, real model quality, token savings, provider usage and network latency remain unmeasured. More available RAM/CPU is capacity, not an instruction to consume it while idle.

M2.5b runtime is implemented and verified offline on PR #36, merged with successful post-merge CI. Development source is now a detached review-branch checkout, separate from the pinned installed release; the clean-main source statements above describe the historical deployment verification. [Exact development evidence](m2-text-outcomes.md#verified-candidate-evidence). The [specified CLI session integration](cli-session-contract.md) is next; its launch/command surface exists on a review branch but is not installed. Full verification and human review precede any coordinated deployment. Keep the current release and tmux session pinned during development. The deployed CLI does not yet connect extended atomic replay to conversation or a provider. Explicit memory mutations still use the separate administration binary. Keep credential, spending, private-memory disclosure and live-request gates intact; the three historical attempts and 3,072 retained planning units remain consumed.
