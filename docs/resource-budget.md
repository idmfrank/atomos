# Resource budget and conversational benchmarks

Status: engineering targets plus explicitly labeled measured text-only baselines. Physical voice, whole-device power and reference-device active-AMG measurements remain pending; CI scale benchmarks passed.

## Reference configuration

Efficient x86-64 mini PC: N100/N150-class baseline or better, 16 GB RAM preferred, 256 GB SSD, wired network where practical. Compare a refurbished Core i5/Ryzen 5 host against an efficient low-power new host. USB far-field array with hardware DSP (ReSpeaker XVF3800 is a candidate) and a speaker connected through an AEC-compatible playback path. Target complete cost $300 (planning range $230–$330), not a purchase authorization. Do not purchase GPU/NPU until profiling demonstrates benefit. Reuse an existing PC for initial tests.

| Budget | Target |
| --- | --- |
| Average whole-device power | 6–12 W across a defined workload |
| Annual electricity at $0.15/kWh | $7.88–$15.77 at that average |
| Core OS/runtime memory, excluding speech | 64 MiB stretch goal on specialized image |
| Core image excluding models | 128 MiB stretch goal |
| Initial persistent agent data | 256 MiB configurable quota |
| Physical RAM | 16 GiB preferred; test 8 GiB candidate |
| Idle remote inference | Zero |
| Raw remote tokens | 750,000/month local planning ceiling |
| Incremental AI charge | Zero; provider eligibility/controls must be verified |

Initial active RAM planning allocation: OS/core 64 MiB, context/cache 128 MiB, ASR 768 MiB, TTS 1024 MiB, audio processing 64 MiB, working buffers 512 MiB = 2560 MiB (2.5 GiB). This is a hypothesis; report process PSS/RSS and whole-system memory without double counting. Model storage and installed OS footprint are measured separately; no hard model storage ceiling until engines are selected. Keep the 64 MiB and 128 MiB core goals as optimization hypotheses, not reasons to degrade voice.

## Latency definitions and target gates

| Metric | Definition | Initial target |
| --- | --- | --- |
| Wake latency | End of wake phrase to activation | <200 ms |
| ASR real-time factor | Processing seconds / audio seconds | <1 |
| Endpoint latency | Annotated speech end to final turn commit | <400 ms for clear turns |
| Local response | Annotated speech end to first audible output | <500 ms for simple intents |
| Remote response | Annotated speech end to first audible output | median <1.5 s, report p95 |
| Barge-in | Near-end user speech onset to actual speaker stop | <150 ms |
| Far-field | 1, 3 and 5 m, quiet and noise | Report accuracy and failures |
| Conversation persistence | Restore identity/context after restart | Pass |
| Idle inference | Provider calls while resting with no due work | 0 |

Capture time points with monotonic clock; use audio loopback or independent microphone to confirm actual speaker timing, not just enqueue time. Separate endpointing, ASR, context, network/model first delta, chunking, TTS and playback delay. Report p50/p95, sample count and failures; do not omit failed requests. Remote latency is partly outside local control.

## Reproducible workload

Create versioned, consented or synthetic speech fixtures with transcripts, annotated endpoints and interruption times. Test short commands, open questions, follow-ups, corrections, long pauses, code-switching where supported, competing audio and agent speech. Include quiet/noisy rooms and distances 1/3/5 m; record microphone orientation, room, noise level and audio chain. Use at least 30 repetitions per latency scenario in initial comparisons, with cold/warm starts labeled; report uncertainty and limitations. Evaluate word error rate, semantic task success, false wakes, missed wakes, false endpoints, echo-induced activations, interruption success and owner-rated naturalness/prosody. Do not treat WER as equivalent to conversational quality.

Define a separate repeatable 100-event daily workload (25 open-ended turns, 50 routine commands/fact lookups, 25 timer/sensor events) for token tests. Track the actual local resolution rate, counting only correctly resolved events. Report conversation-only results separately. Include follow-ups, retries, background consolidation, cancellation and provider usage gaps. Compare against a baseline with full context/no local resolver on mock or eligible bounded tests; never exceed spending policy to benchmark.

## Power and cost method

Use an AC wall meter including power supply, microphone and speaker; record idle wake listening, active local voice, remote turn and playback. Measure energy per completed task and an overnight/24-hour idle run, then a representative mixed-use day. Report average watts, peak watts, Wh/day, joules/task, total setup cost and estimated 3-year cost. External host energy and network/provider energy are separate; report unknowns rather than assuming zero. A local model is optional only if task quality, latency and total energy justify it.

