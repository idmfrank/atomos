# AtomOS engineering rules

## Session startup and handoff

Read [PROJECT_STATE.md](PROJECT_STATE.md) before starting work. It records the verified baseline, current objective, environment boundaries and pending gates. Check the actual checkout and latest CI; never assume a snapshot is current. Update the handoff and relevant docs after a verified milestone. Codex may build and run offline tests; interactive vault operations and live inference require explicit owner action per the documented M2.4 procedure.

## Engineering rules

- Preserve natural conversation quality, zero incremental inference spend, privacy, and deterministic bounded local operation. No unofficial provider access or paid fallback.
- Keep production hot paths in Rust. Measure any cross-language/IPC overhead before a rewrite. Native speech adapters remain provisional; do not sacrifice speech quality.
- For behavioral changes, freeze the existing reference and compare equivalent inputs/state. For retrieval compare facts, revision/source IDs, ranking/connector flags, truncation and compiled bytes, not just counts. Add independent expected-result tests; never weaken an assertion merely to make CI pass. Document intentional semantic changes explicitly.
- Prefer fast synthetic local replay and differential checks before PR review; then run locked offline regression CI and scale benchmarks on reviewed commits. Reference and candidate must use equivalent durability, policy and inputs. Do not call a live model twice for verification: replay one recorded synthetic or authorized response stream through parsers. Never duplicate billing, side effects or writes.
- Record failures by stage, sanitized category, counts/lengths and build ID; no private memory, prompt, credentials, audio or raw transcript logs. Synthetic fixtures may be inspected in full. Keep diagnostic work bounded and opt-in.
- A performance improvement must preserve security, correctness and resource caps. Do not disable checks, durability, or disclosure policy to improve timings. Record failed operations, timeout and incomplete results. Do not equate bytes with tokens, CI RSS with whole-system memory, or synthetic recall with general answer quality.
- Keep README and relevant docs aligned with measured results and actual implementation; state outstanding gates. Production private data and live inference remain gated. Human approval before merging runtime changes.
- When a defect recurs, convert the review lesson into a durable rule and a regression test where possible. Human judgment decides whether a passing change is architecturally sound.

Inspired by Anthropic's [Messages API Rust migration report](https://www.anthropic.com/research/migrating-api-to-rust) (2026-11-25); these are AtomOS-specific rules, not a claim of production parity or automated agent deployment.
