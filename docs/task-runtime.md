# Durable synthetic task runtime

The `atomos-tasks` Rust library implements the persistence foundation for [MVP gates A05-A07](mvp-contract.md#acceptance-gates). It is not a service, planner, real executor, external approval system or LAN interface. The installed CLI is unchanged.

## Interfaces and state

`Runtime::initialize` explicitly creates a fresh private store with owner, policy version and a required main-database byte cap. `open` never initializes/replaces missing state. Goals, submission keys, task/action state, bounded failure categories and metadata events are durable. Goals are private application state needed for recovery, not diagnostic logs or an automatic chat archive. Task Debug and event/action-status views omit goal text and keys.

`submit` returns the existing task for an identical key/goal and rejects conflicting reuse, including after restart or terminal completion. Maximum goal size is 8 KiB UTF-8. Token fields are bounded ASCII identifiers. Terminal tasks and idempotency keys are retained in this slice; automatic retention/compaction is not implemented. The store fails closed at its configured capacity instead of dropping acknowledged tasks.

`start` selects the first queued task and permits one running/cancelling task. Up to 16 queued/waiting/interrupted tasks may be newly admitted; recovery can additionally retain the already accepted active task as interrupted. Recovery never discards it to satisfy admission capacity. `wait_for_approval` releases the slot, but is only a synthetic planning state. `requeue` does not grant external permission; there is no external-effect approval/dispatch API.

`prepare` commits immutable synthetic action intent before effects and increments the dispatch count. Only built-in `Add` and `Assert` operations on private fixture counters exist. There are no arbitrary callbacks, subprocesses, commands, credentials or network calls. `apply` commits a fixture effect and its result separately from `verify`; duplicate application fails. `verify` uses that durable fixture result, not a model claim or arbitrary command exit code. `complete` requires verified fixture actions. It does not interpret or independently prove an arbitrary natural-language goal; real goal verification is later work.

`cancel` commits the stop state before acknowledging. Further prepare/apply calls fail. `finish_cancel` closes unresolved intents without executing them; existing effects remain visible through bounded `actions` status. Cancellation is not rollback. This synchronous library has no worker to stop and does not qualify device/load/browser cancellation latency.

## Recovery and concurrency

The main SQLite database uses DELETE journal mode and FULL synchronous transactions. A separate SQLite connection holds an exclusive transaction on the lease file for the runtime lifetime; another runtime fails busy. Process death releases the OS lock without deleting lock files or treating stale markers as permission. Database changes remain separate transactions. Directory entries are synced during initialization.

After every open, mutating calls are blocked until explicit `recover`. Recovery marks active work interrupted, preserves cancellation intent and reconciles no effects automatically. It conservatively counts time since the last running checkpoint, including downtime. An interrupted action with a known fixture effect can be verified without reexecution; an intent with no durable effect proof stays ambiguous and cannot be requeued. It can be cancelled. This is a synthetic guarantee, not exactly-once execution for arbitrary future tools.

Caller-supplied nonnegative timestamps must not precede the last durable accepted timestamp. They make tests deterministic but are not an authenticated clock. Production adapters must supply reviewed time/policy inputs, not browser-provided timestamps. Task execution limit: 32 dispatches and 600,000 ms; ordinary fixture effect deadline: 60,000 ms from intent. Approval waiting is excluded. Task-time, dispatch-limit, action-deadline and fixture-mismatch failures retain their category across restart. Budget exhaustion fails durably before further effect; already committed effects are not refunded.

## Storage and trust

Linux canonical owner-private directory (0700), regular owner-private files (0600), one hard link, no symlink paths/sidecars; application/schema IDs, owner/policy, quick integrity and foreign keys checked on open. Unexpected WAL/SHM files are rejected. Main DB cap is 64 KiB to 256 MiB in 4 KiB pages, fixed at initialization. It does not bound rollback journal bytes, total filesystem use or free-space reserve. Those deployment controls remain required. No store reset, repair, forgetting, migration or automatic deletion interface is added.

Any SQL persistence/commit failure poisons the runtime: it will not report unconfirmed state or perform further actions. Reopening requires file/integrity checks and recovery. Same-UID/root modification, filesystem rollback, power-loss durability, backup deletion and privileged/network confinement are not solved by this library. No private AMG or inference journal is touched.

`events` provides at most 64 metadata transitions after a durable cursor, rejects future/invalid cursors and never resubmits work. `actions` returns at most 32 statuses, including committed-effect presence after cancellation. These are state inspection primitives; browser sessions, streaming protocol, event expiry and action-progress subscriptions remain unimplemented.

## Validation and measurement

Seventeen independent tests cover restart/idempotency, FIFO/exclusivity/capacity, exact fixture effects, false-success prevention, wait time, cancellation before/after effects, time/dispatch limits, rollback clocks, bounded cursors/redaction, invalid/cross-task inputs, private files, missing state and database-full poisoning. One parent actually kills a child at eight boundaries: before intent, after intent, inside an uncommitted effect transaction, after effect, after verification, after completion, and cancellation before/after effect. The ignored child-only helper is not an additional test. Tests use temporary synthetic data only.

The 30-trial cancellation test measures durable library acknowledgement only; A07 worker shutdown under device load remains pending. Kill/recovery fixtures are not power-loss certification. Rust 1.77 is the declared new-library minimum because the existing rusqlite dependency uses C string literals; core Rust 1.75 checks remain separate. Final minimum-version verification is recorded in [current state](../PROJECT_STATE.md).

```bash
cargo test --locked --offline -p atomos-tasks --all-targets
bash scripts/benchmark-tasks.sh
```

The benchmark requires private persistent TMPDIR and reports 200 submissions/intents, 100 effects/verifications, 100 cancellations, one open/recovery sample, p50/p95/p99, process RSS, binary and apparent store bytes. Independent assertions require 100 completed, 100 cancelled, counter 100, 700 transition events and zero real calls. It is an initial measurement with no equivalent prior task engine or optimization claim. [Resource method](resource-budget.md).

Next: add the bounded worker/service loop and a thin authenticated LAN adapter around these APIs. Preserve synthetic testing until a separately reviewed real executor and permission boundary exist. Natural-language intelligence, external approvals, learning/retention and MVP gates remain pending beyond their tested persistence subsets.
