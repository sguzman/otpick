# OTPick

OTPick is a one-shot TOTP picker for Linux.

The normal lifecycle is intentionally tiny:

Super+, -> search -> Enter -> clipboard -> process exits

The popup path is sacred. Network access, imports, backups, QR decoding, vault maintenance, update checks, and other administrative work do not belong on it.

## Current bootstrap state

The repository now contains the native Rust + egui picker shell, ranked account search, RFC 6238 TOTP generation, and a Wayland clipboard handoff designed to survive OTPick exiting immediately.

Real vault persistence, session unlocking, Google Authenticator migration import, and management UI are the next layer. The current executable intentionally starts with no accounts rather than introducing a plaintext development vault.

## Build

Requirements:

- Rust stable with edition 2024 support
- Linux / Wayland or XWayland
- wl-clipboard for the current clipboard handoff

Build:

    cargo build --release

Run:

    ./target/release/otpick

The release profile uses thin LTO and one codegen unit. Startup and first-frame latency will be measured before further tuning.

## Interaction contract

- Typing filters/ranks immediately.
- The top result is selected automatically.
- Enter generates the selected account's current TOTP, copies it, and closes OTPick.
- Up/Down changes selection.
- Esc exits without copying.
- No mouse is required.
- One invocation produces at most one copied code.

## Clipboard lifecycle

A normal Wayland clipboard is served by its owner process. OTPick itself must die immediately after a successful selection, so the clipboard boundary is deliberately separate from the egui process lifecycle.

For the first Linux implementation OTPick writes the code to wl-copy over stdin. wl-copy takes ownership of the Wayland clipboard and backgrounds itself, allowing OTPick to terminate without losing the copied code. The TOTP is not passed as a process argument.

This boundary can later move to an internal wl-clipboard-rs helper without changing the picker core.

## Security direction

The intended vault design is encrypted at rest, unlocked once per login/session, and cheap to access from each ephemeral picker invocation. An expensive passphrase KDF must never run on the already-unlocked hot path.

OTPick deliberately does not claim that putting password and TOTP material on the same machine provides the same isolation as a separate authenticator device.

## License

MIT OR Apache-2.0.
