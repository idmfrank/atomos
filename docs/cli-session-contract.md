# Maintenance text session

The CLI maintains shared Rust components; the primary planned interface is LAN web. [Roadmap](agent-roadmap.md) | [Architecture](architecture.md).

## Commands

Default text and `--single-turn` remain. `--session` enables bounded line interaction; optional `--memory DIR OWNER` precedes mode and opens existing owner-scoped memory read-only. Changes use atomos-memory. Voice/AEC/grant commands are removed and fail clearly.

Session commands: `/help`, `/status`, `/clear`, `/quit`. Validate complete arguments before session storage opens. Reject duplicate/unknown flags, missing arguments, conflicting modes, memory plus replay and incomplete replay configuration. No inferred ledger, auto-reset/unlock/repair or fallback.

Input cap: 1,024 raw UTF-8 payload bytes, terminal line-ending handling and unsafe-control rejection. History: at most four delivered completed pairs/16 KiB, whole-pair eviction. Local/refused/failed results do not enter successful history; failed output/flush/stale delivery suppress retention. History is volatile and local-only, never implied provider context. Status distinguishes mock/synthetic accounting.

## Replay

Unix feature `session-replay` enables fixed fixtures: `--session --replay NAME --ledger DIR`, with explicit `--init-ledger` for fresh synthetic state only. Default build rejects replay. Memory and replay cannot combine; no credentials/private facts enter fixtures. No production transport.

Durable admission precedes reading; known usage settles before delivery, unknown stays pending/charged, halt follows policy. Storage faults suppress output. No reset/retry path.

## Termination/checks

EOF or `/quit` exits. Ctrl-C terminates; crash may leave a lock requiring investigation. Closure does not prove remote cancellation/refund. Service/task recovery is separate planned work.

Tests compare default/single-turn/text-memory with temporary Git-history reference and explicitly reject removed modes. Cover UTF-8/raw limits, local commands/memory correction/deletion, bounded history, output/stale failures and ledger accounting. [Validation](resource-budget.md).
