# AtomOS threat model

Status: active engineering threat model, target controls plus implemented boundaries below; not production security certification. PR #33 post-merge CI passed; last full-scale evidence remains PR #32, with unchanged PR #31 M2.5a runtime. CLI MVP scope replaces the former voice-first release gate. [Architecture](architecture.md) | [Current state](../PROJECT_STATE.md).

## CLI MVP security boundary

The [CLI MVP](cli-mvp-roadmap.md) uses one unprivileged owner process through existing SSH/local login. No new network listener, browser session, multi-user service or arbitrary shell/tool execution is introduced. Reuse host authentication; do not change SSH configuration or add a new service under this roadmap. A later web interface needs a separate authentication/session/CSRF and hosting review.

Treat terminal input, retrieved text and model output as untrusted at their respective boundaries. SSH does not authorize model-directed actions, private AMG disclosure or credential access. Do not print tokens, raw provider diagnostics or private memory in logs. Terminal echo/scrollback, `tmux` buffers, shell history, redirected output and crash dumps can retain content; full transcript logging is outside the MVP. Explicit memory operations remain owner-controlled and fact values enter on stdin.

Define cancellation, EOF, disconnect and process termination behavior. Preserve unknown usage and stale-crash-lock investigation; disconnect is not proof of cancellation/refund. Keep scope, journal caps, budget checks and fail-closed storage behavior. No unattended inference or automatic retries.

Voice risks and controls below are retained for optional later use. Physical mute, AEC and bystander-capture qualification are deferred; they no longer block the text MVP. Existing offline tests remain mandatory.

## Verified local deployment and retained gates

The [Linux CLI setup](linux-cli-deployment.md) uses the explicitly selected owner account for build, release and data; package administration is separate. Existing SSH public-key access was authorized for that account without copying a private key or changing sshd/network configuration or sudo privileges. Source stays clean at the tested SHA; copied release hashes, store integrity and installed local/mock behavior were verified. The private new store contains generic identity only, with no seeded personal facts. Public docs omit actual account/host/key identifiers.

A tmux session is process continuity, not authenticated private-memory deployment, credential unlock, transcript persistence, reboot recovery or automatic service startup. No real owner inference vault, raw provider stream or private AMG data was accessed/copied. Same-UID/root/rollback and storage power-loss limitations remain. Benchmark temp stores must be explicitly placed on the intended persistent filesystem before making device-durability claims; tmpfs evidence remains separately labeled.

## Planned M2.5b outcome boundary

The [offline integration contract](m2-text-outcomes.md) preserves strict/extended parser caps and explicit profile selection. Refusal and failure carry no answer/raw refusal/reasoning text. Validated known usage settles once; unknown/cancelled/invalid outcomes retain the reservation. Rate/auth failures and overrun suspend durably. Admission, settlement and revocation storage faults must suppress delivery and poison the instance; complete surviving records may have committed despite a returned sync error. Forced termination leaves a stale writer lock for investigation, not automatic cleanup. These are planned acceptance requirements, not newly verified runtime controls.

## Assets and trust boundaries

Protect owner identity, OAuth tokens, device keys, conversation history, audio, stored facts, task history, spending allowance and the integrity of tool actions. Boundaries: physical room to microphone; audio workers to runtime; untrusted model responses and tool content to policy; local state to provider; owner to agent; future satellite to host.

Assume malicious speech or audio playback, untrusted webpages/documents, prompt injection, malformed model output, compromised tools, stolen devices and interrupted power/network. Physical access and host administrator compromise cannot be fully defeated in M1. Do not claim speaker recognition proves authorization.

| Threat | Required mitigation |
| --- | --- |
| Deferred voice: background listening / bystander capture | Local wake gating, bounded volatile pre-roll, visible listening state, physical mute where supported, no raw audio retention by default. |
| Deferred voice: replay, TV, or impersonated voice | Voice alone cannot approve sensitive operations. Require authenticated owner interaction out of band; prevent TTS feedback from becoming a command. |
| Prompt injection | Separate instructions from retrieved data; validate typed tool requests; no model-directed privilege elevation. |
| Tool misuse / duplicate actions | Default deny, scoped capabilities, explicit approval for sensitive or externally visible actions, idempotency keys and durable outcome journal. |
| Token or money exhaustion | Single inference broker, durable reservations, rolling budgets and task ceilings; fail closed on unverifiable spending controls or exhausted provider allowance. |
| Credential theft | OS-restricted credential store, no secrets in logs or prompts, least privilege, revoke and rotate credentials. No token access by speech workers/tools. |
| Memory poisoning / leakage | Provenance, trust labels, scoped retrieval, correction and deletion, minimal transmitted context and redacted logs. Treat inferred facts as unconfirmed. |
| Denial of service | Bounded buffers, input lengths, retries, deadlines and worker supervision; preserve local stop/mute and safe mode. |
| Supply chain | Pin dependencies, review licenses, scan advisories, verify model checksums, signed update design before distribution. |
| Device loss | Restrict local access, evaluate disk encryption and secure key storage/recovery; document residual physical-access risk. |
| Cross-device abuse | Defer remote enrollment until mutual authentication, revocation, scoped access and replay protection are implemented. |

## Minimum implementation controls

