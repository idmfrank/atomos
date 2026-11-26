# M2.3c — Auth hardening and read-only discovery

Status: merged PR #16; post-merge CI and scale passed. No live inference, actual account authentication,
catalog call, provider registration or credential exchange performed in development
or CI. This is the first hardening slice, not completion of every M2.3c gate.

## CI maintenance

Both workflows pin actions/checkout v7.0.1 at
3d3c42e5aac5ba805825da76410c181273ba90b1. Verified signed release commit
and action.yml runs.using node24 from upstream. Original Node20 warning
originated in checkout, not the benchmark. Preserve contents:read and
persist-credentials:false. Verify both workflows after merge.

## Credential vault

Replace the M2.3b plaintext profile.json with versioned authenticated ciphertext
using Node native AES-256-GCM, random 96-bit nonce, 128-bit tag, fresh 128-bit
salt per write, scrypt N=65536/r=8/p=1 with fixed parameters and a 96 MiB
maxmem limit. AAD binds the vault format and local host ID. At most 256 KiB
plaintext and 512 KiB encoded envelope; canonical base64url and exact format
checks. Reject plaintext legacy profiles; no automatic migration, deletion,
re-registration or reset. No real previous profile has been observed; if one
exists, stop and design a reviewed migration preserving the host/issued ID.
Same UID, private modes, atomic replacement, exclusive lock and fsync retained.
Nonsecret host and pending registration metadata separate. Wrong passphrase,
tampering, host mismatch or malformed ciphertext fails closed.

Unlock uses installed /usr/bin/systemd-ask-password, interactive TTY only,
no caching flag, timeout, no secrets on command line or in environment. Require
a strong passphrase of at least 20 bytes (recommend six random words or a
password-manager-generated secret); new sign-in confirmation. No unattended
unlock or recovery if lost; no auto-rotation. Cold-path scrypt uses about
64 MiB, not a resident runtime. JS strings, TLS library buffers and process
memory cannot be guaranteed erased; encrypted filesystem/swap and trusted
local OS still advised. Best-effort buffer clearing only. Same-UID/root
compromise, rollback, vault replacement and crash residue not solved.
No new npm or Rust dependency. Production JOSE/storage review remains needed.

## Protocol and catalog

Native JSON grammar parser plus bounded audit rejects duplicate keys even
when escaped or nested; depth 32 and 16,384 value limit, at most 512 KiB
input. Applied to JWT header/claims, discovery/JWKS/token responses and local
records. This is not a full independent JSON or JOSE implementation.
Returning explicit sign-in with missing grant uses documented prompt=consent
and existing client ID (not unconfirmed force_reconsent rollout).
Missing host with existing registration/pending marker rejected.

Optional owner-run `models` command uses the same verified local active
profile and current unexpired access token, no automatic refresh.
Only GET https://api.openai.com/v1/models with Bearer, no redirects and
15-second deadline / 8 MiB catalog-only response limit. Authentication endpoints retain the 256 KiB wire limit; vault/JWT JSON defaults remain unchanged. Catalog JSON audit retains depth 32 with a separate 262,144-value bound. Validate documented models array;
preserve server ordering for visibility=list, bounded slug/display_name,
reject duplicate visible slugs/control chars. Print names/slugs only. Do
not log token or raw responses. No /v1/responses code path or model fallback.
The catalog may identify model choices, but does not prove the model's event
profile, included allowance, billing settings or a generation cap.
Offline tests cover malformed catalogs, expiry, rollback, wrong scope.
Actual HTTPS interoperability not yet exercised, no live network in CI.

## Spending and inference gates

[OpenAI models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference),
[preview limits](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations),
[sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in),
[accounts](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
[personal plan usage](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites)
reviewed 2026-11-26. App weekly limit below 100% prevents purchased-credit
usage for personal plans. Disable app credit use and auto purchases separately,
inspect saved preferences with owner; no authoritative programmatic
verification identified. Do not invent a setting/usage endpoint. Private
client eligibility not independently established. Preview does not support
max_output_tokens. Decoder cap and cancellation are not generation ceilings.
Local limits do not establish an actual raw-token entitlement. Do not
introduce live dispatch until owner approves a documented resolution of
generation-bound requirement and provider-side spending controls, plus
account-bound admission, response/transport cancellation/deadlines, secure
credential integration, model profile and unknown-usage reconciliation.
No alternate API/key/billing path. Actual registration and read-only discovery
only after security review and owner-confirmed eligibility, on owner Linux
machine. Do not run auth utility on CI or share secrets.

## Tests

```bash
node --test scripts/auth/protocol.test.mjs
node --check scripts/auth/atomos-auth.mjs
```

Existing 11 synthetic tests plus 5 independent additions: duplicate JSON,
encrypted profile/tamper/wrong key/legacy, catalog, signed duplicate claims,
explicit returning consent. No inference calls. Actual TTY password prompt,
provider HTTPS/callback interoperability and integrated runtime unmeasured.
Full scope not a production security certification. Remaining follow-up:
manual review, supported real sign-in and model catalog, owner spending
confirmation, decision on output ceiling and real account-bound transport.

## Owner-run WSL qualification (2026-11-26)

Owner reported all 16 offline tests passed on Ubuntu 24.04.4 LTS / Node 24.22.3. After PR #17 corrected the password prompt option, actual sign-in succeeded and a fresh status invocation reported active registration, granted plan scope, unexpired access, and inference disabled. The subsequent read-only catalog GET passed HTTP status validation but exceeded the original 256 KiB body limit. Catalog-only bounds increased to 8 MiB and 262,144 JSON values; auth and vault limits unchanged. Added two synthetic regression tests; actual catalog response size and successful parsing still unverified. No tokens, callback URLs, account identity or credentials recorded. No live inference. No new registration or vault reset required for this fix.

Owner subsequently reported all 18 local tests passed at ab677ba and successful read-only model discovery with the existing vault. Owner screenshot showed AtomOS enabled with 50% app limit. No account-wide credit setting verified; personal app limit below 100% blocks purchased-credit use per official documentation. Live refresh/revocation and production review still outstanding. M2.4 [one-shot qualification](m2-first-live-qualification.md) explicitly handles absence of provider output ceiling; no general inference permission.
