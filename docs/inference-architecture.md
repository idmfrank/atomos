# Reasoning and inference

Inference is optional; identity, memory, tasks and permissions belong to the agent. [Roadmap](agent-roadmap.md).

## Local first

Use deterministic commands, scoped retrieval and validated procedures. Benchmark local models on the actual hardware for task quality, latency and memory before selecting one. Fitting in RAM does not qualify reasoning. Learning means retained knowledge/evaluated procedures, not automatic training or constant inference.

## External augmentation

Use official supported access under an approved account/model/disclosure/spending policy. Zero incremental spend; no paid/API-key fallback or automatic credits. Recheck [official provider documentation](https://developers.openai.com/siwc/token-sharing-open-source) when implementing access. Authentication/catalog discovery is not proof of usable inference or enforced spending/generation caps.

[Owner auth](owner-auth.md) is an explicit cold-path utility with encrypted credentials. No runtime credential bridge or production inference transport exists; no unattended unlocking. Preserve account identity and existing usage; no re-registration or reset to bypass state.

## Request/accounting contract

Compile one canonical owned request containing all transmitted instructions, input, working history and approved memory/tool context. Bind exact bytes, account/model/policy, task/turn and fresh time to admission. Private disclosure needs policy approval. Never append context to an already authorized different request.

Offline foundations reserve durably before response reading and settle known usage before delivery. Unknown usage retains reservations and blocks according to policy; auth/rate/overrun conditions suspend durably. Storage failure suppresses delivery. Output failure is not a refund. Synthetic planning units are not measured provider entitlement/charges.

Production transport needs bounded lifetime, cancellation, size limits, validated endpoint/TLS/media/encoding, no implicit retries, token-expiry handling and sanitized errors. Do not claim a provider generation cap where unsupported. Live activation needs a reviewed concrete owner-authorized procedure. Model output never authorizes tools.

## Validation

Use synthetic strict/extended fixtures covering completion/refusal/failure, malformed framing, usage, cancellation, identity and persistence faults. Preserve independent text/charge expectations. Test full context and transport failures before credentials. Keep local operation useful offline. [Resources](resource-budget.md), [security](threat-model.md).