Run the CLI as the unprivileged owner, restrict control to the existing terminal session and disable arbitrary shell execution. If a service is later approved, use a dedicated restricted account. Keep audio out of logs in the retained optional voice path. Do not expose network listeners or tools in M1 unless necessary and explicitly authorized. Restrict credential access to the broker; never store secrets in repository, build artifacts or fixtures. Model adapters return data, not executable instructions.

Mute stops capture and discards pending volatile audio; software mute is not equivalent to a physical disconnect. An interruption must stop stale audio and prevent uncommitted actions, but an already committed external operation may not be reversible. Report outcome honestly.

## Security gates

- Mock-provider tests for denied, malformed, duplicate and stale tool calls; no arbitrary execution.
- Test budget persistence after crash, concurrent admissions, cancelled streams and missing usage records.
- Verify no remote calls in idle/offline/muted tests; inspect outbound payloads for unnecessary sensitive data.
- Confirm official sign-in eligibility and subscription-only settings before live tests. No API-key fallback, automatic credits or limit circumvention.
- Test memory export, correction and deletion, log redaction and recovery.
- If voice is resumed, verify physical mute and listening indicators before normal household use; deferred from CLI acceptance.

Residual risks: provider-side retention and account-wide usage are outside local control; audio transcription can misinterpret speech; physical compromise and upstream vulnerabilities remain possible. Document provider policies and hardware properties before deployment. AtomOS is not a safety-critical or emergency response device.

## AMG memory-specific controls (target and partial implementation)

See [AMG architecture](amg-architecture.md). Explicit owner scope and authenticated sensitive mutations; typed queries, bounded traversal, source trust and temporal conflicts; model-inferred data cannot upgrade its authority. Delete or invalidate dependent assertions, derived indexes/caches and queued context. No credentials, raw audio or ASR partials in memory. SQLite is not encrypted by default; address file permissions, encryption/key recovery, WAL/FTS deletion caveats and backups before private deployment. Logical deletion is not forensic erasure; transmitted context cannot be recalled. Test prompt injection, cross-scope reads, rollback, corruption, disk-full and restoration without deleted-record resurrection. Synthetic CI data only.

### Verified M1.4b boundary

Conversational access is opt-in and read-only both at the typed Rust interface and SQLite open mode. Voice accepts only demonstration owner preference text fields (favorite_color, favorite_food, favorite_music) and display name; this is not a sensitivity classifier or authenticated authorization. Anyone in earshot can hear responses. Text administration trusts the OS account; owner scope is not authentication. No real private data yet. Missing stores, wrong scope, stale finals and mutation requests are covered by regression tests; no automatic learning or remote inference. Corrected/deleted values are re-read on subsequent turns, but already spoken content is not retractable. Remaining controls include authenticated scope/sensitivity policy, encryption, full source/derived-data deletion, backup/restore, crash/disk-full and aggregate quota validation. See [M1.4b](m1-amg-conversation.md).

### M1.4d–f implemented boundaries

Schema v2 adds default-deny revision-specific expiring voice grants, explicit revoke and grant retirement on correction/deletion. Restore never carries grants. Logical backup/restore preserves scope, identity and history but is plaintext and not authenticated; trusted private directory only. Explicit new-directory restore may resurrect older deleted data; retention reconciliation and encrypted backup/key recovery remain gates. No provider disclosure, no real private data. See [M1.4d](m1-amg-recovery.md).

M1.4e/f adds affected-record deletion indexes, scoped reference parity and bounded streaming backup verification. Hard aggregate quota, forensic erasure, encrypted backup, restore rollback/reconciliation and authenticated private-memory use remain open.

## M2 verified boundaries and residual risks

- M2.5a [atomic admission candidate](m2-atomic-admission.md): synthetic account/host/grant/model/version binding, SHA-256 immutable request identity, one synced admission/reservation record, private 0700/0600 namespace, exclusive writer and bounded replay. Torn/malformed/missing/incompatible records fail closed; no reset or migration. Unknown dispatch retains reservation and blocks admission; revocation and provider suspension are durable. This trusted-local-directory prototype does not protect against same-UID tampering, hash-based content guessing, rollback, root compromise, malicious filesystem/hardware or untrusted clock sources. No live transport, credentials or private AMG egress is introduced.

- Owner-run official OAuth PKCE, signed identity, scope validation, explicit refresh, encrypted private vault and catalog succeeded. Revocation untested; same-UID/root tampering, rollback, trusted time, autonomous unlock and secure broker credential handoff remain unresolved. No credential access by speech workers. See [auth hardening](m2-auth-hardening.md).
- Offline governor and synthetic authorization reserve before dispatch, retain unknown usage, and halt/suspend under documented conditions. Integrated atomic account-bound request admission, rolling/monthly windows and real usage reconciliation remain open. Provider output ceiling is unavailable on the documented preview route; local byte/deadline caps cannot guarantee upstream generation limits. Owner settings observed, not a programmatic billing attestation.
- Three historical fixed live attempts consumed; preserve all markers, no fourth attempt, no paid fallback or alternate vault workaround. Final body reached strict decoder, no completed response or usage validated. Sanitized diagnostics only. [Qualification closure](m2-first-live-qualification.md).
- Offline compatibility profiles validate bounded known lifecycle events, opaque reasoning and typed refusal. Unknown events, tool execution and raw reasoning are denied. No live broker or private AMG disclosure; refusal does not trigger fallback. [Protocol compatibility](m2-protocol-compatibility.md).
