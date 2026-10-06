# OTPick

OTPick is a one-shot TOTP picker for Linux.

The normal lifecycle is intentionally tiny:

Super+, -> search -> Enter -> clipboard -> process exits

The popup path is sacred. Network access, imports, backups, QR decoding, vault maintenance, update checks, and other administrative work do not belong on it.

## Current state

The native Rust + egui picker is wired to a real encrypted vault and can ingest normal TOTP accounts plus Google Authenticator migration payloads.

The vault uses Argon2id for passphrase-derived keys and XChaCha20-Poly1305 for authenticated encryption. The expensive KDF runs only when creating or explicitly unlocking the vault. The derived 32-byte vault key is cached in the Linux session keyring, so each disposable picker process can retrieve it without keeping an OTPick daemon resident.

Vault writes are encrypted before touching disk, written through a mode-0600 temporary file, fsynced, and atomically renamed. The on-disk format is versioned and authenticates its header as AEAD associated data.

## First setup

Build:

    cargo build --release

Create the vault:

    ./target/release/otpick init

Add a normal account interactively:

    otpick add

Or import standard otpauth URIs, Google Authenticator migration payloads, or QR images:

    otpick import accounts.txt
    otpick import google-auth.png
    otpick import google-auth-1.png google-auth-2.png

PNG, JPEG, and WebP QR images are supported. Multiple image arguments are decoded into one import transaction, which is important for Google Authenticator exports split across several QR codes.

Stdin is supported too:

    otpick import -

Passing the full otpauth URI as a command-line argument is deliberately not supported because it would put the TOTP secret into shell history and process arguments.

Useful commands:

    otpick unlock
    otpick lock
    otpick status
    otpick list
    otpick backup
    otpick backup /mnt/archive/otpick

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

The text importer accepts standard otpauth://totp URIs with SHA1, SHA256, or SHA512, 6-8 digits, and configurable period. It also decodes Google Authenticator otpauth-migration payloads, maps supported algorithms/digit counts, rejects HOTP entries, and assembles complete multi-part export batches before writing the vault. Missing standard-URI algorithm/digits/period values default to SHA1/6/30.

An import file may contain multiple URI lines. QR images may contain one or more detected codes, and multiple image files may be supplied together. The entire combined import is validated before the encrypted vault is rewritten, and duplicate issuer/account identities are rejected.

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

## Startup tracing

Set `OTPICK_TRACE_STARTUP=1` when launching the picker to emit microsecond timestamps for process entry, session-key retrieval, vault decryption, egui app creation, and first frame.

The trace is opt-in and does not run logging work during normal launches. It exists specifically to protect the picker latency contract with measurements from the actual Linux/Hyprland machine.

## Backups

`otpick backup` creates a byte-for-byte snapshot of the encrypted vault. It never decrypts secrets into a backup format and does not maintain a second cryptographic format.

The default backup directory is a `backups` directory beside the vault. Set `OTPICK_BACKUP_DIR` or pass an explicit directory to place snapshots on a separate archival disk.

Backup files are mode 0600 and the destination is fsynced before the command returns.

## Security model

OTPick protects the vault at rest and avoids plaintext secret files.

While unlocked, the vault key is intentionally available to processes possessing the same Linux session keyring. This is what permits the picker process itself to remain ephemeral. It is not equivalent to keeping the second factor on a physically separate device.

## Next

With ingestion, encrypted archival snapshots, and startup tracing in place, the next major target is measuring the release binary on the real Hyprland machine. If management/import dependencies materially hurt picker cold start, those commands will be split into a companion binary rather than weakening the latency contract.

## License

MIT OR Apache-2.0.
