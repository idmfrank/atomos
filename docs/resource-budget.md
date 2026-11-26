# Resource budget and conversational benchmarks

Status: proposed, M0. All figures are engineering targets, not measured capabilities.

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

M1.3 [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45752758740): release executable 314,224 bytes; peak RSS 2,136 KiB for 1,000 local simulated text interactions. Excludes OS, real speech models, Python adapter, active ledger and memory DB; no inference calls. Not an acoustic, power or whole-system benchmark.

AMG provisional targets: <8 MiB incremental memory engine overhead within the overall RAM budget, 256 MiB configurable aggregate data quota including derived indexes and journal sidecars, warm p95 exact retrieval <5 ms, bounded traversal <20 ms, context compilation including retrieval <50 ms. Depth 2 default/3 maximum. Zero remote tokens for local retrieval. Compare equivalent SQLite, embedded graph and KV workloads before final engine selection. See [AMG benchmark protocol](amg-architecture.md#performance-and-comparison). All AMG targets unmeasured.
