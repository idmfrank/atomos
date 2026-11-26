# AtomOS

**An OpenAI-native, always-on operating environment for personal AI agents, designed for natural voice conversation, zero incremental AI spending and efficient use of compute, power and tokens.**

AtomOS is an experimental project to build a persistent, conversational AI agent that can operate continuously on inexpensive, low-power hardware. The agent owns its identity, memory, permissions and scheduled activities independently of the language model it uses. Reasoning is delegated to OpenAI only when necessary, with the goal of operating within an owner's existing ChatGPT Plus subscription without additional AI spending. The runtime remains useful offline and retains a narrow inference interface for optional self-hosted models.

**Status:** M1.1–M1.3 merged and automated validation passed. M1.4a (AtomOS Memory Graph durable foundation) in development; bootstrap CI passed, locked verification pending, resource validation pending. Physical speech/model tests, always-on wake word, persistent identity, live OpenAI inference and bootable OS remain unvalidated or unimplemented.

## Current implementation

| Slice | Implemented (automated tests passed) |
| --- | --- |
| M1.1 | Dependency-free Rust core/CLI, local/mock inference, turn epochs, bounded playback queue, fixed-budget persistent reservation ledger. [PR #2](https://github.com/idmfrank/atomos/pull/2) |
| M1.2 | Single-turn ALSA/Whisper.cpp/Piper smoke harness, transcript bounds, instance-scoped reservations, stub audio tests. [PR #3](https://github.com/idmfrank/atomos/pull/3) |
| M1.3 | Persistent Rust voice coordinator, streaming PCM protocol, RMS endpointing, follow-up window, AEC-gated cancellation, optional warm Sherpa-ONNX/Kokoro adapter, IPC startup/deadline fixes. [PR #4](https://github.com/idmfrank/atomos/pull/4) |
| M1.4a | Initial SQLite-backed typed memory, explicit owner CLI and regression tests on development branch; bootstrap CI passed, locked verification pending. See [AMG foundation](docs/m1-amg-foundation.md). |

Verified post-merge baseline: [83dbf8c](https://github.com/idmfrank/atomos/commit/83dbf8c39e7ed7fae4bcb3230eb4022452540cb5), [Actions run 45752758740](https://github.com/idmfrank/atomos/actions/runs/45752758740). 29 Rust and 5 Python integration tests, CLI and stub audio smoke checks passed; zero remote inference.

| Text-only release benchmark | M1.1 | M1.2 | M1.3 |
| --- | ---: | ---: | ---: |
| Executable bytes | 296,824 | 302,072 | 314,224 |
| Peak process RSS (KiB) | 2,068 | 2,020 | 2,136 |
| Rust tests | 15 | 18 | 29 |
| Python integration tests | 0 | 0 | 5 |

1,000 simulated local text interactions; excludes OS, real speech models, Python adapter and active memory DB/ledger. RSS differences may reflect measurement variability. No acoustic latency, power or total system RAM claim. M1.1–M1.3 Rust baseline has zero external crate dependencies; M1.4a adds system-linked SQLite through rusqlite. Optional hardware tools/models not bundled.

### Development quick start

Linux, Rust/Cargo (declared minimum 1.75, CI previously verified on 1.100), Bash, Python 3, GNU coreutils, pkg-config and SQLite development library with FTS5. M1.4a requires explicitly provisioned, locked Cargo dependencies. Rust 1.75 compatibility of the new transitive dependency set has not been verified; CI uses the installed newer toolchain:

```bash
cargo fetch --locked # explicit provisioning step
bash scripts/test.sh
cargo run --locked --offline -p atomos-cli --bin atomos
bash scripts/benchmark.sh
```

Try "what is your name?" (local), "explain entropy" (mock), and "/quit". See [local voice smoke test](docs/local-voice-spike.md) and [persistent conversation](docs/persistent-conversation.md) for optional hardware adapters. Passing stub tests do not validate physical audio. Executable changes require passing CI before merge.

## Design goals

- Make natural, responsive, interruptible voice conversation the primary interface; retain text for accessibility and debugging. No screen or desktop is required.
- In order of priority: conversational quality, zero incremental AI spending, persistent identity and autonomy, token efficiency, power efficiency, then hardware cost. Security and privacy are non-negotiable constraints.
- Run continuously on efficient x86-64 hardware initially; support ARM, RISC-V and microcontroller-class satellites later.
- Minimize idle power, RAM, storage, CPU wakeups and network traffic without compromising conversational quality.
- Preserve identity, conversation context, commitments and preferences across reboots and model changes.
- Make OpenAI the first-class remote inference provider, using official ChatGPT plan sharing where eligible and authorized. Keep agent state and local operation independent of the provider.
- Treat inference as a finite system resource: resolve routine events locally, compile minimal context and enforce token budgets before every remote request.
- Target zero incremental AI spending; never purchase credits or silently switch to a separately billed API.
- Keep routine scheduling, simple commands and local memory available without internet access.
- Enforce least privilege for tools, data and device access. Keep the owner in control of the agent and its data.
- Prefer simple, auditable components and reproducible, hardware-specific builds.

## Non-goals for the first release

- Training or hosting a general-purpose LLM on a microcontroller.
- Replacing Linux with a new kernel before the agent runtime is validated.
- Supporting arbitrary desktop applications, POSIX compatibility on microcontrollers or a graphical desktop.
- Unrestricted autonomous actions, continuous audio recording or mandatory remote storage.
- Unlimited remote inference, unofficial ChatGPT endpoints, browser automation for inference, or circumventing subscription limits.

## Architecture

```mermaid
flowchart TD
    U[Human: voice or text] <--> P[Streaming ASR, turn detection, TTS and barge-in]
    E[Timers, sensors and authorized events] --> R
    P <--> R[Persistent agent runtime]
    R <--> M[AMG: identity, graph, journal, FTS]
    R <--> S[Event scheduler and task journal]
    R --> A[Capability and approval policy]
    A --> T[Authorized tools and devices]
    R <--> D[Local intent resolver and approved workflows]
    R --> X[Context compiler]
    M --> X
    X --> G[Token Governor]
    G --> B[Inference broker]
    B <--> C[OpenAI Responses API via authorized ChatGPT plan]
    B <--> H[Optional self-hosted inference]
    R --> O[OS services: network, storage, power]
    O --> HW[x86-64 voice host; optional ARM or MCU satellites]
```

The diagram represents the target architecture, not all implemented services. The language model is a replaceable computational resource. The persistent agent runtime is responsible for state, context selection, task execution and authorization. Model output is treated as untrusted input, never as direct authority to execute a tool.

### Runtime lifecycle

| State | Activity |
| --- | --- |
| Resting | Wait for a wake word, timer, message or sensor event. |
| Perceiving | Capture speech or text and classify the event. |
| Responding | Retrieve relevant context and perform local or delegated conversation. |
| Reasoning | Invoke an appropriate model only when necessary. |
| Consolidating | Checkpoint state, summarize selected interactions and maintain memory within quotas. |

Use an event-driven architecture with bounded queues, explicit timeouts, retry backoff and no mandatory continuous inference loop. Keep task checkpoints durable, and make retried operations idempotent wherever possible.

## OpenAI-native inference and token economics

### Subscription-only operating policy

Use the official [Sign in with ChatGPT developer documentation](https://developers.openai.com/siwc/token-sharing-open-source) for eligible locally hosted integrations. The first integration must verify AtomOS eligibility, supported capabilities and the user's consent before enabling live inference. A private development repository does not itself establish eligibility.

- Use the documented OAuth registration and authorization flow, PKCE, validated identity and granted `chatgpt.tokens.use.direct` permission. Persist the issued client registration and a stable, opaque host identifier; protect and revoke credentials according to the provider's guidance.
- Discover available models through supported endpoints. Use authorized streaming Responses API requests with `store: false` and `stream: true` as required; keep durable conversation state locally. Do not assume this setting guarantees zero provider retention.
- No API-key billing fallback, credit purchases, unofficial endpoints or automatic plan upgrades. The owner must disable additional credit use for the app in ChatGPT's controls; confirm this prerequisite and fail closed if subscription-only spending cannot be established. An internal token cap alone cannot enforce the provider's billing rules.
- Treat provider usage limits and eligibility as authoritative. On a subscription usage limit, pause remote inference and direct the owner to ChatGPT usage settings. Do not infer a reset time, rotate accounts or retry around the restriction. Use bounded backoff only for transient failures.
- Plus allowances are shared, variable and not a guaranteed monthly raw-token quota. Account-level usage outside AtomOS may reduce available capacity. The agent does not inherit access to existing ChatGPT conversations or memories.

Reference: [OpenAI recovery guidance](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery) and [ChatGPT plan sharing controls](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites).

### Token Governor

The Token Governor is a mandatory runtime service, not a model prompt. All remote inference, including tool follow-ups, retries, memory consolidation and background work, must pass through a single admission interface.

1. Attempt safe deterministic intent resolution, structured memory lookup and authorized workflow execution locally.
2. Assign a priority and the lowest sufficient model/reasoning effort without an extra model call.
3. Compile only relevant instructions, recent context, selected memories and necessary tool schemas. Preserve authorization and safety-critical context; expand context when accuracy requires it.
4. Estimate and reserve input and maximum output usage before dispatch. Enforce rolling and monthly local quotas, concurrency limits and per-task call/deadline limits.
5. Stream the response, reconcile reported usage with the reservation and persist a durable ledger. Count reasoning tokens as part of output where reported, not twice. Track cached tokens separately; do not assume subscription discounts.
6. Preserve interactive capacity by deferring low-priority work. Stop remote calls when budgets, provider limits or authorization prohibit them. Continue local operations and explain degraded capabilities.

Use compact typed memory, local indexing, bounded context windows, stable prompt prefixes where beneficial, and validated reusable workflows. No mandatory remote embeddings, vector database, local LLM or idle inference. Never truncate essential context merely to meet a token target; request permission for a larger task or defer it. Cache only appropriate responses with scope, freshness and invalidation controls.

| Intelligence level | Execution |
| --- | --- |
| L0 — Reflex | Local timers, sensors, simple commands and facts; zero remote tokens. |
| L1 — Procedure | Local, previously authorized workflows; zero remote tokens. |
| L2 — Conversation | Efficient available OpenAI model, minimal sufficient context and reasoning. |
| L3 — Analysis | Increased context or reasoning within the task budget. |
| L4 — Deep reasoning | Explicitly prioritized complex tasks, bounded by available allowance and policy. |

### Initial inference budgets

| Metric | Engineering target / policy |
| --- | --- |
| Incremental AI spending | $0 above existing subscription; hard policy subject to verified provider controls. |
| Idle remote inference | 0 calls and tokens. |
| Routine events handled locally | At least 75% on a defined representative workload; unrestricted conversation measured separately. |
| Routine remote request | At most 1,500 raw input + output tokens, including reasoning. |
| AtomOS monthly raw-token target | 750,000, configurable downward as needed. |
| Default input / output budget | 1,024 / 256 tokens; output includes reasoning where applicable. |
| Background allocation | At most 10% of local monthly budget. |
| Interactive reserve | 30% of local monthly budget protected from noninteractive work. |
| Mandatory remote embeddings / local LLM | None. |

These are internal design targets, not ChatGPT Plus entitlements. For example, 100 events/day with 75% resolved locally and 900 total tokens per remote call yields 675,000 raw tokens over 30 days, before any additional calls. Measure actual usage and quality with a representative workload including tool loops and memory maintenance. Do not claim a month fits Plus until tested against real account limits.

### Voice pipeline

Implemented: 20 ms/16 kHz mono PCM, explicit activation, 60 ms onset, up to 100 ms pre-roll, 300 ms quiet endpoint and eight-second follow-up. Default half-duplex; barge-in only when AEC explicitly verified. Optional Sherpa-ONNX/Kokoro warm development adapter (Python); whole-utterance synthesis, not incremental. Whisper.cpp/Piper remains the batch baseline. Wake word, neural VAD, actual acoustic AEC and physical latency/quality validation remain outstanding. See [M1.3](docs/persistent-conversation.md). The following describes the target pipeline.

Keep wake-word detection and voice activity detection local. Use a bounded volatile pre-roll buffer, stream ASR during active speech, commit text only at a valid turn boundary, route through local intent resolution and the Token Governor, and synthesize streamed text in speakable chunks. Continue listening during playback with acoustic echo cancellation and cancel playback promptly on barge-in. Do not persist audio by default. No continuous cloud audio stream. Do not assume standalone OpenAI audio or Realtime endpoints are covered by plan sharing; disable them under subscription-only policy unless eligibility and zero incremental cost are verified. Voice quality and latency must be measured from the first prototype, including in-room acoustics and interruptions. See [voice architecture](docs/architecture.md) and [resource budget](docs/resource-budget.md).

## AtomOS Memory Graph (AMG) and identity

**Decision:** temporal property graph with provenance, backed provisionally by SQLite on Linux. Logical model independent of physical storage through typed Rust interfaces. Indexed relationships, FTS5 and durable event journal; no separate database daemon, mandatory vector database or remote embeddings. Compare equivalent embedded graph and key-value alternatives before permanent selection. See [AMG architecture](docs/amg-architecture.md).

| Memory class | Purpose |
| --- | --- |
| Identity | Stable agent ID, owner configuration, personality and instructions. |
| Working | Current conversation and active task context. |
| Episodic | Selected interactions, events and their provenance. |
| Semantic | User-approved facts, relationships and preferences. |
| Procedural | Tool descriptions, approved procedures and workflows. |
| Prospective | Reminders, commitments and scheduled tasks. |

Use typed entities, directed edges and assertions with source, trust, valid and recorded times. Exact lookup first, bounded traversal, FTS5, local ranking; optional local embeddings only after measured benefit. Compile minimal sufficient context including conflicts, provenance and policy before governor admission. Working context stays in bounded RAM. Support explicit memory operations, scoped retrieval, correction/supersession, export and deletion including derived indexes/caches. No raw audio, ASR partials or credentials in memory. Evaluate compact flash journal/CBOR on constrained targets.

Initial unmeasured targets: <8 MiB incremental memory overhead, 256 MiB aggregate data quota, warm p95 exact lookup <5 ms, traversal <20 ms, context compilation <50 ms, depth 2 default/3 maximum, zero remote retrieval tokens. Existing governor journal remains independent. SQLite is not itself encryption; owner access, encrypted storage/key recovery, retention and backup/deletion policies must be addressed before real private data.

Target identity design uses a stable agent ID and separate protected cryptographic credentials; persistent identity and enrollment are not implemented yet. Device enrollment, key rotation, recovery and revocation must be designed before cross-device identity synchronization is enabled. Private keys must never be sent to an inference provider.

## Technical approach

| Component | Initial choice |
| --- | --- |
| Agent runtime | Rust, with a small dependency footprint |
| OS foundation | Headless Linux for development and audio validation; Buildroot x86-64 image after the pipeline is proven |
| Event processing | Bounded asynchronous Rust tasks |
| Persistent state | AMG temporal property graph, provisional SQLite + FTS5; compact journal on constrained targets |
| Serialization | CBOR internally where useful; JSON for provider APIs |
| Networking | HTTPS/TLS, streaming WebSocket where supported |
| Voice | Rust coordinator; optional Sherpa-ONNX/Kokoro warm adapter, Whisper.cpp/Piper batch baseline; wake word, neural VAD, AEC and physical validation pending |
| Inference | OpenAI-first streaming Responses API adapter using eligible ChatGPT plan sharing; local rules and mock adapter as offline baseline |
| Inference policy | Local resolver and fixed-budget journal prototype implemented; context compiler, calendar/rolling budgets and authenticated broker permits planned |
| Security | Capability-based tool authorization, Ed25519 device identity, vetted cryptography |
| Embedded evolution | Rust `no_std` where practical; existing ESP-IDF/RTOS components when justified |

Use Linux first to avoid spending the initial development cycle on hardware drivers. Consider ESP32 firmware or a dedicated kernel only after profiling shows where the OS foundation creates a measurable bottleneck. Keep platform-specific code behind narrow interfaces; do not assume Linux binaries or libraries can simply be recompiled for a microcontroller.

## Reference hardware and resource budgets

Primary conversational reference: efficient x86-64 mini PC, Intel N100/N150-class minimum or equivalent; prefer a refurbished Core i5/Ryzen 5 with 16 GB RAM and 256 GB SSD if it meets the latency and power gates. A USB far-field microphone array with echo cancellation (e.g. ReSpeaker XVF3800) and a compatible speaker form the reference audio path. Exact SKU and audio routing remain subject to measured tests.

| Metric | Conversational host | Lightweight core / satellite |
| --- | --- | --- |
| Complete device cost | Target $300; indicative $230–$330 | Pi Zero 2 W core ~$35; MCU satellite TBD with audio |
| Average device input power | 6–12 W target | Core ~1.5 W; MCU target 0.1–0.5 W idle |
| Physical RAM | 16 GB preferred, 8 GB minimum candidate | Pi Zero 512 MB; ESP32-S3 PSRAM |
| OS + non-audio core RAM | 64 MB stretch target on specialized image | 64 MB core; 1–4 MB MCU target |
| Active voice stack and core RAM | Approximately 2.5 GiB planning budget | Voice inference offloaded to host |
| OS + non-audio runtime image | 128 MB stretch target, excludes model assets | 128 MB core; 4–16 MB MCU target |
| Local persistent agent memory | 256 MiB initial configurable aggregate quota | Hardware-dependent |
| Operation | 24/7 | 24/7 |
| General-purpose inference | OpenAI via eligible plan sharing; optional local model | Delegate to authorized host |

These are unvalidated engineering targets, not guarantees or price quotes. Host costs include audio and basic accessories; measure at the wall including audio equipment. Report OS, runtime, models and retained user data separately. Prioritize conversation quality over hardware price or an arbitrary footprint; report peak/idle RAM, whole-device energy, latency and quality together. Do not buy a dedicated GPU for the initial prototype.

### Voice quality gates

| Metric | Initial target |
| --- | --- |
| Wake detection | Under 200 ms after wake phrase ends |
| ASR | Faster than real time on reference hardware |
| End-of-turn detection | Under 400 ms for clear completed turns |
| Local spoken response | Under 500 ms after speech ends, simple intents |
| First audible OpenAI response | Under 1.5 s median after speech ends; report p95 |
| Barge-in to playback stop | Under 150 ms |
| Far-field operation | Evaluate 1, 3 and 5 m in quiet and noisy conditions |
| Continuity | Context survives sessions and restart |
| Idle remote inference | Zero |
 
Targets must be validated end to end; network latency is variable. A text-mediated voice pipeline may not match native speech-to-speech prosody or turn-taking. See [resource and benchmark plan](docs/resource-budget.md).

## Security and privacy

- Default-deny capabilities for filesystem, network, devices, tools and secrets.
- Require explicit confirmation for sensitive, irreversible or externally visible actions according to owner-configured policy.
- Treat model output, external content and tool results as untrusted; validate tool calls against typed schemas and authorization rules.
- Use secure transport, credential isolation, signed updates and encrypted storage where supported by the threat model and hardware.
- Support physical microphone mute where possible and clear indicators when capturing or transmitting audio.
- Maintain bounded audit logs of tool calls, policy decisions and task outcomes; redact credentials and sensitive payloads.
- Apply resource quotas, rate limits, cancellation, deadlines and recovery procedures to agent workloads. Prevent tools, extensions and background jobs from bypassing inference admission or accessing provider credentials.
- Provide a local owner-controlled shutdown and safe mode. Fail closed on authorization errors.

## Development roadmap

| Milestone | Deliverable / exit criteria |
| --- | --- |
| M0 — Design | README, architecture, threat model, resource and inference budgets, and reference hardware selection. |
| M1 — Voice-first agent core | Rust runtime, mock inference, identity and journal, local intent resolver and Token Governor. Parallel local voice spike: streaming ASR/TTS, VAD, barge-in and latency instrumentation. Validate official OpenAI eligibility before live calls. |
| M2 — Conversational integration | Integrate streaming OpenAI text responses with local voice pipeline, context compiler, model discovery and quota/error handling. Validate full-duplex interaction and no paid fallback. |
| M3 — Memory and scheduling | Bounded local retrieval, memory editing/export, durable reminders, approved workflow reuse, cancellation and restart recovery; budgeted consolidation. |
| M4 — Minimal OS | Bootable Buildroot x86-64 image for selected voice host, automatic startup, health checks and measured resource budgets. |
| M5 — Voice refinement | Improve acoustic robustness, prosody, turn-taking and far-field quality; tune power, latency and memory against measured baselines. |
| M6 — Tools and security | Typed tool calls, policy enforcement, approval flow, audit logging and adversarial tests before broad tool access. |
| M7 — Embedded port | Evaluate ESP32-S3 implementation, power management, storage durability and audio performance. |
| M8 — Distributed agent | Optional authorized node discovery, encrypted synchronization, conflict resolution and inference routing. Evaluate Nostr as an optional transport. |
| M9 — OS specialization | Compare optimized Linux, RTOS and custom kernel approaches using equivalent workloads. Proceed only with demonstrated benefits. |

M1 slices: M1.1 offline core, M1.2 audio smoke harness, M1.3 persistent voice coordinator (automated tests passed), M1.4 AMG identity/memory/retrieval/context compiler (design accepted). M1 exit still requires real model/device validation, measured acoustic interruption, activation evaluation and persistent identity. M2 requires official provider eligibility and spending controls. M3 expands memory management, scheduling and approved workflows rather than duplicating M1.4.

Security requirements apply from M1; M6 expands and verifies the tool execution surface rather than deferring basic security until then.

### First prototype acceptance criteria

1. Boot and start automatically on the reference board without a desktop environment.
2. Accept text input, resolve supported routine intents locally and stream an eligible OpenAI response through the Token Governor when inference is necessary; verify subscription-only controls before live use.
3. Preserve agent identity, conversation history and pending reminders after a controlled restart.
4. Continue local scheduling and safe commands when the inference provider is unreachable.
5. Recover from provider errors without an unbounded retry loop or duplicate side effects.
6. Measure image size, peak and idle RAM, CPU utilization, power, boot time and response latency. Document hardware and test conditions.
7. Verify zero remote inference while idle and no remote calls for deterministic test cases. Measure local resolution rate, total tokens per task, response quality and quota consumption on a reproducible workload.
8. Test budget exhaustion, concurrent reservations, restart recovery, denied scopes, unavailable allowance and provider quota limits. Demonstrate offline continuation, no paid fallback and no bypass of the Token Governor.

9. Demonstrate a complete microphone-to-speaker local conversation loop, barge-in, echo suppression and visible mute/listening states with deterministic mock inference; measure latency before enabling live OpenAI integration.

Voice is a first-milestone requirement. A CLI is a test interface, not the intended user experience.

## Repository layout and planned expansion

```text
atomos/
  README.md
  docs/             # Architecture, AMG, threat model, budgets, prototype notes
  crates/atomos-core/ # Rust domain, governor, audio, voice and tests
  apps/atomos-cli/    # CLI and streaming voice protocol
  scripts/          # Offline tests, benchmarks, optional audio adapters
  .github/workflows/ # CI
  # Planned: memory/inference/policy crates, platform/ and expanded benchmarks/
```

Design: [architecture](docs/architecture.md), [AMG architecture](docs/amg-architecture.md), [threat model](docs/threat-model.md), [resource budget](docs/resource-budget.md), [inference architecture](docs/inference-architecture.md). Implementation: [M1.1](docs/m1-prototype.md), [M1.2](docs/local-voice-spike.md), [M1.3](docs/persistent-conversation.md).

## Prior art to evaluate

- [MimiClaw](https://github.com/memovai/mimiclaw): microcontroller agent runtime.
- [XiaoZhi](https://github.com/78/xiaozhi-esp32): ESP32 conversational voice endpoint.
- [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw): Rust agent runtime.
- [NullClaw](https://github.com/nullclaw/nullclaw): small Zig agent runtime.
- [Letta](https://github.com/letta-ai/letta): stateful agent and memory architecture.
- [AIOS](https://github.com/agiresearch/AIOS): OS-level abstractions for agents.
- [Tock](https://github.com/tock/tock): embedded kernel and isolation architecture.
- [Buildroot](https://buildroot.org/): minimal embedded Linux images.

These are research references, not selected dependencies. Verify current upstream status, licenses, security properties and performance claims before incorporating any code.

## Open decisions

- Exact x86-64 host, microphone/speaker configuration and audio engine selection after benchmarks.
- Validate AtomOS eligibility for OpenAI plan sharing, account-level spending controls, supported model capabilities and data retention policies. Define headless sign-in/enrollment using only documented flows.
- Establish real-world Plus usage headroom and benchmark token efficiency without compromising response quality.
- AMG schema/bindings, FTS5 and query benchmarks, embedded engine comparison, retention, flash wear and encryption/key recovery.
- Physical model compatibility, full-duplex AEC, wake word/neural VAD, incremental TTS and hardware latency/power.
- CI checkout action Node.js deprecation and minimum Rust version verification.
- Runtime dependency choices and whether shared core code can remain portable to MCU targets.
- Distributed identity protocol and synchronization semantics.
- Project license and eventual public release. Apache-2.0 is a candidate; no license is granted by this planning document.
- Project name availability before a public release.

## Project principles

Own the agent and its memory. Use OpenAI intelligence deliberately, spend tokens only when they improve the result, and never incur additional AI costs automatically. Spend energy on useful work, preserve state across failure, and measure before optimizing.
