# OTPick

OTPick is a one-shot TOTP picker for Linux.

The normal lifecycle is intentionally tiny:

Super+, -> search -> Enter -> clipboard -> process exits

The popup path is sacred. Network access, imports, backups, QR decoding, vault maintenance, update checks, and other administrative work do not belong on it.

## Current state

The native Rust + egui picker shell is wired to a real encrypted vault.

The vault uses Argon2id for passphrase-derived keys and XChaCha20-Poly1305 for authenticated encryption. The expensive KDF runs only when creating or explicitly unlocking the vault. The derived 32-byte vault key is cached in the Linux session keyring, so each disposable picker process can retrieve it without keeping an OTPick daemon resident.

Vault writes are encrypted before touching disk, written through a mode-0600 temporary file, fsynced, and atomically renamed. The OTPick data directory is mode 0700.

The on-disk format is versioned and authenticates its header as AEAD associated data.

## First setup

Build:

    cargo build --release

Create the vault:

    ./target/release/otpick init

That initializes and unlocks the vault for the current login session.

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

## Clipboard lifecycle

A normal Wayland clipboard is served by its owner process. OTPick itself must die immediately after a successful selection, so the clipboard boundary is deliberately separate from the egui process lifecycle.

The current implementation writes the code to wl-copy over stdin. wl-copy takes ownership of the Wayland clipboard and backgrounds itself, allowing OTPick to terminate without losing the copied code. The TOTP is not passed as a process argument.

## Vault location

By default:

    $XDG_DATA_HOME/otpick/vault.otpvault

or, when XDG_DATA_HOME is unset:

    ~/.local/share/otpick/vault.otpvault

For development/testing, OTPICK_VAULT can override the full vault path.

## Security model

OTPick protects the vault at rest and avoids plaintext secret files.

While unlocked, the vault key is intentionally available to processes possessing the same Linux session keyring. This is what permits the picker process itself to remain ephemeral. It is not equivalent to keeping the second factor on a physically separate device.

## Next

The next functional layer is account ingestion:

- standard otpauth:// URIs
- manual entry
- QR images
- Google Authenticator migration payloads, including multi-QR exports

After ingestion works, startup/first-frame latency becomes the primary optimization target.

## License

MIT OR Apache-2.0.
