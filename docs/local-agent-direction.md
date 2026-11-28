# Local agent and private browser interface

Design revision, 2026-11-27. The owner has requested a local agent with broad authority over its dedicated Linux device, optional external inference, and approval before actions outside the device. The selected interface is a web service accessible directly within the owner's local network only. The agent runs continuously, learns from interactions and verified outcomes, and can ingest scoped host/network observations under bounded retention. Voice is removed from the roadmap. These are requirements, not implemented-capability or deployment claims.

[Current state](../PROJECT_STATE.md) | [Active roadmap](agent-roadmap.md) | [Architecture](architecture.md) | [Security](threat-model.md) | [Resources](resource-budget.md)

## Product and interface

Keep Linux as the operating system. AtomOS is an agent runtime with memory, goals, tools and optional model reasoning. A CLI or browser is an adapter to that runtime, not the agent itself. Preserve the existing CLI for diagnostics and recovery; pause further CLI-only product work while the web contract is defined.

Start with one owner and one active task. Serve a small browser interface from Rust on an explicitly selected LAN address, with firewall rules restricting access to the intended local subnet. Do not default to all interfaces. Validate IPv4 and IPv6 exposure, and require no router port forwarding, public reverse proxy or public tunnel. A private address alone is not an access-control boundary. The exact address and subnet must be verified before deployment.

Require owner authentication and encrypted HTTPS with a trusted local certificate arrangement before enabling privileged actions. An IP allowlist does not replace authentication. Bind browser sessions to the intended origin, use protected cookies and CSRF defenses, reject unexpected Host/Origin headers, and authorize every command and event stream. Render model/tool text as untrusted text. Keep secrets out of URLs and access logs. Certificate provisioning and LAN firewall changes are deployment work, not completed in this revision. Avoid a separate frontend service or database unless measured needs justify one.

The initial browser surface should provide conversation, task progress, a cancel control, local/provider status, external-action approval requests, and a concise action history. Browser reconnects read existing task state; they must not resubmit work. Closing a tab is not cancellation. Define explicit cancel, disconnect, restart and crash behavior before tool execution is enabled. Persist bounded task metadata and sanitized action outcomes; full transcript persistence remains a separate retention decision.

## Intelligence and execution

Use deterministic local operations and memory retrieval first. Benchmark small local models on the actual machine for task quality, latency and resource consumption before selecting one. No model, performance target or general reasoning capability is verified by this revision. Keep the current zero-incremental-spend constraint for optional external inference; no paid fallback or automatic credit spending.

The runtime owns task state and permissions. A model proposes a plan or structured action; a local policy check authorizes it, an executor performs it, and a verifier checks the result. Model output, files, retrieved pages and tool output cannot grant permissions or override owner policy. Bound steps, runtime, output, retries and concurrent work; retain an interactive cancel path and reserve resources for SSH and recovery. Do not add continuous inference merely to keep the agent busy.

## Authority and the device boundary

The intended standing authority covers local files, processes, services and system administration on the dedicated device. Routine actions within an approved task should not require repeated prompts. Administrative capability should be mediated by a narrow executor; the web process should not run as root. Protect connectivity and maintain a recoverable previous deployment/configuration when applying changes. These are automatic execution requirements, not requests for repeated owner approval.

An action is external when its effects or disclosures extend beyond the device, even when started by a local command. This includes outbound requests, inference, package downloads, Git pushes, email, purchases, and changes through mounted remote storage, remote databases or cloud credentials. A local file path or subprocess is not proof of a local-only effect. Network policy must cover subprocesses and privileged execution, not only the model client. DNS, redirects and dependencies also count as network activity. Preserve separately approved existing SSH connectivity. Authenticated LAN browser request/response traffic is permitted by the configured service policy and does not require per-packet approval.

External work requires owner approval of the concrete destination, effect, data and cost boundary. A bounded standing policy can authorize repeated inference or approved downloads without a prompt per request. No such new policy is activated by this design revision. The model cannot approve its own requests. Credential access and private-memory disclosure retain their separate gates; broad local autonomy must not become credential export authority.

Enforcement needs an explicit design and adversarial tests: an unrestricted root shell can bypass an application-only egress check. Do not claim a hard external boundary while arbitrary commands or administrative tools can bypass it. Separate planning, execution privileges and network authority, document residual trust in the owner/host administrator, and fail closed where an action's boundary cannot be established. Exact isolation and privilege mechanisms remain to be selected and validated, not implemented here.

The [MVP contract](mvp-contract.md) supplies the concrete first-release scope and acceptance gates. Its default retention/work limits refine these architectural requirements without enabling any capability.

## Delivery sequence

The authoritative stages and continuous-learning/storage acceptance criteria are in [the agent roadmap](agent-roadmap.md). The following interface/execution details support those stages.

1. Define the web/task API, local authority and external approval contract, persistence/retention limits, and isolation design. Review against the deployed local/mock baseline.
2. Build a LAN-restricted web adapter over shared Rust interfaces. Exercise HTTPS and authentication, subnet restrictions on IPv4/IPv6, Host/Origin validation, CSRF, reconnect, duplicate-submit prevention, cancellation, rendering and resource limits with synthetic tasks. No live inference or command execution required for this slice.
3. Implement the local task executor and enforceable external boundary, with bounded actions, administrative mediation, audit outcomes and recovery. Test attempted bypasses and interrupted tasks in disposable environments before granting device administration.
4. Benchmark local inference separately from existing retrieval and deterministic operations. Select a model only if measured usefulness justifies its footprint. Failure to find a useful local model must be reported, not disguised by mock responses.
5. Complete the optional provider adapter behind the same task, context, accounting and approval interfaces. A production provider adapter is not implemented. Review credentials, spending and permitted disclosure before any owner-authorized live qualification.
6. Deploy a reviewed release and validate the owner journey: reconnect to a task, execute and verify a local task, cancel safely, preserve SSH, reject unauthorized egress, recover after restart, and use external reasoning only within approved policy. Record quality and resource evidence separately.

## Implementation and evidence boundary

The installed release remains the verified local/mock CLI. No web server, autonomous tools, local model, observation pipeline, new privileges, network policy or provider connection is installed by this revision. Voice-only source components are removed by the accompanying cleanup; legacy database compatibility remains. Current tests cover the implemented local/mock foundations; they do not qualify the planned agent service. Proposed acceptance checks above have not run. Preserve existing account identity, records and outstanding usage; no reset or re-registration bypass.
