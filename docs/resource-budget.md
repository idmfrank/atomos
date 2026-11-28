# Resource requirements and validation

Use the target Linux environment. Determine filesystem, usable/free space and operational headroom privately before quotas/model selection; public docs must not identify deployment hardware or capacity. [State](../PROJECT_STATE.md) | [Roadmap](agent-roadmap.md).

## MVP defaults and gates

[The MVP contract](mvp-contract.md#initial-bounds-and-service-behavior) defines initial task/action/input/output/queue bounds, mandatory storage caps and resource calibration before activation. These are proposed defaults, not measurements or deployed limits. Test task quality with 20 frozen ordinary-language trials (at least 18 correct, zero unsafe/false completion), cancellation over 30 load trials, and a 24-hour device soak. New checks A01-A13 remain unimplemented; do not count them as existing tests.

## Measure

Idle/active service RSS, startup, CPU/wakeups, web/reconnect/cancel latency, executor/isolation overhead, durable tasks and recovery. Report model task success/errors, first/completion latency, peak RAM and assets separately from commands/retrieval. Reserve SSH/recovery resources.

Learning: throughput/drop counts, queue bounds, consolidation CPU/I/O, evidence rotation, graph/index growth and derived deletion under disk pressure. Set retention from actual growth and reserve; no indefinite packet/transcript archive.

Compare equivalent inputs/state/policy/durability with independent expected facts, revisions, ranking/context bytes, output and charge totals. Record commit, host/compiler/filesystem, counts/errors and p50/p95/p99. Tmpfs is not device durability; preserve fsync and report incomplete samples. Bytes are not tokens, retrieval is not general intelligence, RSS is not whole-device memory/power.

## Offline checks

```bash
bash scripts/test.sh
bash scripts/test-responses.sh
node --test scripts/auth/protocol.test.mjs
bash scripts/test-atomic-admission.sh
bash scripts/benchmark-outcomes.sh
bash scripts/test-session.sh
bash scripts/benchmark-session.sh
cargo +1.75.0 test --locked --offline -p atomos-core --features atomic-admission --all-targets
AMG_SAMPLES=100 AMG_RECOVERY=1 bash scripts/benchmark-amg.sh 1000 10000 100000
```

Provision locked dependencies explicitly. Durability benchmarks need private TMPDIR on the intended persistent filesystem and strace for syscalls. Tests may obtain frozen reference source temporarily from Git history; it is not archived in the branch. Core Rust 1.75 checks do not establish whole-workspace compiler compatibility.

CI runs regressions/decoder/auth/admission/session/1K checks; the manual workflow runs full 1K/10K/100K graph scale/recovery. Record current candidate results in PROJECT_STATE with exact source/run links. New service/model/executor/learning checks await implementation. No power, token-saving or local-model quality claim is established.
