# Bounded synthetic worker and service

This completes the source/test portion of MVP Step 2. It does not add a LAN listener, model, real tool executor, approval grant, learning pipeline or deployment. [Contract](mvp-contract.md) | [Task store](task-runtime.md) | [Current state](../PROJECT_STATE.md).

## Actor and fixed policy

One Rust actor owns Runtime and its exclusive store lease. The bounded control channel holds at most 16 commands; oversized submissions are rejected before enqueue. Full rejects before acceptance. Replies have a five-second wait: Unconfirmed means the operation may have committed, so retry/query the SAME submission key rather than creating another task. No implicit retries.

Worker::start requires an existing owner-bound store with the fixed synthetic-worker-v1 policy, performs recovery before readiness and processes queued work in FIFO order. Each task runs one Add(worker-fixture, 1) action, verifies it and completes. Goal text is a private label, never interpreted as a plan. Existing actions must match this recipe before continuation; another recipe under the same policy is rejected. Default phases advance without artificial sleeps. A bounded optional fixture delay exercises cooperative cancellation.

Status/events do not resubmit work. Cancel commits stop state, closes unresolved fixture intent and retains any already committed effect before replying. No later effect for that task is dispatched after acknowledgement. The worker stops on persistence or clock failures without claiming success. There are no arbitrary command callbacks or network operations.

## Recovery, clock and shutdown

Queued accepted tasks run after startup. Active work becomes interrupted; cancellations are reconciled without effects. Resume is explicit: a known committed fixture effect is verified without repeating it, a verified action proceeds to completion, and intent with no proof remains ambiguous and cannot be retried. Cancel can close that interruption. The store is never reset or repaired automatically.

The actor supplies timestamps from a startup wall-clock anchor plus monotonic elapsed time; callers of its command API cannot choose them. Persisted clock rollback across restart fails closed. System time and filesystem rollback are not authenticated or solved by this design.

Worker::shutdown requests stop and joins after durable quiescence and lease release. Setting a stop flag alone is not acknowledgement. Active work is preserved as interrupted, queued tasks remain queued and no new effects execute during quiescence. Drop also requests stop and joins. Filesystem stalls can delay I/O; this is not a hard real-time guarantee.

The loop sleeps on a bounded channel/deadline. Idle stop polling is at most 100 ms and does not continuously query SQLite or call inference. The binary uses pinned signal-hook flag registration for SIGTERM/SIGINT; handlers only set the atomic stop flag, with store work in the actor.

## Local control process

atomos-taskd is an optional fixture supervisor/maintenance process. A future Rust web service can own Worker directly; an additional resident process or cross-language bridge is not required.

Arguments, validated before file access:

```text
atomos-taskd --store <PRIVATE_STORE> --owner <OWNER_SCOPE>
atomos-taskd --store <PRIVATE_STORE> --owner <OWNER_SCOPE> --init-bytes <BYTE_CAP>
atomos-taskd --store <PRIVATE_STORE> --owner <OWNER_SCOPE> --daemon
```

Initialization is explicit and only for a fresh empty private directory. Normal startup never initializes missing state. The daemon ignores stdin EOF and waits for a stop signal. It has no control socket or listener. The default stdio mode accepts bounded lines, one serialized request per line:

| Request | Response |
| --- | --- |
| SUBMIT key hex_utf8_goal | TASK id state plus dispatch count/failure category |
| STATUS id | Bounded task metadata, no goal/key text |
| CANCEL id | Durable current task metadata |
| RESUME id | Current metadata or a typed ambiguity/state error |
| EVENTS cursor limit | Metadata lines then END count; at most 64 events |
| STOP or stdin EOF | Quiescence followed by process exit; exit code zero is the acknowledgement |

Readiness is READY synthetic=1 after store recovery. Input is capped at 16,640 bytes, goal at 8 KiB decoded UTF-8, key at 128 bytes. Malformed/unknown requests return sanitized errors. An oversized line ends the control session rather than interpreting its tail. No raw goals, keys, private paths or OS errors are echoed. Output uses a duplicate FD sharing the original file offset, without holding the global stdout lock while reading or writing. After readiness the main thread emits no final output: unread stdout cannot prevent signal-driven quiescence or process exit.

## Supervision template and deployment boundary

deploy/atomos-taskd.service.in is a user-service template with placeholders only. It uses a private existing store, bounded on-failure restart, SIGTERM, a five-second supervisor stop timeout and restrictive fixture sandbox settings. If forced kill is required, the next start still reconciles durable state instead of deleting locks or retrying ambiguity.

Render/escape the binary, store and owner values privately; never commit rendered inventory. Validate the actual service identity, filesystem permissions and sandbox/user-namespace support before installing. Do not silently remove isolation settings to obtain a start. Enabling a user service at boot needs a separately reviewed session/linger or service-account policy. No unit is installed/enabled by this slice.

Tests parse a rendered template with a dummy executable, not a running service. Process tests exercise the real taskd binary but do not qualify supervisor boot behavior, privilege confinement, LAN authentication or a deployment. Those remain later acceptance/deployment work.

## Validation

New Rust coverage tests bounded channel admission, exact fixture execution/idempotency, cancellation, clean/drop shutdown, queue recovery, known-effect resume, wrong-recipe rejection, competing instances, invalid setup and full-store fail-closed preservation. Process coverage tests completion, 30 cancellation trials, TERM/INT while stdin waits, SIGKILL/restart, malformed/capped input, competing/missing stores, EOF versus daemon mode, supervisor syntax, unread-output shutdown and regular-file output order. Timeouts are not relaxed to accommodate failure.

```bash
bash scripts/test-task-service.sh
```

The existing task/store and other regressions remain required. Performance claims may use explicitly identified hosted-CI fixture results only; no target inventory or private-environment measurements are published. [Resource method](resource-budget.md).
