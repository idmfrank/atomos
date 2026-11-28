# Owner authentication

The owner-run Node utility manages official authorization and model discovery only. It does not send inference or expose obsolete qualification commands. Credentials are separate from AMG and task state. [Inference](inference-architecture.md) | [Security](threat-model.md).

## Existing utility

```text
node scripts/auth/atomos-auth.mjs status ABSOLUTE_PRIVATE_DIR
node scripts/auth/atomos-auth.mjs sign-in ABSOLUTE_PRIVATE_DIR --confirm-eligible-auth-only
node scripts/auth/atomos-auth.mjs refresh ABSOLUTE_PRIVATE_DIR --confirm-eligible-auth-only
node scripts/auth/atomos-auth.mjs models ABSOLUTE_PRIVATE_DIR --confirm-eligible-auth-only
node scripts/auth/atomos-auth.mjs sign-out ABSOLUTE_PRIVATE_DIR --confirm-eligible-auth-only
```

These are owner-operated commands, not authorization for an agent to run them. Use only the reviewed existing store; a new host enrollment needs its own review. The absolute canonical Linux directory must already exist, be owned by the user and private (0700). Do not put tokens or passphrases in arguments, environment, screenshots or logs. The utility prompts through systemd-ask-password on an interactive terminal; passphrase length is at least 20 bytes. New vault creation requires confirmation. The browser authorization callback is loopback-only and separate from the planned LAN interface.

One issued registration and stable host identity bind an encrypted profile. A missing host can be initialized only in an otherwise empty store; unknown files are preserved and block identity replacement. Read/write ownership, mode, symlink, size and writer-lock checks remain enforced. Do not delete locks or state to retry an ambiguous operation.

Refresh persists unusable pending state before requesting rotation; an uncertain outcome blocks use of old credentials. Sign-out reports whether remote revocation was confirmed; local success alone does not prove remote disconnection. Model catalog visibility is not proof of inference/spending eligibility. Never register another account/host to bypass outstanding usage. There is no credential handoff to the runtime or unattended unlock.

Offline tests cover signed identity, callback/PKCE, refresh/revocation forms, encrypted storage, model catalog, removed-command rejection before vault interaction and preservation of unrecognized state. Tests use synthetic credentials only.
