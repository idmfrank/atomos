# M2.4b — Offline Responses protocol compatibility

Status: proposed; no provider calls, no new live command. All three historical attempts remain consumed. Original raw response was deliberately not retained; unknown event after eight validated events at part stage is not identified.

## Public contract audit (2026-11-26)

[SIWC inference guide](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference): successful completion requires the terminal completed event; example consumes text deltas and handles failure, not a complete event validator. [Streaming API reference](https://developers.openai.com/api/reference/resources/responses/streaming-events): documents response.queued with queued snapshot, output_text.annotation.added with nullable annotation and item/part/annotation indices, text deltas and final snapshots, reasoning/refusal/tool events. Public examples illustrate sequence number 1; not a universal stream-start guarantee. [SDK source](https://github.com/openai/openai-node/blob/main/src/lib/responses/ResponseStream.ts) is a potential further reference, not executed or used as a differential oracle here (raw source retrieval unavailable). General API features do not imply entitlement under SIWC.

| Event class | AtomOS candidate policy |
| --- | --- |
| Core text lifecycle | Same strict reference rules and expected text/usage. |
| Created/queued/in-progress | Same response ID, empty output and no usage/error/incomplete; one queued event before progress, queued creation must progress before output. |
| Sequence numbering | Start 0 or 1 only, strictly consecutive; independent 128-event bound. |
| Annotation added | Explicit null only, active part, matching item/index, consecutive annotation indices; no actual annotations in snapshots. This is a narrow candidate no-op policy, not a claim about provider final annotation behavior. |
| Non-null annotations, reasoning items/events, refusal, tools, audio, other capability-bearing events | Deny; never execute or expose. |
| Unknown events | Deny; never generic skip or usage extraction from unvalidated stream. |
| Failure, incomplete, error | Existing typed failure; no partial text; known validated usage only. |
| Missing, duplicate or mismatched terminal; invalid usage | Deny. |

New decode_compat API is offline only; decode/decode_diagnostic, broker and owner harness retain strict reference. Same 256 KiB wire, 32 KiB frame, 128 events, 4096 text bytes, strict JSON and EOF, cancellation. No dependencies or schema changes. Diagnostic labels distinguish queue/annotation without raw event names. The synthetic auxiliary fixture is derived from existing fixture with independently specified expected output and usage. Not a recorded provider response.

Five added Rust tests cover original-wire parity, fragmentation, queue identity/order, null annotations and explicit capability denial, sequence/terminal/usage/limits, failure handling. Benchmark repeats strict reference, candidate on identical reference fixture and candidate auxiliary fixture (1000 samples each at 1/64/4096 chunk sizes). Compare equal fixture for overhead; auxiliary fixture has different event count/bytes. No claims of full provider compatibility or actual SDK parity.

Next: evaluate documented reasoning/refusal lifecycle and model-specific no-tools profiles with separate fixtures; account-bound Governor integration offline. Do not add a fourth live attempt or reset reservations. Production admission, provider generation ceiling and actual token settlement remain gated.
