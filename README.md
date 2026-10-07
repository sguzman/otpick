# OTPick

OTPick is a one-shot TOTP picker for Linux.

The normal lifecycle is intentionally tiny:

Super+, -> search -> Enter -> clipboard -> process exits

The popup path is sacred. Network access, imports, backups, QR decoding, vault maintenance, update checks, and other administrative work do not belong on it.

## Current state

The native Rust + egui picker is wired to a real encrypted vault and can ingest normal TOTP accounts plus Google Authenticator migration payloads.

The vault uses Argon2id for passphrase-derived keys and XChaCha20-Poly1305 for authenticated encryption. The expensive KDF runs only when creating or explicitly unlocking the vault. The derived 32-byte vault key is cached in the Linux session keyring, so each disposable picker process can retrieve it without keeping an OTPick daemon resident.

OTPick can also persist that derived key in the desktop Secret Service used by GNOME Keyring and KWallet. The normal picker still checks the fast kernel session keyring first. On the first invocation after login, if the session key is absent and desktop-keyring integration is enabled, OTPick retrieves the key from Secret Service and repopulates the kernel session cache. Later invocations stay on the fast path.

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
    otpick keyring enable
    otpick keyring disable
    otpick status
    otpick list
    otpick backup
    otpick backup /mnt/archive/otpick

`otpick keyring enable` is a one-time opt-in that stores the vault key in the desktop Secret Service. It may trigger the desktop keyring's normal unlock prompt. After that, the first OTPick invocation after a login can restore its kernel-session cache from the already-unlocked desktop keyring instead of asking for the OTPick passphrase.

`otpick lock` creates a manual-lock marker in the Linux session keyring. While that marker exists, OTPick will not automatically rehydrate from the desktop keyring. The marker disappears with the login session, and `otpick unlock` clears it explicitly.

The popup never performs Argon2 when the vault is already session-unlocked.

## Interaction contract

- Typing filters/ranks immediately.
- The top result is selected automatically.
- Enter generates the selected account's current TOTP, copies it, and closes OTPick.
- Up/Down changes selection.
- Esc exits without copying.
- No mouse is required.
- One invocation produces at most one copied code.

If the vault is locked, OTPick first checks whether a desktop-keyring copy is available unless the user explicitly locked OTPick for the current login session. Secret Service access therefore occurs only on the cold first-use path after login; normal summons stay on the Linux session-keyring path.

## Ingestion

The text importer accepts standard otpauth://totp URIs with SHA1, SHA256, or SHA512, 6-8 digits, and configurable period. It also decodes Google Authenticator otpauth-migration payloads, maps supported algorithms/digit counts, rejects HOTP entries, and assembles complete multi-part export batches before writing the vault. Missing standard-URI algorithm/digits/period values default to SHA1/6/30.

An import file may contain multiple URI lines. QR images may contain one or more detected codes, and multiple image files may be supplied together. The entire combined import is validated before the encrypted vault is rewritten. Exact duplicate credentials are collapsed, while distinct credentials that share the same visible issuer/account label are preserved and displayed with deterministic [2], [3], ... suffixes.

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

While unlocked, the vault key is intentionally available to processes possessing the same Linux session keyring. With desktop-keyring integration enabled, an encrypted copy is also stored through the freedesktop Secret Service implementation provided by the user's desktop keyring. This is what permits OTPick to share the desktop's normal secrets-unlock ceremony while keeping the picker itself ephemeral. It is not equivalent to keeping the second factor on a physically separate device.

## Next

The core authenticator path is operational. The remaining integration work is desktop polish: install the release binary into the user's normal executable path, bind the summon key in Hyprland, measure first-frame latency with and without a cold desktop-keyring restore, and refine the picker UI without weakening the one-shot latency contract.

## License

MIT OR Apache-2.0.
