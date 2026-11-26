# M2.3b — Official sign-in, auth-only utility

Status: merged PR #15; offline CI and scale passed; actual account registration pending. No inference
or model discovery requests. No OpenAI account accessed by CI or development.

Official references checked 2026-11-26:
[registration](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
[overview](https://developers.openai.com/siwc/token-sharing-open-source),
[OIDC discovery](https://auth.openai.com/.well-known/openid-configuration),
[JWKS](https://auth.openai.com/.well-known/jwks.json),
[preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations).
Discovery currently advertises RS256, S256 and public clients. The accounts
guide mentions private and open-source clients sharing Plus usage. That
does not establish this project's actual eligibility or account consent.

## Design

An optional, short-lived Linux Node.js 22+ auth-only utility, not a new
resident runtime or a Rust hot-path dependency. Node standard crypto
(OpenSSL RSA verification, CSPRNG and SHA-256), URL parsing, HTTP and FS;
no npm packages, no client secret. Restricted RS256 JWT validation is
implemented using native primitives rather than a full JOSE library;
production security review / maintained JOSE library evaluation remain
required. No inference API or bearer-token handoff to the broker. No
private AMG data. Existing Rust authorization simulation is unchanged.

Explicit owner command first loads discovery over verified HTTPS (only
known OpenAI endpoints, no redirects, 15 s deadline, 256 KiB response
bound). Requires stable UUIDv4 host ID, random state, nonce, S256 verifier,
loopback 127.0.0.1 random port and fixed /auth/callback path. Owner opens
a one-time local Continue with ChatGPT URL that redirects to the official
browser flow; no credentials or authorization URL printed. Listener binds
IPv4 loopback, checks Host, path, state, duplicate fields and five-minute
attempt deadline; code exchange uses issued client ID and exact redirect
URI/resource. No automatic repeat registration or retry. Initial issued
client ID is recorded as pending before exchange; failed/ambiguous exchange
blocks further registration pending manual review. Denied callback is not
exchanged. Only token-response scopes count, never callback scope.

JWKS is loaded fresh from the discovery-validated URL. Require one matching
RSA signing key, RS256, 2048–4096 bits; verify actual signature, issuer,
exact single audience, subject, expiration/iat/nbf, nonce, optional at_hash.
Returning identity must match saved subject and issued client ID. Unknown
key/algorithm or metadata fails closed. Email optional and trusted only
when provider marks verified. Require all six requested scopes and valid
Bearer/access/refresh/expiry before active local record; otherwise retain
registration mapping without usable tokens and attempt refresh revocation.
An active profile indicates granted scopes, NOT subscription eligibility,
available allowance, spending verification or inference permission.

Single registration per private canonical Linux directory. Files mode 0600,
directory 0700, same UID, no symlinks/hardlinks for reads, bounded record
size, exclusive writer, fsynced atomic rename. Stable host persists across
sign-out. Refresh explicitly invoked: persist pending credential-free
record before exchange to avoid reusing an ambiguously rotated token;
accept only same registration, no scope expansion, replace token set
together. Failed refresh leaves pending. Sign-out attempts documented
revocation with refresh token, clears local tokens and accurately reports
unconfirmed remote revocation. Pending state cannot authorize inference.
Stale lock or interrupted registration requires manual investigation, not
deletion/recreation or host-ID rotation. No automatic refresh or retry.

Limitations: owner-only files are plaintext (not OS keyring or encryption).
Use an encrypted local filesystem, exclude from backups/sync and source
control, trusted owner/parent directories and OS. Same-UID tampering, root,
rollback, crash leftovers and secure erase are outside the guarantee.
Native JSON parsing does not reject duplicate JSON keys; full JWT/JSON
security review, transport integration tests, actual issuer interoperability,
multi-account UX, real signed-in session and spend controls are pending.
Do not use a production account before review and owner eligibility
confirmation. No live model call even when signed in.

## Offline verification

```bash
node --test scripts/auth/protocol.test.mjs
node --check scripts/auth/atomos-auth.mjs
```

Tests use generated synthetic RSA key pairs and fixture claims; no external
requests. CI default CLI/AMG/Responses benchmarks remain unchanged.
Optional owner-run authentication experiment, only after PR review, local
security review and confirmed eligibility:

```bash
mkdir -m 700 "$HOME/.config/atomos-auth"
node scripts/auth/atomos-auth.mjs status "$HOME/.config/atomos-auth"
node scripts/auth/atomos-auth.mjs sign-in "$HOME/.config/atomos-auth" --confirm-eligible-auth-only
```

Open the displayed loopback URL on the same Linux host, select the desired
account and inspect consent. Do not paste codes, URLs, tokens or credential
files into ChatGPT or GitHub. Status prints booleans and lifecycle only.
The same explicit flag is required for refresh/sign-out. Auth utility never
calls /v1/models or /v1/responses. Do not run this in Codespaces/CI; intended
for owner-controlled local Linux desktop. For headless hosts, design a
supported separate host enrollment flow rather than forwarding tokens.

## M2.3c remaining gates

Verify actual eligibility and account settings, disable credits and automatic
purchase, app limit below 100%; independent verification and changes to
settings not established. Preview route excludes max_output_tokens; local
caps cannot guarantee provider generation bound. Require explicit design
decision on that gate, qualified model event profile, transport deadlines,
cancellation and real account-bound governor admission before inference.
No paid fallback or unauthorized alternate route.

[Post-merge CI](https://github.com/idmfrank/atomos/actions/runs/45862345791) passed 11 Node auth tests (108 distinct total); [scale](https://github.com/idmfrank/atomos/actions/runs/45862674857) passed. The original plaintext-file and native JSON limitations above describe M2.3b. M2.3c changes storage to an encrypted vault with strict JSON validation and adds read-only model discovery. Follow the updated [M2.3c specification](m2-auth-hardening.md) for deployment; no real account use before review.
