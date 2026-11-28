# AtomOS engineering rules

## Revised product direction

Follow [the agent roadmap](docs/agent-roadmap.md) and [LAN agent design](docs/local-agent-direction.md). The owner selects an always-running local agent, LAN-only web interaction, broad local agency, graph-based continuous learning and optional external inference. Voice is removed from the product and roadmap. The CLI is a maintenance/recovery adapter. Preserve historical evidence and existing owner data; remove unnecessary voice code without deleting schema migration compatibility. This overrides earlier voice-preservation and CLI-only scope instructions.

## Session startup and handoff

Read [PROJECT_STATE.md](PROJECT_STATE.md) before starting work. It records the verified baseline, current objective, environment boundaries and pending gates. Check the actual checkout and latest CI; never assume a snapshot is current. Apply the per-commit documentation rule below, including documentation-only changes. Verify post-merge results before updating verified evidence. Codex may build and run offline tests; interactive vault operations and live inference require explicit owner action per the documented M2.4 procedure.

## Current product scope

Build on the existing Linux device with shared Rust task, memory, permission and inference interfaces. The installed release remains the local/mock CLI; proposed service, LAN web access, tools, learning, capture and local models are not implemented or validated. External effects require approval or an approved standing policy; routine authorized local actions should not prompt repeatedly. LAN authentication, HTTPS, explicit subnet restrictions, privileged execution mediation and egress enforcement are required. Keep consumed qualification history; live inference, credential access and private disclosure retain separate gates. Follow [the active roadmap](docs/agent-roadmap.md), not historical CLI/voice milestones, for new work.

## Documentation is part of every commit

- **Every commit requires a documentation review and update, not only milestone releases.** Include an accurate change/status entry in PROJECT_STATE.md in the same authored commit as the change, including documentation-only commits. Update README.md and every affected architecture, security, protocol, operation, test, resource-budget and milestone document; review other documents for stale claims. If no other document needs a change, record that review and why in the handoff or PR. Do not modify unrelated historical evidence merely to change a date.
- Before committing, reconcile instructions, actual implementation, known limitations, test commands and expected tests; label unrun CI, benchmarks and live checks as pending. Record any intentional design decisions, blockers and gaps. Do not publish secrets, raw provider content or private account details.
- After every merge or validation run, verify the actual commit and workflow results and reconcile README, PROJECT_STATE and affected technical docs at the next reviewed commit (a documentation-only follow-up when necessary). Record exact tested commit/run links, observed measurements and whether evidence is CI, owner-reported or historical. Do not report a proposed or untested change as verified. A documentation-only follow-up records the last tested executable baseline and its own scope; it need not recursively generate another commit merely to cite its own hash. Never fabricate a self-referential commit ID or merge without owner approval.
- Use [PROJECT_STATE.md](PROJECT_STATE.md) as the canonical current status; README provides the public summary and latest verified tables. Preserve labeled historical milestone results, add links to superseding documents, and keep current operating instructions distinct from historical procedures. Review documentation in the PR alongside code and test evidence. A change is not done until documentation and evidence are reconciled.

## Engineering rules

- Preserve natural conversation quality, zero incremental inference spend, privacy, and deterministic bounded local operation. No unofficial provider access or paid fallback.
- Keep production hot paths in Rust. Measure any cross-language/IPC overhead before a rewrite.
- For behavioral changes, freeze the existing reference and compare equivalent inputs/state. For retrieval compare facts, revision/source IDs, ranking/connector flags, truncation and compiled bytes, not just counts. Add independent expected-result tests; never weaken an assertion merely to make CI pass. Document intentional semantic changes explicitly.
- Prefer fast synthetic local replay and differential checks before PR review; then run locked offline regression CI and scale benchmarks on reviewed commits. Reference and candidate must use equivalent durability, policy and inputs. Do not call a live model twice for verification: replay one recorded synthetic or authorized response stream through parsers. Never duplicate billing, side effects or writes.
- Record failures by stage, sanitized category, counts/lengths and build ID; distinguish media type from content encoding using allowlisted classifications, never raw headers or response bodies; no private memory, prompt, credentials, audio or raw transcript logs. Synthetic fixtures may be inspected in full. Keep diagnostic work bounded and opt-in.
- Before durability or storage performance comparisons, identify the actual backing filesystem. RAM-backed temporary directories are not device-durability evidence. Use an explicit private TMPDIR on the intended persistent filesystem for admission/AMG durability benchmarks, preserve sync semantics, and report RAM-backed runs separately from disk-backed runs.
- A performance improvement must preserve security, correctness and resource caps. Do not disable checks, durability, or disclosure policy to improve timings. Record failed operations, timeout and incomplete results. Do not equate bytes with tokens, CI RSS with whole-system memory, or synthetic recall with general answer quality.
- Production private data and live inference remain gated. Human approval before merging runtime changes.
- When a defect recurs, convert the review lesson into a durable rule and a regression test where possible. Human judgment decides whether a passing change is architecturally sound.

Inspired by Anthropic's [Messages API Rust migration report](https://www.anthropic.com/research/migrating-api-to-rust) (2026-11-25); these are AtomOS-specific rules, not a claim of production parity or automated agent deployment.
