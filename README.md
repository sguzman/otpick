# OTPick

OTPick is a low-latency, one-shot TOTP authenticator and picker for Linux.

**Status:** maintained v1 product. The accepted MVP graduated on 2026-10-07.

The normal interaction is intentionally tiny:

`summon -> search -> Enter -> code copied -> OTPick exits`

OTPick keeps the authenticator vault local, encrypted at rest, and outside the repository. The normal picker path performs no network access and does not run Argon2 when an unlocked key is already available.

## Daily use

Run OTPick with no arguments:

    otpick

Typing filters and ranks accounts immediately. The top result is selected automatically. Up/Down changes selection, Enter copies the selected account's current TOTP and exits, and Escape exits without copying.

One invocation produces at most one copied code.

If the binary is still being run from a development checkout:

    ./target/release/otpick

## Build

OTPick is a Rust application:

    cargo build --release

Maintained releases are also published as versioned GitHub Release archives for x86_64 GNU/Linux.

## First setup

Create the encrypted vault once:

    otpick init

The default vault location is:

    $XDG_DATA_HOME/otpick/vault.otpvault

or, when XDG_DATA_HOME is unset:

    ~/.local/share/otpick/vault.otpvault

`OTPICK_VAULT` can override the full vault path.

### Import

OTPick accepts:

- standard `otpauth://totp` URIs;
- Google Authenticator migration payloads, including migration versions 1 and 2;
- complete multi-QR Google Authenticator export batches;
- PNG, JPEG, and WebP QR images;
- text files and stdin;
- interactive manual entry for ordinary SHA1 / 6-digit / 30-second TOTP accounts.

Examples:

    otpick import accounts.txt
    otpick import google-auth-1.png google-auth-2.png
    otpick import -

Passing a full `otpauth://` URI as a command-line argument is deliberately unsupported so TOTP secrets do not land in shell history or process arguments.

Imports are validated before the encrypted vault is rewritten. Exact duplicate credentials are collapsed. Distinct credentials that share the same visible issuer/account label are preserved and displayed with `[2]`, `[3]`, and so on.

## Unlocking

The vault passphrase is processed with Argon2id only when a real passphrase unlock is required.

During normal use, OTPick caches the derived 32-byte vault key in the Linux session keyring. This keeps repeated picker launches fast without a resident OTPick daemon.

Optional desktop-keyring integration can persist the derived key through the freedesktop Secret Service implementation used by GNOME Keyring or KWallet:

    otpick keyring enable

After that, the first OTPick use in a login session can restore the fast Linux session-keyring cache from the desktop keyring when that service is available and unlocked.

Useful controls:

    otpick unlock
    otpick lock
    otpick keyring enable
    otpick keyring disable
    otpick status

`otpick lock` explicitly locks OTPick for the current login session and prevents automatic desktop-keyring rehydration until `otpick unlock` or the next login session.

The OTPick-specific passphrase remains the fallback if the desktop keyring is unavailable or its stored OTPick item is removed.

## Other commands

    otpick list
    otpick add
    otpick backup
    otpick backup /path/to/archive
    otpick import SRC...

`otpick list` exposes labels only, never current codes or secrets.

`otpick backup` creates a byte-for-byte encrypted snapshot of the vault. The default backup directory is `backups/` beside the vault; `OTPICK_BACKUP_DIR` can override it.

## Clipboard behavior

OTPick writes the generated code to `wl-copy` over stdin with sensitive clipboard handling. The code is not passed as a process argument.

`wl-copy` owns the Wayland clipboard after OTPick exits, so the visible picker can terminate immediately without losing the copied code.

## Security model

The vault uses:

- Argon2id for passphrase-based key derivation;
- XChaCha20-Poly1305 for authenticated encryption;
- mode-0600 vault files and mode-0700 created data directories;
- atomic encrypted writes with fsync + rename;
- zeroization for sensitive transient buffers where implemented.

Vault format and plaintext schema are versioned.

Desktop-keyring integration stores the already-derived vault key in Secret Service so OTPick can share the desktop's normal secrets-unlock ceremony. The Linux session keyring remains the fast hot-path cache.

OTPick intentionally trades physical factor separation for desktop convenience. An authenticator running on the same machine as the service being authenticated is not equivalent to keeping the second factor on a separate phone or hardware token.

## Startup tracing

Set `OTPICK_TRACE_STARTUP=1` to emit opt-in startup timing marks for process entry, key retrieval, vault decryption, egui creation, and first frame.

Normal launches do not perform this tracing work.

## Project governance

OTPick is post-MVP and follows maintained Semantic Versioning and Conventional Commit history.

- Product/release contract: `docs/versioning.md`
- Canonical work queue: `docs/queue.md`
- Release history: `CHANGELOG.md`
- Agent working rules: `AGENTS.md`

## License

MIT OR Apache-2.0.
