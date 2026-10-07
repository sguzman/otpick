# Project Contract

## Mission

OTPick is a low-latency, one-shot TOTP authenticator and picker for Linux.

Its purpose is to make the normal second-factor lookup path extremely small:

`summon -> search -> copy -> gone`

It is not a general password manager, cloud authenticator service, browser extension, or account-synchronization platform.

## Product invariants

- The normal picker path is local and network-free.
- One invocation produces at most one copied TOTP.
- Enter on a selected result copies the current code and exits.
- Escape exits without copying.
- The picker must not require a resident OTPick daemon.
- Argon2 must not run on every picker invocation.
- The encrypted vault is independent of the repository or current working directory.
- Secrets must not be exposed through logs, process arguments, changelogs, screenshots committed to the repository, or release artifacts.
- Google Authenticator import must remain all-or-nothing for multi-part export batches.
- Import/display disambiguation must not mutate the source issuer/account identity merely to make labels unique.
- Desktop-keyring integration is an optional unlock convenience; the vault passphrase remains the recovery/fallback authority.

## Stable user workflow

The v1 workflow is:

`invoke -> type/search -> select -> Enter -> clipboard -> process exits`

Keyboard use is primary. Pointer use may exist without becoming required.

## Platform stance

### Maintained target

- Linux
- Wayland as the primary desktop path
- Hyprland as the first real deployment environment
- x86_64 GNU/Linux as the initial prebuilt release target

The application also builds with eframe's X11 support, but compositor-specific configuration remains outside this repository.

## Security boundary

OTPick protects authenticator secrets at rest and minimizes their exposure in process interfaces.

The encrypted vault is the durable source of authenticator state. The Linux session keyring and optional desktop Secret Service contain only the derived vault key needed to open that state.

Same-machine TOTP is a deliberate convenience/security tradeoff and does not provide the same physical separation as a separate authenticator device.

## Non-goals

OTPick is not:

- a network synchronization service;
- a cloud account;
- a password manager;
- a browser credential store;
- a resident tray daemon;
- a desktop keyring implementation;
- a replacement for hardware security keys;
- a general QR-code library product.

## Current state

- lifecycle: **maintained**
- MVP accepted: **2026-10-07**
- maintained line: **v1**
- graduation release: **v1.0.0**
- target-host acceptance: **real release binary exercised on EndeavourOS/Hyprland and accepted by the principal**
- normal picker: **working**
- encrypted vault/import path: **working**
- Google Authenticator migration v2 + multi-QR path: **working against a real export**
- desktop Secret Service integration: **implemented and enabled; cold-login shared-unlock behavior remains normal dogfood observation rather than an MVP blocker**
- release distribution: **versioned GitHub Release archive**
- host keybinding/install policy: **external to this repository unless explicitly requested**

The MVP gate is closed. Future work is maintenance or post-MVP product development.