## Go/no-go

M1 must demonstrate interruptible local voice and mock inference without credentials; failures become issues before integration. M2 must validate eligibility, spending controls and streaming remote text, then compare complete voice quality with local baseline. Do not promise equivalence to native speech-to-speech ChatGPT. Choose the lowest total-cost host that meets quality, latency and security gates. If none passes, revise speech engine/hardware before reducing quality expectations.

## Verified text-only baseline and AMG targets

| Verified release baseline | Executable bytes | Peak RSS (KiB) | CI |
| --- | ---: | ---: | --- |
| M1.3 | 314,224 | 2,136 | [45752758740](https://github.com/idmfrank/atomos/actions/runs/45752758740) |
| M1.4a | 314,224 | 2,040 | [45754844830](https://github.com/idmfrank/atomos/actions/runs/45754844830) |
| M1.4b | 366,440 | 2,644 | [45755478646](https://github.com/idmfrank/atomos/actions/runs/45755478646) |
| M1.4c | 462,552 | 2,628 | [45796667551](https://github.com/idmfrank/atomos/actions/runs/45796667551) |
| M1.4d | 464,912 | 2,608 | [45798237897](https://github.com/idmfrank/atomos/actions/runs/45798237897) |
| M1.4e (schema v3) | 464,912 | 2,776 | [45803143396](https://github.com/idmfrank/atomos/actions/runs/45803143396) |
| M1.4f | 466,464 | 2,796 | [45807144684](https://github.com/idmfrank/atomos/actions/runs/45807144684) |
| M2.1 | 466,464 | 2,704 | [45833540395](https://github.com/idmfrank/atomos/actions/runs/45833540395) |
| M2.2 | 466,464 | 2,676 | [45839901448](https://github.com/idmfrank/atomos/actions/runs/45839901448) |

Each benchmark runs 1,000 local simulated text interactions without an active memory store or ledger. M1.4b links the memory crate and system SQLite; the binary grew by 52,216 bytes and peak process RSS differed by 604 KiB from M1.4a. Do not attribute the entire RSS change to SQLite or equate it with active-store overhead. Excludes OS, native speech engines, Python adapter and separate administration binary. No remote inference, acoustic latency or wall-power measurement. M1.4e must measure cold/warm active store, cache, indexes, sidecars, query latency and comparable alternative engines.

AMG provisional targets: <8 MiB incremental memory engine overhead within the overall RAM budget, 256 MiB configurable aggregate data quota including derived indexes and journal sidecars, warm p95 exact retrieval <5 ms, bounded traversal <20 ms, context compilation including retrieval <50 ms. Depth 2 default/3 maximum. Zero remote tokens for local retrieval. Compare equivalent SQLite, embedded graph and KV workloads before final engine selection. See [AMG benchmark protocol](amg-architecture.md#performance-and-comparison). Selective retrieval and CLI RSS targets met on the CI fixture; aggregate quota, reference hardware and energy remain unverified.

M1.4e [active-store benchmark protocol](m1-amg-benchmark.md) adds latency, attempted failures, fixture retrieval metrics, context bytes and same-binary CLI process RSS with/without store. CI 1k, manual 1k/10k/100k. Scale results passed; redb/CozoDB alternatives deferred, not benchmarked.

[Verified 100k scale baseline](m1-amg-optimization.md#reference-baseline-and-decision): selective context p95 0.375 ms; common-term p95 44.8992 ms; active CLI RSS 4,228 KiB (delta 1,628 KiB); main DB 116,084,736 bytes. Quiescent measurements exclude transient storage peaks. M1.4f adds aggregate file accounting, not hard quota enforcement; external filesystem/project quota and disk-full testing remain gates.

M1.4f [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45807144684): 466,464-byte release CLI, text-only RSS 2,796 KiB. [Scale run](https://github.com/idmfrank/atomos/actions/runs/45807347862): 100k entities, active CLI 4,152 KiB vs no-store 2,568 KiB; common-term p95 5.9669 ms vs same-run reference 60.7457 ms. Streaming 600,005-record export/restore verified, recovery peak 9,156 KiB. No physical power or hard-quota claims. M2.1 fixture accounting units are not provider tokens; no live spend.

M2.1 post-merge CI passed 61 Rust and 11 Python tests. No-store CLI size unchanged; broker example is separate, so no integrated broker footprint claim. Active 1K regression: 4,108 KiB peak RSS vs 2,672 KiB same-run no-store, 0.3194 ms selective context p95, 0.3896 ms common-term p95, zero parity mismatches. M1.4f was the verified combined scale and recovery baseline for M2.1. Synthetic broker charged 47 fixture units, zero real provider calls. See [M2.1 results](m2-inference-broker.md#verified-results).

M2.2 optional decoder benchmark (`scripts/test-responses.sh`) separately measures release binary size, peak RSS and p50/p95/p99/max latency for finite synthetic SSE replay (1,000 each at 1/64/4,096-byte chunk caps). No disk reconciliation, network, speech or live model calls; default no-store CLI benchmark remains separate. Results passed; see M2.2 measurements below.

M2.2 [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45839901448): 75 Rust / 11 Python, decoder example 361,776 bytes / 2,080 KiB RSS, p95 20.2/14.1/13.6 µs (1/64/4096 byte chunks), 3,000 synthetic replays without errors. [Full scale](https://github.com/idmfrank/atomos/actions/runs/45840592568): 100K active CLI 4,300 KiB vs 2,600 KiB baseline, selective context p95 0.2129 ms, common-term p95 1.6624 ms vs reference 26.0003 ms; export 85,222,570 bytes, restore 11.828 s, peak recovery 8,928 KiB. Zero mismatches across 90 comparisons. Write p95 tail increased; median stable, runner variability unisolated. M2.3 fixed synthetic policy journal 1 MiB, existing governor 4 MiB, no real account data or new dependencies. No measured integrated runtime overhead or provider output cap; preview route excludes max_output_tokens.

M2.3a [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45859290355): 466,512-byte CLI, no-store RSS 2,624 KiB; 86 distinct Rust + 11 Python tests. Decoder p95 34.6/28.8/21.6 µs (1/64/4096-byte reads), 2,116 KiB peak RSS; 3,000 replays zero errors. [Full scale](https://github.com/idmfrank/atomos/actions/runs/45859954016): 100K active CLI 4,212 KiB vs 2,548 KiB, selective context p95 0.2805 ms, common-term 2.0427 ms vs 29.1539 ms reference, forget 0.6421 ms, restore 9.041 s / 9,172 KiB; all checks passed. M2.3b optional Node auth-only process, not resident or linked; its actual resource cost unmeasured.

M2.3b [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45862345791): default CLI 466,512 bytes / 2,664 KiB RSS, 86 distinct Rust + 11 Python + 11 Node tests. Decoder p95 34.5/21.5/21.1 µs (1/64/4096 bytes), RSS 2,132 KiB, 3,000 samples zero errors. [Full scale](https://github.com/idmfrank/atomos/actions/runs/45862674857): 100K active CLI 4,232 KiB vs 2,780 KiB, common-term p95 3.2039 ms vs reference 46.3175 ms, selective context p95 0.4502 ms, forget p95 1.6534 ms, restore 18.747 s / 9,020 KiB; 90 parity comparisons zero mismatches. M2.3c scrypt memory cost ~64 MiB per cold-path derivation (not measured process RSS); unlock is intentionally nonresident and interactive, not part of runtime budget. No full integrated footprint claim.

M2.3c [CI](https://github.com/idmfrank/atomos/actions/runs/45865041099) 466,512-byte CLI / 2,524 KiB, 113 distinct tests. Decoder 362,368 bytes / 2,116 KiB, p95 19.0/12.5/19.6 µs at 1/64/4096-byte reads, zero errors. [Scale](https://github.com/idmfrank/atomos/actions/runs/45865299130) 100K active CLI 4,120 KiB vs same-run 2,572 KiB; selective p95 0.3548 ms, common 2.4256 ms vs 35.2295 reference, forget 3.2001 ms, restore 12.901 s / 9,100 KiB; zero parity errors. M2.4 separate cold-path Node HTTPS and Rust decoder process, not default runtime. Actual combined RSS, time to first visible text, wall power and subscription consumption unmeasured.

M2.4 verified 6dff964: [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45874158043), [full scale](https://github.com/idmfrank/atomos/actions/runs/45874467379), 123 distinct tests, default CLI 466,512 bytes. Full-scale no-store 2,588 KiB, 100K active 4,324 KiB (increment 1,736 KiB); selective p95 0.3957 ms, common 3.1946 ms vs 43.8345 reference, forget 1.5756 ms; restore 17.352 s / 9,028 KiB, zero parity/recovery errors. Decoder 2,188 KiB, p95 40.9/22.4/22.0 microseconds at 1/64/4096-byte reads. No live inference yet. [PROJECT_STATE.md](../PROJECT_STATE.md) records handoff and outstanding qualification gates.
