# Changelog

All maintained releases from v1.0.0 onward are recorded here.

## [1.0.0] - 2026-10-07

### MVP

- Graduated OTPick from ravenous development into the maintained v1 release line.
- Delivered the one-shot `summon -> search -> Enter -> clipboard -> exit` Linux authenticator workflow.
- Added the encrypted local vault using Argon2id and XChaCha20-Poly1305 with versioned formats, atomic writes, and zeroized sensitive buffers.
- Added fast Linux session-keyring unlock caching with no resident OTPick daemon and no Argon2 work on already-unlocked picker launches.
- Added optional freedesktop Secret Service persistence so the first use after login can repopulate the fast session cache from the desktop keyring.
- Added standard TOTP URI ingestion, Google Authenticator migration v1/v2 decoding, complete multi-QR batch assembly, QR image import, manual entry, and encrypted backups.
- Added exact-credential deduplication while preserving same-label distinct credentials with presentation-only `[2]`, `[3]`, ... suffixes.
- Added support for legacy short TOTP secrets encountered in real authenticator data.
- Added deterministic account search, keyboard selection, Enter-to-copy-and-exit, Escape cancel, and sensitive `wl-copy` handoff.
- Added opt-in startup tracing and maintained CI/release validation.
- Established Semantic Versioning, Conventional Commit history, and zero-spend GitHub Release distribution with versioned executable archives.

### Runtime requirements

- Linux.
- `wl-copy` from `wl-clipboard` for Wayland clipboard handoff.
- A freedesktop Secret Service implementation only when desktop-keyring integration is enabled.

### Security note

OTPick keeps authenticator secrets encrypted at rest, but running TOTP on the same machine as the authenticated service does not provide the same physical factor separation as a separate authenticator device.
