# M2.2 — Bounded Responses SSE/JSON replay

Status: draft implementation; locked feature-enabled CI pending. No live
provider traffic, OAuth, credentials, account eligibility or private AMG
disclosure. Existing local text and voice remain mock-only.

## Protocol source and scope

Reviewed 2026-11-26: [Responses streaming events](https://developers.openai.com/api/reference/resources/responses/streaming-events),
[streaming guide](https://developers.openai.com/api/docs/guides/streaming-responses)
and [create response HTTP stream example](https://developers.openai.com/api/reference/resources/responses/methods/create).
The test fixture is synthetic, not a captured provider response. It follows
the documented text lifecycle and includes Unicode and token details.
This is a narrow supported profile, not full API compatibility or authorization
to connect. Reverify current contract and eligible models before live transport.

One response, one assistant message at output index zero, one output_text
part at content index zero. Created, optional in-progress, item added, part
added, one or more deltas, text done, part done, item done, completed.
Every event requires consecutive sequence numbers starting at zero; optional
SSE event name must match JSON type. Response/item IDs and indices must stay
consistent. Final text must match accumulated deltas at all three done
boundaries and terminal response output. Failed and incomplete responses
must bind to the same response ID; error event may occur before creation.
No tools, reasoning items, refusal content, annotations, multiple messages,
audio, images, resume, steering, WebSocket or arbitrary event types; unsupported
input fails closed. In particular reasoning-capable models may emit unsupported
items even for ordinary questions. Do not enable live inference until supported
model behavior is verified or the necessary profiles are implemented. Output
reasoning token counts are handled independently of reasoning item support.

## Bounded decode

Optional core feature `responses-replay`: serde 1.0.228 and serde_json 1.0.145
pinned, Cargo-generated lockfile (seven new resolved packages; default
runtime remains free of JSON dependencies). No hand-written JSON grammar;
custom serde visitor rejects duplicate object keys at all depths. Standard
serde_json recursion limit remains enabled. Only one bounded JSON event tree
is retained at a time, plus accumulated text. Unknown metadata fields are
parsed with bounds but not interpreted. This is schema validation of
consumed fields, not full Responses object validation.

| Limit | Value |
| --- | ---: |
| Entire finite replay including comments | 256 KiB |
| Individual SSE line and accumulated data payload | 32 KiB each |
| Events | 128 |
| Accumulated output text | 4,096 UTF-8 bytes |
| Response/item ID | 128 ASCII bytes |
| Read buffer | 4,096 bytes |

Supports arbitrary byte fragmentation, LF/CRLF/CR, comments, optional event
name, multiple data fields joined with LF, UTF-8 and JSON escapes. Strict
blank-line event boundary and EOF after terminal; no extra data frames or
duplicate terminals. Unknown SSE fields and undocumented [DONE] sentinel
rejected. No full wire buffer, no private logging. Output text disallows
control characters except tab/newline. Empty or whitespace-only completion
rejected. Cooperative cancellation checked at each read and before return;
interrupted reads treated as cancellation, other I/O errors unknown usage.
A blocking Read is not timed or interruptible by this decoder. Future
transport must provide asynchronous cancellation, deadline and bounded
backpressure, including real end-of-stream semantics; finite replay is not
the live network contract.

## Broker and accounting

`Broker::run_sse` validates active turn and local intent and reserves durable
synthetic budget before reading any bytes. Local or denied/stale requests
never read the source. Parser emits one owned validated result, translated
to typed replay using the same M2.1 settlement path. No public detachable
permit. Local turn is bound by caller; Responses events do not carry AtomOS
turn IDs. Future transport must securely bind response identity to the
authorized request and recheck current epoch during delivery.

Usage requires integer input/output/total and cached/reasoning details;
total equals checked input + output and details are subsets. Optional
cache-write detail validated but not added. Completed response settles
reported fixture units; validated failed/incomplete response with usage also
settles via new typed FailedWithUsage event, never releases partial text.
Missing/invalid usage, unsupported/malformed events, cancellation, EOF or
I/O errors retain reservation; no refund or retry. Authoritative cap overrun
atomically settles and halts the ledger as in M2.1. Rate/auth failure suspends
the current broker instance; restart preserves charges, not suspension.
If an unsupported or malformed event precedes usage, the final usage is
unknown and not extracted from untrusted bytes. No live billing inference
or guarantee the reservation covers actual usage. Real transport will need
durable provider suspension and reconciliation.

## Tests and measurement

```bash
cargo fetch --locked
bash scripts/test-responses.sh
```

Independent expected text/usage fixture plus differential typed versus wire
replay (answer, charged units, dispatch count, exact ledger bytes, restart);
failure-with-usage parity, no-read admission proof, malformed JSON/duplicate
keys, UTF-8/escapes, fragmented reads, sequence/ID/index/text mismatches,
duplicate terminals, cancellation, EOF/I/O, event/line/frame/total/output/
nesting limits, rate/auth and usage overrun. Feature-enabled CI runs these
in addition to existing default workspace suite. Decoder-only release
benchmark: 1,000 iterations each at 1/64/4,096-byte chunk limits, p50/p95/p99/
max, errors, executable bytes and process peak RSS using GNU time. No disk
settlement or provider latency included. Expected results pending CI; not
a measurement of the integrated runtime.

Next: M2.3 authorization and account policy design; verify official plan
eligibility, durable account/request/expiry/window metadata, spending
controls and secure credential management. Before live integration also
qualify the selected model's actual event profile, enforce transport
deadline/cancellation and expand synthetic replay cases where needed.
No paid fallback or private AMG context.
