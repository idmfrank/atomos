# AtomOS threat model

Status: proposed, M0. Applies from the first executable prototype. [Architecture](architecture.md).

## Assets and trust boundaries

Protect owner identity, OAuth tokens, device keys, conversation history, audio, stored facts, task history, spending allowance and the integrity of tool actions. Boundaries: physical room to microphone; audio workers to runtime; untrusted model responses and tool content to policy; local state to provider; owner to agent; future satellite to host.

Assume malicious speech or audio playback, untrusted webpages/documents, prompt injection, malformed model output, compromised tools, stolen devices and interrupted power/network. Physical access and host administrator compromise cannot be fully defeated in M1. Do not claim speaker recognition proves authorization.

| Threat | Required mitigation |
| --- | --- |
| Background listening / bystander capture | Local wake gating, bounded volatile pre-roll, visible listening state, physical mute where supported, no raw audio retention by default. |
| Replay, TV, or impersonated voice | Voice alone cannot approve sensitive operations. Require authenticated owner interaction out of band; prevent TTS feedback from becoming a command. |
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

Run the host under an unprivileged service account, bind control interfaces to local access by default, keep audio out of logs and disable arbitrary shell execution. Do not expose network listeners or tools in M1 unless necessary and explicitly authorized. Restrict credential access to the broker; never store secrets in repository, build artifacts or fixtures. Model adapters return data, not executable instructions.

Mute stops capture and discards pending volatile audio; software mute is not equivalent to a physical disconnect. An interruption must stop stale audio and prevent uncommitted actions, but an already committed external operation may not be reversible. Report outcome honestly.

## Security gates

- Mock-provider tests for denied, malformed, duplicate and stale tool calls; no arbitrary execution.
- Test budget persistence after crash, concurrent admissions, cancelled streams and missing usage records.
- Verify no remote calls in idle/offline/muted tests; inspect outbound payloads for unnecessary sensitive data.
- Confirm official sign-in eligibility and subscription-only settings before live tests. No API-key fallback, automatic credits or limit circumvention.
- Test memory export, correction and deletion, log redaction and recovery.
- Verify physical mute and listening indicators on reference hardware before normal household use.

Residual risks: provider-side retention and account-wide usage are outside local control; audio transcription can misinterpret speech; physical compromise and upstream vulnerabilities remain possible. Document provider policies and hardware properties before deployment. AtomOS is not a safety-critical or emergency response device.

## AMG memory-specific controls (design accepted, not yet implemented)

See [AMG architecture](amg-architecture.md). Explicit owner scope and authenticated sensitive mutations; typed queries, bounded traversal, source trust and temporal conflicts; model-inferred data cannot upgrade its authority. Delete or invalidate dependent assertions, derived indexes/caches and queued context. No credentials, raw audio or ASR partials in memory. SQLite is not encrypted by default; address file permissions, encryption/key recovery, WAL/FTS deletion caveats and backups before private deployment. Logical deletion is not forensic erasure; transmitted context cannot be recalled. Test prompt injection, cross-scope reads, rollback, corruption, disk-full and restoration without deleted-record resurrection. Synthetic CI data only.
