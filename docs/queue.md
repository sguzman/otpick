# Canonical Work Queue

OTPick is post-MVP. Completed MVP construction is preserved here so README can remain product-facing rather than becoming a development diary.

## Q000 — Product formalization

**Status: DONE**

- [x] Define one-shot authenticator/picker purpose.
- [x] Define sacred hot path and no-network invariant.
- [x] Define encrypted local-vault model.
- [x] Define unlock/session-key architecture.
- [x] Define import and backup boundaries.

## Q001 — Encrypted vault and session unlock

**Status: DONE**

- [x] Argon2id passphrase derivation.
- [x] XChaCha20-Poly1305 authenticated encryption.
- [x] versioned vault/header/plaintext schema.
- [x] atomic mode-0600 encrypted writes.
- [x] Linux session-keyring cache.
- [x] init/unlock/lock/status commands.

## Q002 — Authenticator ingestion

**Status: DONE / REAL EXPORT ACCEPTED**

- [x] standard `otpauth://totp` parsing.
- [x] SHA1/SHA256/SHA512 and common digit/period profiles.
- [x] Google Authenticator migration v1/v2 decoding.
- [x] multi-QR batch assembly.
- [x] PNG/JPEG/WebP QR decoding.
- [x] transaction-style combined import.
- [x] exact-credential deduplication.
- [x] same-label distinct credentials preserved.
- [x] presentation-only `[2]`/`[3]` disambiguation.
- [x] real Google Authenticator export imported successfully on target host.

## Q003 — One-shot picker

**Status: DONE / HOST ACCEPTED**

- [x] compact egui popup.
- [x] search focus on launch.
- [x] deterministic ranked filtering.
- [x] Up/Down selection.
- [x] Enter generates, copies, and exits.
- [x] Escape cancels.
- [x] Wayland clipboard survives UI exit via `wl-copy`.
- [x] legacy short TOTP secrets supported.
- [x] target-host release binary dogfooded.

## Q004 — Backup and diagnostics

**Status: DONE**

- [x] encrypted byte-for-byte backups.
- [x] default/override backup paths.
- [x] opt-in startup tracing.
- [x] CI format/test/Clippy/release-build gate.

## Q005 — Desktop secrets integration

**Status: DONE / DOGFOODING**

- [x] optional freedesktop Secret Service persistence.
- [x] Linux session keyring remains hot-path cache.
- [x] one-time `otpick keyring enable`.
- [x] explicit session lock prevents automatic rehydration.
- [x] fallback passphrase path preserved.
- [ ] observe cold-login shared-unlock behavior during normal dogfooding.

The unchecked observation is not an MVP blocker because desktop-keyring persistence is optional and the passphrase/session-keyring path is already functional.

## Q006 — MVP graduation and maintained release protocol

**Status: DONE**

- [x] principal accepted OTPick for daily use.
- [x] stable v1 compatibility contract recorded.
- [x] package version graduated to 1.0.0.
- [x] maintained changelog established.
- [x] Conventional Commit policy adopted.
- [x] zero-spend GitHub Release distribution defined.
- [x] versioned Linux x86_64 archive + checksum defined.

## Q007 — Desktop installation and summon binding

**Status: OPTIONAL / EXTERNAL INTEGRATION**

Installing the release binary into the user's normal PATH and wiring a Hyprland summon key are host integration, not unfinished MVP work.

Do not modify compositor configuration unless explicitly requested.

## Q008 — Post-MVP dogfood improvements

**Status: OPEN / EVIDENCE-DRIVEN**

Only add work here when real use exposes a concrete problem or a backwards-compatible capability worth maintaining.

Candidate classes include UI polish, search ergonomics, management UX, or startup measurement. Do not expand scope merely because adjacent authenticator features exist.

---

## Queue discipline

New product work is post-MVP maintenance.

Record concrete work here when it is accepted as project scope. Classify compatibility impact before release.
