# Resource budget and conversational benchmarks

Status: CLI MVP measurement priorities and historical targets; current merged M2.5b evidence and historical PR #32 baseline are recorded below and in [PROJECT_STATE](../PROJECT_STATE.md). M2.5a isolated admission measurements are in [atomic admission](m2-atomic-admission.md). Physical voice and whole-device power remain pending; direct Linux-host active-AMG and hosted scale results are recorded separately below.

## CLI MVP measurements and current baseline

The [CLI MVP roadmap](cli-mvp-roadmap.md) uses the existing Linux/SSH host and requires no audio gear, new hardware or custom image. Measure local terminal response, context compilation, admission/settlement, startup, active-store RSS/storage and binary size. Later authorized provider tests report first validated/displayed text, completion, cancellation, errors and actual usage separately; do not reuse speech timing targets as CLI claims.

One foreground session, no scheduler/background inference and zero idle calls. Capture p50/p95/p99 with sample/error counts and test-host/build details. Optional `tmux` continuity is an operator arrangement, not runtime recovery. Full transcript logging, new purchases, dependencies and host configuration are not authorized.

Last full-scale measurement baseline (PR #32) `5bca8a15e8245e9631d8a978a988c01cdc812f7a`: [CI](https://github.com/idmfrank/atomos/actions/runs/46025742630), [full scale](https://github.com/idmfrank/atomos/actions/runs/46026109238), same 173 distinct tests and 18,000 decoder samples; CLI 466,512 bytes, decoder RSS 2,180 KiB. Admission reference/candidate p95 0.402973/0.216750 ms, traced fsyncs 2/1, RSS 2,236/2,200 KiB, harness bytes 416,520/440,144 and journal bytes/request 52.560/104.280. Charge 14,100 each, candidate background 7,050, zero errors. 100K CLI 4,232 KiB vs no-store 2,592; selective p95 0.3143 ms, common 2.5666 vs 38.7607 reference, restore 14.1180 s / 9,192 KiB. [Full tables](../PROJECT_STATE.md#latest-verified-test-and-benchmark-baseline).

Hosted synthetic timings are not controlled cross-run comparisons or whole-device/energy/billing evidence. Older records below remain historical. This documentation revision's CI is pending until run.

PR #33's documentation-only CLI roadmap is now merged at `c563599f2e2c8b5abb7e03c75bb85e67e831e75d`; [post-merge CI 46029012445](https://github.com/idmfrank/atomos/actions/runs/46029012445) passed with unchanged runtime. Its CI admission/decoder/text/1K smoke checks do not replace the separately labeled PR #32 full-scale measurements above.

## CLI session candidate measurements, full validation pending

The [session contract](cli-session-contract.md) specifies a 1,024-byte input limit, at most four whole delivered pairs/16 KiB retained UTF-8 payload, 4,096-byte inference output and existing 4,096-byte local context packet with an 8,192-byte final local response cap. These are implemented candidate payload bounds, not a measured allocation/RSS claim. A ten-sample WSL smoke passed on persistent storage; full 300-request SSD measurements and actual syscall tracing remain pending. Measure container overhead, default versus replay-enabled binary size, direct replay versus session-driver latency and terminal delivery separately. Retain all existing benchmark/durability assertions and run full regression/scale for the future runtime PR. No final session performance/resource baseline is claimed yet.

## M2.5b offline candidate evidence

PR #36 is merged at `e3ebf241b64cdf5e0840f944b2c311b6f78a2427`. [Post-merge CI 46080072830](https://github.com/idmfrank/atomos/actions/runs/46080072830) passed all 193 distinct tests (139 Rust / 11 Python / 43 Node), Rust 1.75 core all-targets, 18,000 decoder samples and both admission/outcome benchmarks. Its Git tree exactly matches final PR head `2b98805ce335286cffbd802c42b6cbc4c602cdd2`, which passed [full 1K/10K/100K scale/recovery 46079729987](https://github.com/idmfrank/atomos/actions/runs/46079729987). The full-scale provenance remains that branch head; no new post-merge full-scale run is claimed.

Development head `2d66c2e5ce8b9ca80c85251b6d70682b93e21f09` passed [PR CI 46077487399](https://github.com/idmfrank/atomos/actions/runs/46077487399) and [full 1K/10K/100K scale/recovery 46077563729](https://github.com/idmfrank/atomos/actions/runs/46077563729). CI tested synthetic merge `9e13874702dd190fc622ba926c07835ba1048a6a`; full scale tested the branch head. All 193 distinct tests passed: 139 Rust, 11 Python and 43 Node (173 preserved + 20 new). Repeated features/compiler runs and the ignored subprocess-only crash fixture are not additional tests. The parent crash test actually launches and terminates that fixture. Rust 1.75 candidate core all-target tests and 18,000 decoder samples passed. Host runtime regression/full scale passed at `b6152f031aebabd3ca6d9f5c8f0652a09abb2283`; its source plus benchmark additions passed the SSD-backed frozen-reference comparison at `2d66c2e`. No live calls.

### Equal-wire method and boundaries

Run `TMPDIR=PRIVATE_PERSISTENT_DIR bash scripts/benchmark-outcomes.sh` from a complete checkout with frozen reference history available. It builds the same harness separately against strict atomic commit `c563599f2e2c8b5abb7e03c75bb85e67e831e75d` and the candidate extended API, locked/offline, release profile, same compiler and no native-CPU option. The reference source is extracted to a disposable private directory; its runtime is not edited. Custom cfg selects extended API only in the candidate harness. Both use immutable requests, same initial policy/state, mixed priorities and identical strict bytes. Auxiliary/reasoning/refusal are separate workloads, not same-input overhead comparisons. Compilation/validation are measured separately from the request interval. Traces are separate from latency/RSS runs.

Each of five cases has 300 requests and 30 clean reopens: frozen strict, extended strict, extended auxiliary, extended reasoning, extended refusal. The first four each returned 300 completed answers, the last 300 typed refusals. All had failed/cancelled/unknown/unexpected-error counts 0 and accounting/answer/outcome mismatches 0, supported by per-request and reopen assertions. Each charged 14,100 total / 7,050 background units, 600 appended records and 600 file syncs; 31,284 added journal bytes, 104.280 bytes/request. Failure/cancellation behavior is covered by tests, not these successful-workload timings. Fixture usage is 35 input + 12 output; cached/reasoning subsets are not added.

Measured syscall traces (three requests each for frozen strict, extended strict and extended refusal): admission **2 write syscalls + 1 fsync before first response read**, settlement **2 writes + 1 fsync before result delivery**; total 4 writes / 2 fsyncs per settled request on both APIs. Initialization/directory syncs are outside request measurements. No reduction in durability. Forced death leaves a stale lock and refused reopen; clean reopen timings do not certify automatic crash or power-loss recovery.

### Direct Linux SSD run

Candidate `2d66c2e5ce8b9ca80c85251b6d70682b93e21f09`, Ubuntu 26.04 / Linux 7.0.0-44 / i7-6700 4C8T / 16 GB / SATA SSD **ext4**; Rust 1.95.0, eight build jobs. Explicit private TMPDIR on the SSD; /tmp is tmpfs and was not used. Sequential timing runs with uncontrolled host/storage scheduling. Private logs: `~/.local/state/atomos/m25b-validation/regression-b6152f0.log` and `outcomes-2d66c2e.log`. The first contains the full runtime regression and SSD AMG scale at `b6152f0`; the second isolates the new benchmark at `2d66c2e`.

Latency table values are p50 / p95 / p99 **milliseconds**:

| Workload | Admission at first read | Complete decode/settlement | Clean reopen |
| --- | --- | --- | --- |
| Frozen strict | 1.794245 / 1.836982 / 2.008138 | 3.639739 / 3.737196 / 3.917739 | 0.837815 / 0.864011 / 0.922509 |
| Extended strict | 2.380598 / 2.557247 / 2.747668 | 4.912795 / 5.183656 / 5.309620 | 0.377808 / 1.362314 / 1.609114 |
| Extended auxiliary | 1.822903 / 2.283637 / 2.367359 | 3.772728 / 4.757672 / 4.951957 | 1.345501 / 1.571748 / 1.684347 |
| Extended reasoning | 2.050579 / 2.569950 / 2.610335 | 4.239940 / 5.282208 / 5.358519 | 1.489468 / 1.532862 / 1.688887 |
| Extended refusal | 2.460100 / 2.603019 / 2.634805 | 5.070070 / 5.292650 / 5.330227 | 1.204228 / 1.535035 / 1.634899 |

| Workload | Compilation p50/p95/p99 ms | Validation p50/p95/p99 ms | Process peak RSS KiB |
| --- | --- | --- | --- |
| Frozen strict | 0.019099 / 0.019486 / 0.019704 | 0.000275 / 0.000303 / 0.000482 | 2,512 |
| Extended strict | 0.008327 / 0.009551 / 0.028917 | 0.000552 / 0.000643 / 0.000784 | 2,388 |
| Extended auxiliary | 0.006138 / 0.009607 / 0.009977 | 0.000446 / 0.000621 / 0.000642 | 2,452 |
| Extended reasoning | 0.007263 / 0.010377 / 0.022238 | 0.000515 / 0.000730 / 0.000820 | 2,516 |
| Extended refusal | 0.010031 / 0.010658 / 0.028248 | 0.000647 / 0.000776 / 0.000917 | 2,388 |

Frozen/extended harness: **418,344 / 428,248 bytes**. Default CLI **479,672 bytes**, memory admin **462,280 bytes**, unchanged from the same-host deployed baseline; no-store process **3,060 KiB**. These feature harnesses are not default-CLI overhead. Installed release symlink remains at `5869e205167a3dde3a6a77392af294101e6a46c3`.

The extended strict path is slower in this SSD run, including admission before decoding. Compilation and restart also vary between otherwise identical operations. Retain these observations; no isolated decoder-overhead or performance-improvement claim is supported by this single sequential run.

Preserved old two-journal versus atomic admission comparison also passed: 14,100 total units each, candidate background 7,050, actual admission write/fsync counts 4/2 versus 2/1, zero errors. It is a different comparison from strict-atomic versus extended-atomic above.

SSD full AMG scale/recovery at runtime `b6152f0`:

| Entities | Exact p95 ms | Context p95 ms | Common-term p95 ms | Restore ms | Active CLI KiB |
| --- | --- | --- | --- | --- | --- |
| 1,000 | 0.0662 | 0.3338 | 0.3764 | 166.3505 | 4,536 |
| 10,000 | 0.0696 | 0.3789 | 0.7708 | 1,802.3471 | 4,380 |
| 100,000 | 0.0787 | 0.3959 | 3.1746 | 18,847.2844 | 4,484 |

At every size: 300/300 facts, 100 complete packets, precision/recall 1, 0/30 parity mismatches, zero restore/verification errors and zero remote calls. At 100K common-term reference p95 44.7892 ms; recovery peak 9,348 KiB. This preserves the existing Rust/SQLite decision; it does not measure real-world answer quality.

### Hosted CI and full scale

[PR CI 46077487399](https://github.com/idmfrank/atomos/actions/runs/46077487399) ran Rust 1.100 on the synthetic merge above; Rust 1.75 package-only core check and all-target tests also passed. Whole-workspace Rust 1.75 remains unverified because of the existing unrelated edition-2024 dependency. The outcome benchmark verified ext4 and used the same durability and assertions. No provider traffic or owner state.

Hosted p50 / p95 / p99 **milliseconds**:

| Workload | Admission | Complete decode/settlement | Clean reopen | Peak RSS KiB |
| --- | --- | --- | --- | --- |
| Frozen strict | 0.199625 / 0.262484 / 0.349567 | 0.440137 / 0.558310 / 0.657256 | 0.317888 / 0.410342 / 0.483188 | 2,292 |
| Extended strict | 0.200376 / 0.262744 / 0.342283 | 0.443254 / 0.535687 / 0.660382 | 0.315804 / 0.405192 / 0.495111 | 2,240 |
| Extended auxiliary | 0.199947 / 0.260292 / 0.404331 | 0.449355 / 0.541649 / 0.714453 | 0.320733 / 0.418457 / 0.524766 | 2,236 |
| Extended reasoning | 0.200337 / 0.266030 / 0.330361 | 0.451148 / 0.555194 / 0.661584 | 0.314882 / 0.376037 / 0.480884 | 2,304 |
| Extended refusal | 0.207620 / 0.291658 / 0.364656 | 0.456348 / 0.561395 / 0.673717 | 0.322145 / 0.402997 / 0.477017 | 2,368 |

Hosted harness bytes frozen/extended **413,000 / 422,624**. Hosted decoder process RSS 2,196 KiB; all 18,000 samples passed; strict p95 at 1/64/4096-byte reads 43.8/26.2/25.7 microseconds. Full compilation/validation percentiles and all sample counts remain in linked job logs. No controlled comparison across host/compiler/runners, power, actual token use or provider capability is claimed.

[Full hosted scale 46077563729](https://github.com/idmfrank/atomos/actions/runs/46077563729), branch head `2d66c2e`, same Rust 1.100:

| Entities | Exact p95 ms | Context p95 ms | Common-term p95 ms | Restore ms | Active CLI KiB |
| --- | --- | --- | --- | --- | --- |
| 1,000 | 0.0246 | 0.1212 | 0.1488 | 78.6876 | 4,232 |
| 10,000 | 0.0340 | 0.1676 | 0.3771 | 959.8413 | 4,232 |
| 100,000 | 0.0481 | 0.2095 | 1.7925 | 10,284.3357 | 4,176 |

All sizes again returned 300/300 facts and 100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery errors/calls. At 100K common reference p95 26.2337 ms, recovery peak 9,092 KiB; no-store CLI 2,576 KiB, default CLI 466,512 bytes. Hosted scale is not the owner's hardware or power-loss evidence.

## Verified Linux CLI host baseline

Direct host verification on 2026-11-27 at `5869e205167a3dde3a6a77392af294101e6a46c3` (merged PR #34), distinct from [main CI](https://github.com/idmfrank/atomos/actions/runs/46055361823). Headless Ubuntu 26.04, kernel 7.0.0-44, i7-6700 (4C/8T), 16 GB RAM, SATA SSD/ext4; Rust 1.95.0, Node 22.22.1, SQLite 3.46.1; eight build jobs and sequential timing runs. No runtime code/profile change. [Reproduction, deployment layout and local log provenance](linux-cli-deployment.md).

All 173 distinct tests and 18,000 decoder samples passed. CLI 479,672 bytes; memory administrator 462,280 bytes; installed-copy SHA-256, identity-store integrity and local/mock smoke checks passed. Decoder process RSS 2,216 KiB; strict p95 at 1/64/4096-byte reads 41.6/27.1/26.9 microseconds. Different hardware/toolchain from hosted CI; these are not controlled binary-size or speed regressions/improvements.

The host's default /tmp is tmpfs. Initial RAM-backed admission/full-scale checks passed but are not device-durability evidence. Storage comparisons were rerun with explicit private TMPDIR on the deployment SSD's ext4 filesystem; statfs reports the shared ext2/ext3 family. No sync/durability/policy check was disabled. SSD results below are the local storage baseline, not a power-loss certification.

| SSD-backed AMG metric, p95 unless stated | 1K | 10K | 100K |
| --- | ---: | ---: | ---: |
| Exact recall (ms) | 0.0661 | 0.0701 | 0.0793 |
| Selective context (ms) | 0.3439 | 0.3764 | 0.3974 |
| Common-term context (ms) | 0.3905 | 0.7697 | 3.0336 |
| Reference common term (ms) | 0.5665 | 4.6226 | 44.0687 |
| Remember (ms) | 7.2749 | 6.5213 | 6.2077 |
| Correct (ms) | 7.6329 | 6.8616 | 6.7329 |
| Forget (ms) | 7.2377 | 6.5010 | 6.3349 |
| Active CLI peak RSS (KiB) | 4,404 | 4,380 | 4,500 |
| Restore (seconds) | 0.1661 | 1.7859 | 18.7494 |
| Recovery peak RSS (KiB) | 7,824 | 9,588 | 9,276 |

No-store CLI RSS 3,052 KiB; 100K incremental 1,448 KiB. Every size: 300/300 expected facts, 100 complete packets, precision/recall 1, 0/30 parity mismatches, zero recovery verification errors and remote calls. Common-term 100K p99 7.1607 ms and maximum 7.1607 ms are retained; remember maximum 12.2628 ms, correct maximum 7.8196 ms and forget maximum 6.9315 ms. Synthetic recall is not general answer quality.

| SSD-backed admission metric | Two-journal reference | Atomic candidate |
| --- | ---: | ---: |
| Admission p50 / p95 / p99 (ms), 300 samples | 4.941412 / 5.150968 / 5.212177 | 2.504588 / 2.623407 / 2.710393 |
| Completion p50 / p95 / p99 (ms) | 10.007163 / 10.323192 / 10.428832 | 5.146991 / 5.347218 / 5.439303 |
| Restart p50 / p95 / p99 (ms), 30 samples | 3.880282 / 4.074861 / 4.085702 | 1.485559 / 1.533084 / 1.648072 |
| Traced admission writes / fsyncs | 4 / 2 | 2 / 1 |
| Process peak RSS (KiB) | 2,516 | 2,348 |
| Release harness (bytes) | 422,584 | 447,368 |
| Journal bytes per completed request | 52.560 | 104.280 |
| Final charge / errors | 14,100 / 0 | 14,100 / 0 |

Candidate background charge 7,050 and 600 append records/syncs include settlement. Independent per-request expectations and parity tests passed. Tracing is a separate run, not part of reported timings.

These local retrieval/footprint results fit the existing targets and do not justify replacing Linux, adding another database server/local model, enlarging caches indiscriminately or consuming CPU/RAM while idle. Local context/recall remains zero-inference work. Physical power/energy, real provider token savings, model quality and end-to-end network conversation remain unmeasured. Test retained data are synthetic; no private AMG or vault was copied. Earlier GitHub tables retain their own tested SHA and provenance.

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
