# AtomOS current engineering state

Updated 2026-11-28. Active work: finish MVP Step 2 with a bounded synthetic worker and supervised local process. Main base 135ad65256def353ff106e04d57a172ef29f2f5f (PR #44) passed [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/46105407113). This slice changes source/tests/docs only; no PR merge, deployment, live inference, credential access or real command execution.

## Progress

| Step | Scope | Current status |
| --- | --- | --- |
| 1 | MVP contract | Complete and merged |
| 2 | Persistent task runtime and worker/service loop | Implementation and remote offline tests complete; review/CI pending |
| 3 | Authenticated LAN web interface | Next; not started |
| 4 | Local tools and administrative authority | Not started |
| 5 | Local reasoning/model integration | Not started; private calibration required |
| 6 | Continuous learning/graph consolidation | Not started; AMG foundations available |
| 7 | Scoped system/network observation | Not started; payload capture remains later |
| 8 | Optional external inference | Not integrated; auth/accounting foundations available |
| 9 | Deployment and MVP qualification | Not started |

Publication privacy rules/checks are implemented. Current-text corrections do not erase earlier public revisions or copies.

## Implemented scope

Rust task persistence now has a single owning actor, a bounded 16-command control queue, fixed-policy synthetic execution, FIFO work, idempotent submissions, cancellation, explicit recovery/resume and clean/drop shutdown. The new atomos-taskd process supports bounded private stdio control or daemon mode, TERM/INT quiescence, EOF semantics, redacted output and competing-instance rejection. It processes one fixed counter recipe; goals remain private labels, not plans. [Worker/service](docs/worker-service.md), [task storage](docs/task-runtime.md).

Known effects resume without reexecution; unknown intent remains ambiguous until cancelled. Queued work survives restart. Persistence failures stop the actor without resetting state or losing acknowledged submissions. Supervisor template syntax is tested with a dummy executable; actual boot/session/linger policy, sandbox qualification and installation remain Step 9 work. No network listener, model, real tools, external approval grant or automatic learning is added. The text CLI is unchanged.

## Validation

The approved remote development checkout passed workspace/CLI/memory recovery, decoder, session/text-reference, worker/process, auth, task measurement and publication checks. No private environment inventory or numeric performance data is published. Source-level minimum compiler gates remain in hosted CI: tasks Rust 1.77 and core Rust 1.75. Candidate hosted CI and full graph scale/recovery are pending at authoring.

New coverage: one bounded-channel unit test, eight Rust worker integration tests, and ten process tests. Total expected distinct suite is 210: 163 Rust, 27 Python, 20 Node. Repeated feature/compiler runs, test samples and child-only fixtures are not additional tests. Thirty process cancellation trials and real TERM/INT/SIGKILL cases retain their deadlines. Signal tests caught a stdout-lock shutdown defect; the corrected duplicate-FD transport, unread-output test and regular-file output-order test pass. Existing task/store tests remain required.

The signal-handling dependency is pinned and flag-only; its callbacks do no storage work. The lock adds signal-hook/registry and supporting errno/platform packages without upgrading existing package versions. No unsafe code is introduced in project source. API publication will use the connected repository account if remote push authentication is unavailable, without copying credentials.

## Operations and publication

Development source and installed releases remain separate. Builds/tests and Git preparation now run in the approved remote checkout, not the desktop/WSL control client. Private connection details, manifests, resource budgets and raw logs stay outside GitHub. Hosted runner fixture evidence is distinct from deployment qualification. No unit is installed/enabled by this change.

Review/update covers README, roadmap, architecture, task/worker contract, security, resources, operations and engineering instructions. AMG, inference and owner-auth APIs are unchanged. Publication and link/anchor/fence/whitespace checks passed; remote service output confirms ten process tests and eight worker integration tests. Existing registry versions/checksums are unchanged. Hosted minimum-compiler and full graph checks remain pending at authoring. Next is Step 3: a thin authenticated LAN adapter owning the worker directly, without requiring another resident bridge.
