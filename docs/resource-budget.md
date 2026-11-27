# Resource budget and conversational benchmarks

Status: CLI MVP measurement priorities and historical targets; PR #32 post-merge baseline is verified below and in [PROJECT_STATE](../PROJECT_STATE.md). M2.5a isolated admission measurements are in [atomic admission](m2-atomic-admission.md). Physical voice, whole-device power and reference-device active-AMG measurements remain pending; CI scale benchmarks passed.

## CLI MVP measurements and current baseline

The [CLI MVP roadmap](cli-mvp-roadmap.md) uses the existing Linux/SSH host and requires no audio gear, new hardware or custom image. Measure local terminal response, context compilation, admission/settlement, startup, active-store RSS/storage and binary size. Later authorized provider tests report first validated/displayed text, completion, cancellation, errors and actual usage separately; do not reuse speech timing targets as CLI claims.

One foreground session, no scheduler/background inference and zero idle calls. Capture p50/p95/p99 with sample/error counts and test-host/build details. Optional `tmux` continuity is an operator arrangement, not runtime recovery. Full transcript logging, new purchases, dependencies and host configuration are not authorized.

Last full-scale measurement baseline (PR #32) `5bca8a15e8245e9631d8a978a988c01cdc812f7a`: [CI](https://github.com/idmfrank/atomos/actions/runs/46025742630), [full scale](https://github.com/idmfrank/atomos/actions/runs/46026109238), same 173 distinct tests and 18,000 decoder samples; CLI 466,512 bytes, decoder RSS 2,180 KiB. Admission reference/candidate p95 0.402973/0.216750 ms, traced fsyncs 2/1, RSS 2,236/2,200 KiB, harness bytes 416,520/440,144 and journal bytes/request 52.560/104.280. Charge 14,100 each, candidate background 7,050, zero errors. 100K CLI 4,232 KiB vs no-store 2,592; selective p95 0.3143 ms, common 2.5666 vs 38.7607 reference, restore 14.1180 s / 9,192 KiB. [Full tables](../PROJECT_STATE.md#latest-verified-test-and-benchmark-baseline).

Hosted synthetic timings are not controlled cross-run comparisons or whole-device/energy/billing evidence. Older records below remain historical. This documentation revision's CI is pending until run.

PR #33's documentation-only CLI roadmap is now merged at `c563599f2e2c8b5abb7e03c75bb85e67e831e75d`; [post-merge CI 46029012445](https://github.com/idmfrank/atomos/actions/runs/46029012445) passed with unchanged runtime. Its CI admission/decoder/text/1K smoke checks do not replace the separately labeled PR #32 full-scale measurements above.

## M2.5b benchmark plan, unrun

The [implementation contract](m2-text-outcomes.md#benchmark-contract) requires equal-wire frozen strict-atomic versus extended-atomic measurements on identical policies, request sequences, state, filesystem and durability. Preserve 300-request mixed-priority independent expectations (14,100 total / 7,050 background), separate compilation/validation, first-read admission, complete decode/settlement and restart timings, actual syscall traces, peak RSS, harness/CLI bytes and journal growth. Auxiliary/reasoning/refusal workloads are reported separately with typed outcome/error counts; different fixtures are not equal-input overhead evidence. Full runtime regression/1K/10K/100K validation remains required for the future implementation PR. No M2.5b measurements or performance improvement are claimed by this documentation-only contract.

## Historical evidence and deferred voice reference configuration

Historical verified main `a91ccd9e9190334f1d7259c9413e40494121a01c`: [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/46024100103) / [scale](https://github.com/idmfrank/atomos/actions/runs/46024473201), all 173 tests, Rust 1.75 core targets and 18,000 decoder samples passed. Admission p50/p95/p99 reference 0.563104/0.641059/0.777855 ms vs candidate 0.276983/0.324064/0.413111 ms; trace fsyncs 2/1, write syscalls 4/2. RSS 2,260/2,264 KiB, harness size 416,520/440,144 bytes, journal bytes/request 52.560/104.280, exact charge 14,100 each, zero errors/parity. Decoder process 2,204 KiB. 100K CLI 4,268 KiB vs 2,612 no-store; selective p95 0.2966 ms, common 1.9801 vs 30.1055 reference, restore 9.1048 s / 8,988 KiB. Earlier head 10K remember tail returned to 0.6367 ms p95; cause unisolated. [Full post-merge method/results](m2-atomic-admission.md#post-merge-main-verification). Both runtime PRs were merged externally, not by the agent; the subsequent PR #32 documentation reconciliation is merged.

Historical executable follow-up `3895b2bd1dd3c113397b9d685be03350156aa88d`: [CI](https://github.com/idmfrank/atomos/actions/runs/46023799399) / [full scale](https://github.com/idmfrank/atomos/actions/runs/46023520346), 173 distinct tests plus Rust 1.75 core verification, 18,000 decoder samples, no live calls. Reference/candidate admission p50/p95/p99 0.332693/0.428936/0.631346 vs 0.161890/0.206696/0.363438 ms; trace write syscalls 4/2, fsyncs 2/1; process RSS 2,240/2,200 KiB; harness bytes 416,520/440,144. Same 14,100-unit charge, zero errors/parity mismatch, journal 52.560/104.280 bytes per completed request. Candidate memory validation p95 0.171 microseconds; compilation p95 0.651 microseconds. [Complete benchmark](m2-atomic-admission.md). Full-scale 100K selective p95 0.3302 ms, common 2.6488 vs 37.0744 reference, active CLI 4,288 KiB vs 2,580 no-store, restore 14.0874 s / 9,240 KiB; elevated 10K remember p95 8.8151 ms is retained, cause unisolated. No whole-system/energy claim. PR #30 main 2bd3ade was merged externally and passed post-merge CI/scale; the PR #31 time/task follow-up was subsequently merged externally; the subsequent documentation-only reconciliation merged in PR #32. Prior evidence below is historical.

M2.5a candidate resource bounds: one 1 MiB journal, records at most 512 bytes, at most 1 MiB + 1 byte replay input and constant accounting/pending state. Optional pinned `sha2` 0.10.9 provides compiled identity without custom cryptography; no default runtime dependency is added. Existing strict decoder retains its own caps. `scripts/benchmark-admission.sh` separates memory validation/compilation from fsynced admission, complete replay/settlement and restart; syscall tracing is a separate run, with per-process RSS and executable sizes reported. Local storage and clock trust remain explicit limitations.

PR #29 [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45919844209) / [scale](https://github.com/idmfrank/atomos/actions/runs/45919947851) verified on 5f564ad: same 150 tests and 18,000 replays, CLI 466,512 bytes; 100K active 4,196 KiB vs no-store 2,588 KiB, selective p95 0.2051 ms, common 1.6446 vs 24.9433 reference, restore 9.8047 s / 9,108 KiB; decoder process 2,116 KiB. All parity/recovery checks passed. Historical PR #28 evidence below is preserved. M2.5a local WSL final admission results: reference/candidate p95 8.6549/5.2026 ms, trace fsyncs 2/1, harness sizes 416,528/440,152 bytes, RSS 2,312/2,336 KiB, journal bytes/completed request 52.560/104.280. Larger journal buys durable request identity. No inference, energy, whole-system memory or hardware certification claim; full local scale and caveats are in PROJECT_STATE and the candidate document.

The following reference appliance, audio allocation, speech workload and latency targets are deferred voice design; they are not CLI MVP requirements or purchase authorization.

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

## Deferred voice latency definitions and target gates

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

## Deferred voice and future mixed-use workload

Create versioned, consented or synthetic speech fixtures with transcripts, annotated endpoints and interruption times. Test short commands, open questions, follow-ups, corrections, long pauses, code-switching where supported, competing audio and agent speech. Include quiet/noisy rooms and distances 1/3/5 m; record microphone orientation, room, noise level and audio chain. Use at least 30 repetitions per latency scenario in initial comparisons, with cold/warm starts labeled; report uncertainty and limitations. Evaluate word error rate, semantic task success, false wakes, missed wakes, false endpoints, echo-induced activations, interruption success and owner-rated naturalness/prosody. Do not treat WER as equivalent to conversational quality.

Define a separate repeatable 100-event daily workload (25 open-ended turns, 50 routine commands/fact lookups, 25 timer/sensor events) for token tests. Track the actual local resolution rate, counting only correctly resolved events. Report conversation-only results separately. Include follow-ups, retries, background consolidation, cancellation and provider usage gaps. Compare against a baseline with full context/no local resolver on mock or eligible bounded tests; never exceed spending policy to benchmark.

## Later physical-device power and cost method

Use an AC wall meter including power supply, microphone and speaker; record idle wake listening, active local voice, remote turn and playback. Measure energy per completed task and an overnight/24-hour idle run, then a representative mixed-use day. Report average watts, peak watts, Wh/day, joules/task, total setup cost and estimated 3-year cost. External host energy and network/provider energy are separate; report unknowns rather than assuming zero. A local model is optional only if task quality, latency and total energy justify it.

## Go/no-go

Use [CLI MVP acceptance](cli-mvp-roadmap.md#cli-mvp-acceptance-criteria) for the first release: local identity/fact persistence and offline continuation, bounded text context, explicit status/cancellation, correct durable admission/settlement and separately gated real conversation. Preserve existing audio regression tests without requiring physical audio qualification. No measured provider quality, token savings or energy result is implied by hosted fixtures.

If voice is later resumed, require complete acoustic/device quality, latency and interruption qualification before declaring that interface usable. The former M1 microphone-to-speaker gate no longer blocks the CLI path.

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

M2.4 diagnostics [CI](https://github.com/idmfrank/atomos/actions/runs/45912523399) / [scale](https://github.com/idmfrank/atomos/actions/runs/45912716094): 126 tests; CLI 466,512 bytes, 100K 4,272 KiB vs same-run no-store 2,608 KiB, selective p95 0.2671 ms, common 2.0268 ms vs 30.5340 reference, forget 1.1802 ms, restore 11.982 s / 9,188 KiB. Zero parity/recovery errors. Decoder p95 25.2/18.0/17.9 µs, RSS 2,116 KiB. No successful inference validated; original 1,024 planning units retained unknown. See PROJECT_STATE for all three scales.

PR #23 documentation-only merge [CI](https://github.com/idmfrank/atomos/actions/runs/45914518212) / [scale](https://github.com/idmfrank/atomos/actions/runs/45914610881): 130 distinct tests, 466,512-byte CLI, 100K 4,248 KiB vs no-store 2,620 KiB (increment 1,628 KiB); selective p95 0.4279 ms, common 2.9899 ms vs 44.5215 reference, forget 1.6698 ms, restore 17.412 s / 9,096 KiB. Zero parity/recovery errors. Decoder p95 16.0/11.6/11.2 µs, RSS 2,204 KiB. Hosted timing variation; no runtime performance change inferred. Two consumed unknown live attempts, 2,048 local planning units, no successful inference.

PR #24 [CI](https://github.com/idmfrank/atomos/actions/runs/45914977721) / [scale](https://github.com/idmfrank/atomos/actions/runs/45915135417): 135 tests; 466,512-byte CLI; 100K active 4,116 KiB vs no-store 2,768 KiB, increment 1,348 KiB; selective p95 0.3958 ms, common 3.4682 ms vs reference 44.6299 ms; forget 1.1810 ms; restore 17.309 s / 9,048 KiB; decoder p95 23.4/15.0/13.4 µs, RSS 2,236 KiB. Zero parity/recovery errors and no provider calls. No live inference validated.

PR #25 [CI](https://github.com/idmfrank/atomos/actions/runs/45916700932) / [scale](https://github.com/idmfrank/atomos/actions/runs/45916868587): 140 distinct tests; CLI 466,512 bytes; 100K active 4,152 KiB vs no-store 2,684 KiB, increment 1,468 KiB; selective p95 0.4165 ms, common 2.9926 ms vs 44.7725 reference, forget 1.5467 ms, restore 17.542 s / 9,152 KiB; decoder p95 26.6/17.6/17.8 µs, RSS 2,132 KiB. Zero parity/recovery errors. Owner final live probe: 874 ms headers, 875 ms first body byte (not TTFT), 1591 ms end, 8465 bytes, eight validated events then Unsupported; actual usage unknown. All three attempts consumed, 3072 retained planning units, no fourth attempt. No combined process/energy measurement or completed inference claim.

PR #26 documentation-only [CI](https://github.com/idmfrank/atomos/actions/runs/45917707265) / [scale](https://github.com/idmfrank/atomos/actions/runs/45917825275): 140 tests, CLI 466,512 bytes; 100K active 4,204 KiB vs no-store 2,572 KiB (increment 1,632 KiB); exact p95 0.0864 ms, selective 0.3966 ms, common 3.0929 ms vs reference 45.0312 ms; forget 1.4521 ms, restore 17.400 s / 9,128 KiB; decoder p95 38.3/24.1/24.1 µs, RSS 2,172 KiB; zero parity/recovery errors. No live calls. New offline compatibility profile benchmark pending.

PR #27 [CI](https://github.com/idmfrank/atomos/actions/runs/45918304938) / [scale](https://github.com/idmfrank/atomos/actions/runs/45918448554): 145 distinct tests; CLI 466,512 bytes; 100K 4,136 KiB vs 2,684 KiB (increment 1,452 KiB); selective p95 0.2163 ms, common 1.8022 vs 28.7214 reference; forget 1.1674 ms, restore 10.471 s / 9,012 KiB. 1K remember p95 13.5059 ms, median 0.9322 ms; hosted runner tail variation, cause unisolated. 9,000 replays zero errors, strict 38.1/25.5/25.1 µs, equal-wire compat 42.9/25.8/25.3 µs, auxiliary 49.1/37.6/30.1 µs, combined RSS 2,144 KiB. No live calls. Extended candidate tests/bench pending.

PR #28 [CI](https://github.com/idmfrank/atomos/actions/runs/45919051196) / [scale](https://github.com/idmfrank/atomos/actions/runs/45919185353): 150 distinct tests (96 Rust, 11 Python, 43 Node). CLI 466,512 bytes; 100K active 4,144 KiB vs 2,576 KiB no-store, increment 1,568 KiB (1.531 MiB). Exact p95 0.0892 ms, selective 0.4145 ms, common 3.0721 ms vs 44.0512 reference; forget 1.4984 ms; restore 17.411 s / 9,124 KiB. Zero parity/recovery errors. Decoder p95 (1/64/4096-byte reads) strict 23.4/16.5/16.3 µs, same-wire compat 23.0/16.8/16.1, auxiliary 26.9/19.4/18.7, same-wire extended 22.6/16.6/19.8, reasoning 29.1/20.9/20.1, refusal 22.3/15.6/15.2; 18,000 samples zero errors, combined benchmark RSS 2,216 KiB. Prior 1K remember tail returned to 1.4814 ms p95; uncontrolled runner variation. No provider calls in CI; actual token usage, real TTFT, speech, power and integrated memory remain unmeasured.
