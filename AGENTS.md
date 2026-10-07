# AGENTS.md

This file defines working rules for AI and software agents operating in OTPick.

## Prime directive

**Do the work in the repository. Do not turn implementation into a wall of code or shell instructions in chat unless the user explicitly asks for that.**

Inspect the repository, make coherent changes, validate them, and keep human interaction limited to intent, taste, authority, or host behavior that cannot be observed through repository tooling.

## Product invariants

Never violate these casually:

- OTPick is one-shot.
- The normal picker path is local and network-free.
- Successful selection means generate one TOTP, copy it, and exit.
- Escape must cancel without copying.
- No resident OTPick daemon is required.
- Argon2 must stay off the repeated hot path.
- The encrypted vault is independent of repository location.
- Secrets must never be written to logs, process arguments, repository fixtures, release notes, screenshots, or diagnostics.
- Clipboard handoff must survive visible picker exit.
- Import of a Google Authenticator multi-QR batch is transactional.
- Exact duplicate credentials may be collapsed; same-label distinct credentials must remain representable.
- Desktop Secret Service is an optional persistence layer for the derived vault key, not a replacement for the encrypted vault or vault passphrase.
- Explicit `otpick lock` must prevent automatic desktop-keyring rehydration for the current login session.

## Security discipline

Treat all real authenticator material as highly sensitive.

Do not ask the user to upload real QR export screenshots or raw TOTP secrets.

Tests must use synthetic secrets only.

Do not add network access to the picker, import, unlock, or vault paths without an explicit product decision.

Do not expose current TOTP codes through list/status/debug output.

Changes to vault format, key derivation, encryption, keyring storage, import semantics, or backup behavior require explicit compatibility and migration analysis.

## Latency discipline

Treat every operation added before the first useful picker frame as suspect.

- Check the Linux session keyring before slower unlock paths.
- Do not run Argon2 during an already-unlocked picker invocation.
- Do not add network I/O.
- Do not add background services merely for convenience.
- Keep startup tracing opt-in and effectively zero-cost when disabled.
- Measure before claiming a startup optimization.

## Repository workflow

OTPick is post-MVP. Commit history is maintained product protocol.

- Prefer direct commits to `main` for owner-directed work.
- Do not create PR ceremony unless requested.
- Use Conventional Commit structure for every post-MVP commit.
- `feat` normally signals a minor release.
- `fix`, `perf`, and release-relevant `deps` normally signal a patch release.
- Breaking compatibility uses `!` and migration information when relevant.
- `docs`, `test`, `ci`, `refactor`, `style`, `build`, and `chore` do not force a release by themselves.
- Actual compatibility semantics outrank the commit prefix.
- Keep commits coherent and independently understandable.
- Update durable documentation when behavior or architecture changes.
- Keep the canonical work queue in `docs/queue.md`.

## Versioning and release ownership

Read `docs/versioning.md` before changing a stable contract or preparing a release.

The director owns routine version classification, changelog preparation, version mutation, tagging, and release bookkeeping.

The principal should not be asked to choose routine SemVer bumps or manually reconcile release metadata.

Maintained releases use GitHub Releases and a versioned x86_64 GNU/Linux archive while the zero-spend public-repository path remains available.

Do not enable paid runners, paid package storage, paid release hosting, or another billable release service without explicit authorization.

## Definition of done

A product change is done only when the relevant evidence exists:

- formatting is clean;
- tests pass;
- Clippy is warning-free;
- release build succeeds when applicable;
- user-visible behavior is documented;
- security implications are considered;
- compatibility impact is classified;
- host QA is requested only when host behavior cannot be established mechanically.

## User environment boundary

The user's compositor configuration, login stack, and desktop keyring service are external integration surfaces.

Do not modify Hyprland, PAM, GNOME Keyring, KWallet, or system login configuration unless the user explicitly asks for that integration work.
