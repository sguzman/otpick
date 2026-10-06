# OTPick

OTPick is a one-shot TOTP picker for Linux.

The normal lifecycle is intentionally tiny:

Super+, -> search -> Enter -> clipboard -> process exits

The popup path is sacred. Network access, imports, backups, QR decoding, vault maintenance, update checks, and other administrative work do not belong on it.

## Current state

The native Rust + egui picker is wired to a real encrypted vault and can now ingest normal TOTP accounts.

The vault uses Argon2id for passphrase-derived keys and XChaCha20-Poly1305 for authenticated encryption. The expensive KDF runs only when creating or explicitly unlocking the vault. The derived 32-byte vault key is cached in the Linux session keyring, so each disposable picker process can retrieve it without keeping an OTPick daemon resident.

Vault writes are encrypted before touching disk, written through a mode-0600 temporary file, fsynced, and atomically renamed. The on-disk format is versioned and authenticates its header as AEAD associated data.

## First setup

Build:

    cargo build --release

Create the vault:

    ./target/release/otpick init

Add a normal account interactively:

    otpick add

Or import one or more standard otpauth TOTP URIs from a text file:

    otpick import accounts.txt

Stdin is supported too:

    otpick import -

Passing the full otpauth URI as a command-line argument is deliberately not supported because it would put the TOTP secret into shell history and process arguments.

Useful commands:

    otpick unlock
    otpick lock
    otpick status
    otpick list

The popup never performs Argon2 when the vault is already session-unlocked.

## Interaction contract

- Typing filters/ranks immediately.
- The top result is selected automatically.
- Enter generates the selected account's current TOTP, copies it, and closes OTPick.
- Up/Down changes selection.
- Esc exits without copying.
- No mouse is required.
- One invocation produces at most one copied code.

If the vault is locked, the picker tells the user to run otpick unlock. Unlocking is deliberately outside the normal hot path.

## Ingestion

The current text importer accepts standard otpauth://totp URIs with SHA1, SHA256, or SHA512, 6-8 digits, and configurable period. Missing algorithm/digits/period values default to SHA1/6/30.

An import file may contain multiple URI lines. The entire import is validated before the encrypted vault is rewritten, and duplicate issuer/account identities are rejected.

Manual add intentionally defaults to the overwhelmingly common SHA1, six-digit, 30-second profile. More advanced manual editing belongs in management UI rather than the normal picker.

## Clipboard lifecycle

A normal Wayland clipboard is served by its owner process. OTPick itself must die immediately after a successful selection, so the clipboard boundary is deliberately separate from the egui process lifecycle.

The current implementation writes the code to wl-copy over stdin. wl-copy takes ownership of the Wayland clipboard and backgrounds itself, allowing OTPick to terminate without losing the copied code. The TOTP is not passed as a process argument.

## Vault location

By default:

    $XDG_DATA_HOME/otpick/vault.otpvault

or, when XDG_DATA_HOME is unset:

    ~/.local/share/otpick/vault.otpvault

For development/testing, OTPICK_VAULT can override the full vault path. Existing override-parent permissions are not modified.

## Security model

OTPick protects the vault at rest and avoids plaintext secret files.

While unlocked, the vault key is intentionally available to processes possessing the same Linux session keyring. This is what permits the picker process itself to remain ephemeral. It is not equivalent to keeping the second factor on a physically separate device.

## Next

The next ingestion layer is image/QR support and Google Authenticator migration payloads, including multi-QR exports. After those are reliable, startup/first-frame latency becomes the primary optimization target.

## License

MIT OR Apache-2.0.
