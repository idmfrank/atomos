# M2.4 — First live inference qualification

Status: PR #19 merged at 6dff964; [post-merge tests](https://github.com/idmfrank/atomos/actions/runs/45874158043) and [full scale](https://github.com/idmfrank/atomos/actions/runs/45874467379) passed. Owner subsequently ran first attempt: HTTP 200, media/encoding rejection, unknown usage. No completed inference validated; no actual inference by development or CI. M2.3d owner-reported
WSL2 OAuth and persistent vault succeeded, 18 local auth tests passed, model
catalog returned 10 listed models including gpt-6.2-sol. AtomOS enabled at
50% in owner's connected-app settings screenshot. Owner later successfully refreshed and reopened the session; remote revoke not tested. No private account details or credentials logged.

## Contract and decision

[Official models/inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference),
[preview constraints](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations),
[personal plan limits](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites)
checked 2026-11-26. Use direct OAuth Bearer access token for selected account,
GET catalog then one POST https://api.openai.com/v1/responses, store:false,
stream:true, input array, selected listed model gpt-6.2-sol. Fixed nonpersonal
prompt from documented example. No previous response ID, arbitrary prompt,
tools, AMG, API key, alternate provider or paid fallback. Personal app cap
below 100% blocks purchased credits, per provider documentation. Owner must
recheck current saved app cap and intended personal account before running.
No authoritative local setting read; flag is owner attestation, not proof.

Preview does not support max_output_tokens. Owner acknowledges this limitation
before single POST. 1,024 planning tokens reserved, not a server ceiling
or raw plan allowance; actual reported input/output settled if validated,
overrun recorded, unknown retains reservation. Deadline and socket close
do not guarantee upstream generation/usage stops. No automatic retry
even for network, HTTP, decoder or quota error. Do not reset or repeat.
This is a single qualification exception, not production authorization.

## Implementation

Optional short-lived Node utility holds existing vault exclusive writer lock,
checks valid current token/expiry plus 60 s margin and pending-registration
absence, verifies chosen model in fresh catalog with same token, builds
fixed body, records fsynced intent in private m2-4-probe.json before POST.
Binds hash of host/issued ID/subject and SHA256 exact body; no token or
personal context recorded. One per vault, regardless of success, failure,
refresh, sign-out or restart. Pending marker blocks after crash; no retry
or recovery command. Atomic finish record uses existing same-owner store
and compares pending intent; fsync before result shown. Same UID/root,
deletion/rollback/alternate vault or same-user code tampering not protected.
No global provider quota or cryptographic anti-rollback claim.

HTTPS direct with TLS validation, no agent reuse or redirect/retry, 30 s
absolute request deadline, HTTP 200 + SSE content type; diagnostic revision supports identity or gzip with independent 256 KiB received and decompressed bounds. Unknown encodings and non-SSE remain rejected. Response held in RAM only, then existing Rust text-only
Responses decoder via a separate local executable/stdin; no raw body
logged or persisted, child receives no OAuth tokens or environment secrets.
3 s decoder subprocess deadline. Strict text-only profile may reject
actual provider reasoning/metadata/event shape; no silent loosening or
second live call. Only terminal validated usage and exact expected output
comparison returned; no arbitrary model text printed. Report response
headers latency, first body byte latency (NOT TTFT), stream end latency,
bytes, known usage with subsets, exact expected text boolean. Unknown
usage means stop, inspect sanitized category and account usage; no repeat.
No integrated Token Governor, rolling budget, ongoing voice streaming,
generation ceiling, or real usage/charge proof. No default CLI changes.
Normal status live_inference_enabled:false refers to general runtime;
qualification_attempted shows durable one-shot marker presence.

## Original owner-run WSL steps (historical; do not repeat)

The first attempt is already consumed. These commands document the original procedure; they do not authorize a second attempt. See recovery below.

Use existing local vault and a clean updated checkout, not /mnt/c or CI.
Install Rust if absent from official rustup instructions; match CI stable
where feasible. Provision locked dependencies then offline build.

```bash
cd ~/atomos-catalog-test
git fetch origin main
git switch --detach origin/main
cargo fetch --locked
cargo build --release --locked --offline -p atomos-core --features responses-replay --example responses_qualify
node --test scripts/auth/protocol.test.mjs scripts/auth/live-probe.test.mjs
node scripts/auth/atomos-auth.mjs status "$HOME/.config/atomos-auth"
```

If status says access expired, stop and qualify refresh separately; do not
re-register. If qualification_attempted true, stop. Recheck settings
for personal plan and saved enabled app limit below 100%. Explicitly
accept absence of guaranteed provider generation ceiling for one fixed
request; if not acceptable do not run. Only then run once:

```bash
node scripts/auth/atomos-auth.mjs qualify-once "$HOME/.config/atomos-auth" \
  --confirm-personal-app-below-100-and-uncapped-single-request
```

Use existing passphrase. Share only sanitized JSON result, not vault,
callback URLs, secrets or raw stream. Record before/after app usage
where available; provider usage may lag or be rounded. Expected exact
response boolean true and terminal validated usage; no assumed token
number or latency. If nonqualified, unknown, or interrupted: do not
repeat, delete ledger, or change registration. Review sanitized result
and provider usage first. The next PR can address actual response profile
using offline fixtures; owner must separately approve any further
qualification call. Integrated runtime admission and provider-side
generation limit remain outstanding.

## Transport recovery (owner report 2026-11-26)

Owner verified 26 local tests and successful build at 30df9b6. Original probe ledger: unknown, unvalidated, probe_content_type, reported_tokens null, charged planning units 1024. In the original source this category is reached only after HTTP 200 and combines media-type and encoding checks. Exact header values and provider usage unknown; no claim that inference completed. Subsequent explicit live refresh succeeded; active unexpired session, plan scope granted, original attempt marker preserved. Do not delete the original ledger or rerun qualify-once. Diagnostic revision records only allowlisted HTTP status/media/encoding classes and bounded timings/byte counts; never raw headers/body, IDs or secrets. Accepts bounded gzip SSE (both compressed/decompressed 256 KiB, invalid compressed data denied), still refuses JSON/HTML/unknown types and unsupported encodings. Gzip is a hypothesis, not established cause. Three new synthetic tests cover header classification, compressed replay and durable unknown accounting. No new live command or authorization added; a separate reviewed and explicitly owner-authorized design is needed for any further live qualification.
