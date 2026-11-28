# CLI session integration contract

Status: proposed next implementation slice, 2026-11-27. This document defines behavior; it adds no commands or runtime implementation. M2.5b is merged at `e3ebf241b64cdf5e0840f944b2c311b6f78a2427`; [post-merge CI 46080072830](https://github.com/idmfrank/atomos/actions/runs/46080072830) passed 193 distinct tests, Rust 1.75 core targets, 18,000 decoder samples and both admission/outcome benchmarks. The merge has exactly the same Git tree as final PR #36 head `2b98805ce335286cffbd802c42b6cbc4c602cdd2`, which passed [full scale/recovery 46079729987](https://github.com/idmfrank/atomos/actions/runs/46079729987). That scale run is branch evidence, not a new post-merge run.

[Roadmap](cli-mvp-roadmap.md) | [Current state](../PROJECT_STATE.md) | [M2.5b](m2-text-outcomes.md) | [Architecture](architecture.md) | [Resources](resource-budget.md) | [Security](threat-model.md)

## Objective and scope

Build one coherent, bounded terminal session over the existing Rust components. One owner, one foreground process and at most one active request. Provide local commands, explicit read-only AMG access and honest outcome/accounting status. Demonstrate M2.5b through finite synthetic replay. Preserve the existing default, single-turn and voice-event modes as the frozen reference.

This slice does not connect a real model. Mock repeats input; replay returns selected synthetic fixtures. Neither is conversation-quality evidence. Keep the existing Linux/Rust/SQLite foundation. No browser, network listener, service, scheduler, worker pool, shell/tools, automatic memory extraction, credential/vault access, new live attempt, private AMG disclosure, transport, billing route or paid fallback. No host configuration or deployed release change is authorized by this contract. All three consumed owner attempts, markers and 3,072 retained planning units remain intact.

Development and offline tests continue directly from this chat. No separate Codex handoff is needed. A reviewed runtime PR and owner merge precede a separately coordinated deployment.

## Proposed launch surface

These commands are **proposed and not runnable until implemented**. Use generic paths/scopes in public examples.

| Invocation | Meaning |
| --- | --- |
| `atomos --session` | New local/mock session; no inference journal or provider |
| `atomos --memory DIR OWNER --session` | Same session with the existing explicit read-only memory adapter |
| `atomos --session --replay NAME --ledger DIR` | Explicit synthetic replay; open an existing private atomic namespace |
| `atomos --session --replay NAME --ledger DIR --init-ledger` | Explicit create-once synthetic namespace in an already private root; any existing namespace is an error |

Replay is Linux/Unix-only and enabled by a new opt-in CLI feature `session-replay` forwarding to `atomos-core/atomic-admission`. A build without it rejects replay flags before opening files; it never silently selects mock. The new local/mock session is available without that feature. Existing `atomos`, `--single-turn`, `--voice-events` and their memory/AEC combinations retain existing behavior and output. Do not change speech harnesses to use the new session protocol.

Validate the entire argument combination before opening a memory store or ledger. Reject duplicate/unknown options, missing arguments, replay without ledger, ledger/init without replay, memory combined with replay, and session combined with single-turn/voice/AEC. No inferred ledger path, auto-initialize-on-open, reset, unlock, repair, deletion or automatic fallback.

Replay NAME is a fixed allowlist: `text`, `auxiliary`, `reasoning`, `refusal`, `failed-known`, `failed-unknown`, `rate-limit`, `malformed`. Use existing synthetic fixtures and bounded synthetic failed-event construction for the last four. No arbitrary fixture path, raw provider dump, URL, user-supplied policy/model/account identifiers or credential input. Exact fixed configuration is versioned in code and tests: reuse the admission benchmark's synthetic account/host/grant 1/2/3, model 7, policy version 1, task 5, fixture time 200, expiry 230, window 100–1000 and grant expiry 900. Reuse its eligibility flags and limits (total 1,000,000; background 100,000; interactive reserve 300,000; per request 1,500); session requests are interactive with output bound 64. These are fixture IDs, time and planning units, never real authorization or usage.

A normal replay open uses the exact same header/configuration. It resumes the durable next request ID; turn ownership is current session state. Fixed fixture time is explicitly synthetic, not wall-clock protection. No rolling budget or window reset is introduced.

## Commands and local memory

Recognize exact ASCII slash commands after surrounding whitespace trim; keep slash command spelling case-sensitive.

| Input | Required local behavior |
| --- | --- |
| `/help` | Explain available commands, selected mode, mock/replay limits, termination and separate memory administration |
| `/status` | Fixed labels and bounded numeric state, never prompt, response, raw provider ID, credential or journal digest |
| `/clear` | Clear volatile working turns and the existing local-memory follow-up subject; leave durable facts, counters, journal, charges and suspension untouched |
| `/quit` | Clean close while idle; do not refund or reset accounting |
| Unknown slash command, including `/cancel` or arguments to these commands | Local usage error; no inference, replay read, memory operation or consumed request ID |

A leading slash is reserved for commands in this slice. No escape-to-shell, slash completion, terminal history file, profile/fixture switching, retry or reset command. Blank lines do nothing. Preserve existing exact `help` and name queries as local aliases, plus explicit recall/context/follow-up grammar from the existing Conversation adapter. Do not treat ambiguous natural language as a control command.

With explicit memory enabled, use the existing reader and disclosure checks. Re-query committed facts every turn so external correction/deletion is immediately visible; never use saved response text to answer a fresh recall. `/clear`, unrelated input and errors invalidate the follow-up subject. Missing/unavailable memory gives a local message, never an inference fallback for a recognized memory query. Mutation-intent messages remain local refusals directing the owner to `atomos-memory`, including when no reader is configured. No conversational remember/correct/forget/export command, new memory schema or automatic commit. Memory administration remains the existing separate executable.

Local results bypass inference admission and remain usable after an in-session replay suspension or storage fault. Local command processing does not consume a synthetic ID or read the replay source. If initial ledger open/init fails, fail startup without entering a mock session; owner may launch local/mock separately.

## Input, working context and output bounds

The new session accepts a single line at a time, at most **1,024 raw UTF-8 payload bytes before trimming**, excluding one terminal LF and optional preceding CR. Bound reads to payload limit plus terminator/overflow detection; never allocate an unbounded line. Reject invalid UTF-8 and control characters (including embedded CR, tab, ESC and C1 controls) after stripping the terminal line ending. An oversized/invalid line ends the session as input error so its tail cannot be interpreted as another command. A final nonempty EOF-terminated line is processed once; whitespace-only final input does nothing. Legacy modes retain their existing 8,192-byte transcript and whitespace rules.

Retain only successfully delivered mock/synthetic completed question-answer pairs: **at most four whole pairs and 16 KiB of UTF-8 text payload in total**, counting both roles. Evict oldest whole pairs until both limits hold. Reject an individually unrepresentable pair rather than slice UTF-8 or fabricate text. Input is at most 1,024 bytes, ordinary replay output at most 4,096, so one pair fits. Count actual bytes, not allocated capacity or claimed tokens; container/metadata overhead is measured separately.

History is volatile session state, not AMG or a transcript file. Commands, local identity/recall/context previews, raw reasoning/refusal, failed/cancelled/invalid outcomes and undelivered answers do not enter history. Clear on `/clear`, normal exit and process restart. Explicitly state that clearing RAM is not cryptographic erasure and cannot clear the user's terminal scrollback.

Retained turns are **local-only in this slice**. They do not change the fixture response and are not inserted into `CompiledRequest.text`, its digest or token estimate. Admission binds the current bounded question exactly as M2.5b does; do not claim the existing digest attests retained history or AMG context. No automatic use of prior local memory results as synthetic input. Status/help must say that working turns are retained locally and no model context is being sent. This avoids pretending that storing history makes the mock intelligent.

A later provider slice must define a canonical complete request including every history/context byte, bind that owned request to admission, update bounds/estimation honestly and recheck disclosure before sending. It may not send this history through the current-question digest or silently concatenate it past the existing 1,024-byte cap.

Buffer synthetic text until finite stream validation and required durable settlement. Print fixed mode/outcome labels, then validated ordinary text; no raw refusal, reasoning, failed provider message or decoder dump. Ordinary inference output stays within the existing 4,096-byte decoder limit. Reuse the local context-preview budget; cap the final local response at 8,192 bytes including its preview heading, without increasing the 4,096-byte context packet budget. Invalid/unsafe local text yields a fixed local error, never raw terminal controls or a model fallback. Allow newline/tab only in validated response bodies; reject all other control characters. Fixed labels/status do not include user strings.

No prompts, banners, mock labels or accounting changes are added to legacy single-turn/voice protocols. New mode is line-oriented and requires no ANSI sequences, cursor manipulation, terminal size, full-screen UI or shell startup modification.

## Shared session and accounting interface

Keep state, bounds and typed final events in shared Rust code; terminal parsing/rendering and the existing AMG adapter stay thin. Reuse TurnManager/epoch ownership, local resolution and AtomicReplay. A worker pool, new policy engine, duplicate journal, public dispatch permit or second decoder is unnecessary. Equivalent internal names are acceptable; test semantics rather than mirror implementation.

Use a fresh immutable current request and Context for every admitted replay. A fixed selected replay fixture is decoded once only after synced A; the decoder may perform multiple bounded Read calls. Request IDs come from the opened journal; turn ownership is checked before output. A new line after a successful response is an explicit new turn, not an automatic retry of an old one. Never retry a rejected, failed, cancelled or unknown turn under a new ID automatically.

Add only the read-only atomic status view needed to render authoritative distinctions. Suggested snapshot fields: health, permanent halt, optional outstanding reservation units, confirmed total/background charge and next request ID. No mutator, live permission/permit, prompt, digest or response content. On a poisoned instance, mark accounting unavailable/last-confirmed; do not present last confirmed totals as the final durable result of an ambiguous write/sync. Preserve strict replay behavior, journal format, caps and namespace.

`/status` reports mode (local/mock or synthetic replay), provider disabled, memory enabled/disabled, volatile pair/byte counts and local-only context. Local/mock has no token ledger: accounting is not applicable, never a fictitious real quota. Replay additionally reports fixture-unit charge, pending amount, halt/health, next ID and whether further replay is blocked. It distinguishes local resolution, mock calls and synthetic dispatch counters; they are not actual model calls or provider tokens. Counters are bounded checked integers; exhaustion fails closed. Clearing history does not reset counters.

| Result | Rendering and session effect |
| --- | --- |
| Local | Local label and safe text; zero replay dispatch/charge |
| Mock completed | Explicit mock label; existing echo behavior, no ledger; retain pair only after successful output |
| Replay completed | Synthetic completed label and ordinary text after S sync; retain delivered pair |
| Replay refused | Fixed refusal notice, known fixture usage/accounting; no raw refusal, answer, retry or history pair |
| Replay failed, known usage | Fixed typed failure label and known charge; next replay allowed only if atomic state permits |
| Replay failed, unknown usage | Fixed failure/usage-unknown notice; outstanding A remains charged; next replay blocked |
| Rate/auth or overrun | Fixed failure/overrun notice and durable suspension; local commands still available |
| Cancellation, malformed, unsupported, I/O, invalid usage | Fixed category; no partial answer/history/refund; inspect authoritative pending/health state |
| Storage failure | No successful outcome/text; mark accounting unavailable and block replay in that instance; local help/status/clear/quit still available |
| Budget/identity/expiry denial | Local category, no source read or consumed ID; no fallback; local commands still available |

The v1 journal is accounting, not conversation history or a delivery receipt. A complete surviving S may have committed despite a returned sync error. A later output failure may happen after successful settlement. Neither case authorizes refund, repair, a retry or claims that text was successfully displayed.

## Cancellation, EOF, disconnect and exit

Use a single synchronous foreground request and finite bounded in-memory fixture readers. There is no in-flight command channel or background prompt queue managed by AtomOS. Input already buffered by a terminal/pipe is read only after the active call finishes. `/quit` or EOF cannot preempt that call; do not advertise `/cancel`.

Keep cooperative cancellation in the shared replay driver for deterministic tests, forwarding one cancellation source into the existing decoder and invalidating stale delivery ownership. Check ownership before publishing or retaining an outcome. If cancellation arrives after S has committed but before display, suppress stale output/history while keeping the settled charge. No new decoder cancellation point splits an existing synchronous settlement. Do not promise that a cooperative check interrupts a blocking Read, fsync or stdout write.

In this slice **Ctrl-C keeps the operating system's default process-termination behavior**, not graceful request cancellation. No new signal package or unsafe handler is required. On an admitted replay, abrupt termination may leave a charged A and stale writer lock; reopen fails closed. Never automatically remove the lock, truncate/repair the journal or assume a refund. Document manual investigation without supplying a destructive reset shortcut. Ctrl-C at idle terminates rather than keeping the session alive. A future live transport must separately implement and test bounded deadlines, signals and cancellation/reconciliation before activation.

EOF while idle closes cleanly; EOF following an admitted finite response is handled after that response. Direct SSH loss may terminate the process; existing tmux may keep it alive. Neither transport/session state proves inference cancellation. No daemon restart, automatic request resumption, autosave or conversation resurrection. Persistent facts/ledger stay intact; working turns disappear.

Use exit 0 for normal EOF/`/quit` with healthy unblocked accounting, exit 1 for fatal input/I/O/storage or blocked/unavailable replay state at close, and exit 2 for invalid launch options. A handled per-turn failure with known usage that leaves replay healthy may still end normally; its outcome remains visible. Signal termination retains normal platform status; do not claim a portable handled 130 exit. Stop on stdout failure without retrying output or dispatch. Buffered full-answer publication can still partially write to the terminal before an I/O error; no all-or-nothing display guarantee. If S already committed, retain that settlement. Only a fully successful write/flush permits adding the pair to volatile history. If even a fixed error cannot be written, exit without leaking raw data through diagnostics.

## Independent tests and resource evidence

Freeze current main `e3ebf24` for legacy behavior. Preserve all 193 distinct baseline tests, 18,000 decoder samples, strict/extended atomic methods and original benchmark assertions. Add independently expected session unit/integration tests and real-process terminal/pipe tests; do not derive expected output from the candidate renderer.

Required coverage:

- Legacy default, single-turn and voice/AEC/memory behavior unchanged on equivalent inputs; compare exact observable output, exit and store state, with historical reference runs where available.
- Launch grammar, mode/feature conflicts, complete validation before file access, explicit create-once/open-only, private permissions, occupied namespace, wrong header and locked/corrupt ledger. No implicit fallback or repair.
- Input lengths 0/1/1,024/1,025, UTF-8 boundaries, CRLF, EOF-final line, embedded controls, oversized tails and piped command injection; invalid input never dispatches.
- Exact local commands/unknown slash commands, memory-disabled and unavailable cases, mutation-intent denial, local recall/context and correction/deletion visibility with synthetic facts; no local result enters inference history/input.
- Whole-pair eviction by count and bytes, clear/restart semantics, failed/refused/undelivered pair exclusion and stale output suppression; independent text and counter assertions.
- Every outcome row, known/unknown failed usage, pending/halted/unhealthy distinctions, continued local commands during suspension, no retry/refund, fresh ID/turn checks and one decode invocation per dispatch, with zero source reads before admission.
- Real subprocess EOF/quit/invalid input/exit behavior; bounded forced termination after A leaves stale lock. Separately test cooperative cancellation before read, between reads, after terminal before EOF and post-settlement/pre-display cancellation, plus broken stdout after settlement.
- No raw hidden fixture values, unsafe controls, prompts or credentials in status/errors/debug/journal; fixtures/ledger and memory store remain isolated. No network, owner vault, private facts or live calls.

Use bounded private synthetic temp state and externally bounded child tests. Fault injection stays test-only. No added production fault knobs or argument accepting arbitrary synthetic policies. Do not count repeated features/compiler runs or child fixtures as extra distinct tests.

Benchmark the same preloaded synthetic streams, policy, durability and fixture time through direct M2.5b and the new session driver, excluding terminal I/O for shared-driver overhead and measuring terminal I/O separately. Use 300 mixed-priority requests for the direct/session-driver comparison (the benchmark may inject background priority; interactive CLI stays foreground only), plus refusal and failure/cancellation cases reported separately. Assert exact outcomes, charges, turn/ID and no stale/history contamination, not merely counts.

Report p50/p95/p99, sample/error/outcome counts, bounded-history operations, time before first read, full decode/settlement, output delivery separately, RSS, binary sizes with/without session-replay, retained payload and journal bytes. Measure journal admission/settlement syscall order separately; keep one A sync and required S/H sync. Identify backing filesystem and use private persistent TMPDIR for durability/AMG runs. Preserve the slower previous SSD observation without inferring improvement from new runners. Default CLI size may change when local session code links; measure it rather than promise no overhead.

Before runtime review: full locked offline regression, all feature/session/Node tests with decoder prerequisite, Rust 1.75 core all-targets, existing decoder/admission/outcome benchmarks and full AMG 1K/10K/100K recovery. New CLI code runs on the normal supported workspace toolchain; whole-workspace Rust 1.75 compatibility remains unverified because of the pre-existing unrelated dependency. No dependency upgrade, install or host change is part of this specification. Proposed commands/tests/benchmarks are pending until implemented.

## Delivery and next gate

Implement this contract in a reviewed runtime branch; keep the current deployment pinned during development. Reconcile README, PROJECT_STATE and every affected architecture/security/protocol/operation/resource document with exact tested commits and evidence in each authored change. This specification's CI is pending at authoring; passing it validates existing code, not the future session implementation.

Done means the new opt-in terminal journey is coherent, all bounds and outcome/termination semantics pass independent tests, legacy modes remain preserved, exact resource tradeoffs are documented and human review/merge is complete. It remains an offline/local/mock capability.

After this slice, review the complete provider boundary: owned context/AMG disclosure, credentials, eligibility/spending/output exposure, trusted time, deadlines/cancellation and authoritative usage. That gate precedes a transport implementation and any separately authorized new live procedure. Do not reopen the consumed qualification harness.
