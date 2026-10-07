# Versioning and Release Contract

## Lifecycle

- regime: **maintained**
- current release: **v1.0.0**
- MVP accepted: **2026-10-07**
- MVP graduation release: **v1.0.0**
- SemVer adoption mode: **native**
- SemVer adoption boundary: **v1.0.0 graduation commit**
- SemVer adoption baseline: **v1.0.0**
- latest pre-adoption release/tag: **none**
- legacy version history: **not applicable**
- tag scheme: **vX.Y.Z**
- canonical release host: **GitHub Releases**
- initial binary target: **x86_64-unknown-linux-gnu**

The earlier Cargo `0.1.0` value was provisional development metadata. OTPick had no historical release tags or hosted releases before MVP graduation, so the maintained compatibility line begins cleanly at v1.0.0.

## Established v1 contract

### User workflow

A v1 user may reasonably depend on:

- running `otpick` with no arguments opens one compact picker;
- typing filters/ranks local account labels;
- the top result is selected automatically;
- Up/Down changes selection;
- Enter generates the selected account's current TOTP, copies it, and closes OTPick;
- Escape closes without copying;
- one invocation copies at most one TOTP;
- no network access is required for normal operation;
- no resident OTPick daemon is required.

The visual styling and exact row spacing are not stable compatibility surfaces.

### CLI and invocation

The maintained binary name is `otpick`.

The v1 command surface is:

- `otpick`
- `otpick init`
- `otpick unlock`
- `otpick lock`
- `otpick status`
- `otpick list`
- `otpick add`
- `otpick backup [DIR]`
- `otpick import SRC...`
- `otpick import -`
- `otpick keyring enable`
- `otpick keyring disable`
- `otpick --help`
- `otpick --version`

Compatible additions are normally minor-version work. Removing or incompatibly redefining an established command or its important semantics requires a major-version decision.

### Persisted state

Default vault discovery is:

- `$XDG_DATA_HOME/otpick/vault.otpvault`; or
- `~/.local/share/otpick/vault.otpvault` when XDG_DATA_HOME is unset.

`OTPICK_VAULT` overrides the full vault path.

The encrypted vault's on-disk format and plaintext schema are explicitly versioned. Existing maintained vaults must remain readable or receive an intentional migration path before an incompatible format change ships.

Backups are encrypted byte-for-byte vault snapshots. `OTPICK_BACKUP_DIR` controls the default backup destination.

### Import behavior

v1 supports standard TOTP `otpauth://` input, Google Authenticator migration payloads used by the accepted importer, multi-part Google export assembly, text/stdin input, and QR image input for PNG/JPEG/WebP.

Import is transactional at the combined-document level.

Exact duplicate credentials may be collapsed. Distinct credentials sharing the same issuer/account identity remain separate credentials; presentation-only `[2]`, `[3]`, ... suffixes do not mutate their stored issuer/account values.

### Unlock and keyring behavior

The vault passphrase is the recovery authority.

The Linux session keyring is the normal fast cache for the derived vault key.

Desktop Secret Service integration is optional. When enabled and available, OTPick may recover the derived vault key from the desktop keyring and repopulate the Linux session cache.

An explicit `otpick lock` must prevent automatic desktop-keyring rehydration for the current login session.

The exact desktop prompt presentation and whether PAM unlocks the desktop keyring automatically are external host behavior, not an OTPick compatibility promise.

### Clipboard/runtime

Wayland clipboard handoff uses `wl-copy` with the TOTP supplied over stdin rather than as a process argument.

The released prebuilt artifact target is x86_64 GNU/Linux. Linux is the maintained OS family; Wayland/Hyprland is the primary accepted deployment environment.

### Security contract

v1 promises an encrypted-at-rest local vault, no normal-operation network dependency, and no intentional plaintext secret file.

The specific cryptographic construction in v1 is Argon2id plus XChaCha20-Poly1305. Any future cryptographic migration must preserve access to existing maintained vaults or provide an explicit migration path.

Same-machine TOTP is explicitly not a promise of physical second-factor separation.

### Public API

OTPick v1 is an application, not a stable Rust library API. Internal Rust modules are not public compatibility surfaces.

## Release classification

- **major**: breaks an established contract above or another documented compatibility surface;
- **minor**: adds meaningful backwards-compatible capability;
- **patch**: fixes/reliability/performance/polish work without materially expanding or breaking the contract;
- **no release required**: internal/docs/test/CI/refactor/build/chore work with no release-worthy product change.

The highest-impact accepted change determines the bump.

## Commit protocol

Post-MVP commits use Conventional Commits.

Default signals:

- `feat` -> minor;
- `fix`, `perf`, release-relevant `deps` -> patch;
- `type!` or `BREAKING CHANGE:` -> major;
- `docs`, `test`, `ci`, `refactor`, `style`, `build`, `chore` -> no bump by themselves.

Actual compatibility semantics outrank the prefix.

## Release tooling

- `cliff.toml`: changelog generation from maintained Conventional Commit history;
- `release.toml`: cargo-release preparation policy;
- `.github/workflows/release.yml`: exact-candidate validation, packaging, tagging, and GitHub Release publication;
- Cargo manifest: package-version source of truth.

The release workflow only publishes when the main-branch commit subject is exactly:

`chore(release): prepare X.Y.Z`

and that X.Y.Z matches the Cargo package version.

Before publication it checks formatting, tests, strict Clippy, and the locked release build for that exact commit.

The workflow packages a version-bearing x86_64 GNU/Linux archive and SHA-256 checksum and publishes them directly to GitHub Releases. It does not use GitHub Actions artifact storage.

## Zero-spend boundary

OTPick is a public repository and uses the standard public-repository GitHub Actions + GitHub Releases path.

Do not switch to paid/larger runners, private metered build/storage, GitHub Packages, or another billable distribution service without explicit principal authorization.

If repository visibility or provider billing policy changes, re-check this assumption before the next automated release.

## v1.0.0 graduation evidence

The runtime promoted to v1.0.0 is the already-dogfooded release binary whose final product-code head before graduation was `3e628fe`.

The principal exercised the real optimized binary on EndeavourOS/Hyprland, imported a real multi-QR Google Authenticator export, used the picker against the resulting encrypted vault, verified Enter-to-copy-and-exit after the legacy-short-secret correction, enabled desktop-keyring integration, and explicitly accepted OTPick as ready for use.

The graduation commit changes lifecycle/release metadata and governance, not the accepted core runtime behavior.

Cold-login Secret Service behavior remains a normal dogfood observation because the passphrase/session-keyring fallback is functional and the optional persistence layer does not define the essential authenticator purpose.
