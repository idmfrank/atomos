# M2.3 — Synthetic account authorization boundary

> **Milestone context (reviewed through PR #28, 2026-11-26):** This is the historical synthetic account-policy implementation, not a production provider permit. Owner-run official OAuth/catalog/refresh later succeeded, and three separate fixed live qualification attempts were consumed without validated completion/usage. Integrated account-bound Governor admission remains M2.5 work; no general live inference. See [auth hardening](m2-auth-hardening.md), [qualification closure](m2-first-live-qualification.md) and [inference architecture](inference-architecture.md). [Current project state](../PROJECT_STATE.md) is authoritative; status and next-step statements below describe the original milestone unless explicitly updated.


Status: merged PR #14; post-merge CI and full scale passed. Live inference is unconditionally disabled.
No OAuth exchange, real account grant, access token, refresh token,
credential store, HTTP client or private AMG context is implemented.

## Official route and blockers (reviewed 2026-11-26)

Sources: [overview](https://developers.openai.com/siwc/token-sharing-open-source),
[registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
[models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference),
[preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations),
[errors and recovery](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery),
[plan usage and credits](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites).

The documented integration supports eligible locally running personal/open
source projects, subject to OpenAI's terms, account, region and permissions.
No successful registration or actual eligibility verification for AtomOS.
Use the system browser and official registration; stable opaque host ID;
issued client ID retained per account; fresh state, nonce and S256 PKCE;
loopback 127.0.0.1 callback, exact URI within an attempt; validate state and
ID token signature/issuer/audience/nonce/expiry, and granted scopes after
exchange/refresh. Identity sign-in alone is not consent for inference.
Direct plan usage requires chatgpt.tokens.use.direct and the documented
resource/scope set. No copying other apps' credentials or cycling IDs.
Credential implementation must use secure OS storage with restricted access,
no secrets in logs, command arguments, CI, AMG or plaintext backups; support
refresh rotation, disconnect and terminal grant errors. No real OAuth code yet.

Use the official model catalog for the selected account and public
POST /v1/responses with store:false, stream:true, full required input context.
No unofficial endpoints, automatic retries, API-key or paid fallback.
Provider limits govern; shared allowance is not a fixed raw-token quota.
A subscription-limit error suspends inference, not a reason to reauthenticate
or change accounts. A 503 does not establish a reset time. Owner must inspect
Usage settings; no documented push notification of disconnection.

**Spend gate:** Personal-plan app limit below 100% prevents credit usage for
that app per Help Center. Require nonzero limit below 100%, credit use off
and automatic purchases off as defense in depth. Provider docs say direct
errors do not silently change billing route. We have not found an authoritative
programmatic check of the saved credit and app-limit preferences; actual
account/registration eligibility and a reliable spending-control verification
procedure are outstanding. Local fixture booleans are not evidence.

**Generation bound gate:** The preview limitations explicitly exclude
max_output_tokens on this route. Local bytes/output-unit budgets and
cancellation cannot guarantee a provider-side output ceiling. Do not send
unsupported fields or represent a local estimate as a provider-enforced cap.
Before live use, reconcile the design with the documented route, including
output/unknown-usage exposure; verify model event profile (reasoning items
currently unsupported), cancellation, deadlines and true usage estimation.
If required spending and output-bound guarantees cannot be met, live remains
disabled. No fallback to paid API. store:false is not zero retention.

## Implemented offline policy

Unix-only std module authorization::AuthorizedReplay, no new dependencies.
A separate synthetic namespace under an existing private directory contains
an immutable policy header and dedicated existing Broker/Governor ledger.
The policy binds opaque numeric fixture account, host, grant, fixed time
window (at most 31 days), expiry, eligibility/scope/credit flags, app limit
and local Limits. Reopen requires exactly the same configuration; no
automatic reset, rotation, resume or changed budget. Local fixture time is
injected, not provider time or trusted wall clock. Invalid or unknown
evidence denies admission. Live inference enabled is constant false.

For a remote-shaped synthetic request, require active turn, exact account/
host/grant, next sequential request ID, nondecreasing time, grant/window
validity and request expiry within 60 seconds and grant/window. Reject
before reading bytes or touching budget on policy failure. Persist a P
intent with ID, time, expiry, task, turn, byte length, output ceiling and
priority; same immutable borrowed request travels synchronously into the
existing broker, which reserves usage before dispatch/read. No prompt,
body digest or secrets in journal. No detachable permit. D completion
records outcome halt bit. Rate/auth errors, overrun or ledger failure
suspend across restart. H revocation permanent in this fixture grant.
Incomplete P after crash prevents further admission even if governor
was untouched; no replay or automatic recovery. Stale writer lock
requires manual investigation. A consumed ID cannot be retried even on
budget denial; unknown usage remains charged. Exact local help/name
remain available; not authenticated AMG recall.

Policy journal capped at 1 MiB, private directory/file, exclusive writer,
sync before read/dispatch, bounded replay, missing budget file rejected,
no overwriting existing scope. Two journals are deliberately conservative:
durable intent precedes reservation, no cross-file atomic settlement;
crash in between blocks further admissions. Same-UID modification, deletion,
rollback or alternate roots are outside this synthetic trust boundary;
no cryptographic request digest, OS owner authentication, credential
encryption, trusted time, actual provider account or multi-window rollover.
No background jobs or real dispatch. The previous standalone broker
continues to exist only for synthetic reference tests. Not a live
security boundary. Existing governor files reside under private parent
directory; legacy journal mode may be 0644.

Map documented subscription-limit and invalid-user/scope/eligibility
errors to synthetic rate/auth suspension; unsupported capability/route
to unsupported and unavailability to unavailable. No automatic retry
or inferred reset. HTTP status, exact provider error code/request ID
preservation and reauthorization lifecycle remain transport gates.

## Verification

Default workspace tests plus feature-enabled authorization tests in
scripts/test-responses.sh. Independent tests for account/host/grant,
unknown evidence, request expiry, time rollback, monotonic IDs, fixed
limits/window, local resolution, exclusive writer, incomplete intent,
missing journal, truncated records, revocation, rate/auth/overrun
persistent suspension, no-read admission and documented provider error
fixtures. Existing typed and wire reference tests unchanged except for
new error mapping. No model calls, credentials or account data.

Next: complete security/authorization gates before M2.4. Separate
owner-approved supported OAuth experiment and model catalog only after
eligibility confirmed. No live inference until spending and generation
bound issues resolved and transport/credential controls implemented.

Verified [post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45859290355): 86 distinct Rust + 11 Python tests, 11 authorization tests including 2 feature-only, no real provider calls. [Full AMG scale](https://github.com/idmfrank/atomos/actions/runs/45859954016) passed all 3 sizes and recovery. M2.3b develops [auth-only utility](m2-official-sign-in.md), no live inference.
