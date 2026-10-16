# Windows boot identity spike

Read-only, standalone Rust probe. It is not part of the application build,
packaging manifest or lock recovery. It does not change the clock, power state,
registry, credentials or privileges, and never terminates another process.

## Build and sample

On native Windows with Rust and Node already installed:

```powershell
rustc --edition 2021 -D warnings main.rs -o boot-identity-spike.exe
rustc --edition 2021 -D warnings --test main.rs -o boot-identity-tests.exe
./boot-identity-tests.exe --nocapture
node validate.mjs ./boot-identity-spike.exe ./before.json
```

`validate.mjs` starts twelve independent probe processes and writes a receipt
exclusively (it will not replace an existing receipt). Preserve the exact binary;
the receipt includes its SHA-256, API status, returned length, OS build and samples.
The binary prints one JSON sample and accepts no arguments. API failure, missing
function, unexpected layout or empty GUID yields `unknown`, with no timestamp or
uptime fallback. Consumers must inspect JSON status; exit 0 alone is not success.

The probe reads `SystemBootEnvironmentInformation` (decimal 90), resolving
`NtQuerySystemInformation` dynamically from the already loaded ntdll. Only the
known 32-byte layout is accepted. Compatibility with a different layout is
deliberately unknown until investigated.

## Measured on 2026-10-15

[Native run 38844091960](https://github.com/jobeto86/biank-desktop-tauri/actions/runs/38844091960),
source `500210aaebba3f7e1fce88d48b8a558f8fff2cec`:

- Windows Server 2022, build `10.0.20348`: 4 tests passed, 12 independent samples
  with one stable nonzero boot GUID, status `0x00000000`, returned length 32.
- Windows Server 2025, build `10.0.26100`: same result.
- Unit tests reject failed API calls, zero GUIDs and incompatible layouts. A
  native invalid-information-class call also produces unknown.

**Verdict: API_AND_SAME_BOOT_PASS_ONLY.** Hosted runners execute under their
existing token; this is not a standard-account test. Two runner machines with
different IDs do not constitute a before/after reboot experiment. Windows Server
evidence does not certify Windows 10/11 client power management.

## Required persistent-host experiment

Use an explicitly authorized Windows DEV host and window. These instructions do
not themselves authorize a reboot or disruption of work. No automatic power or
clock change is included in the probe or workflow.

1. Record Windows edition/build, token type, binary hash and relevant power-mode
   availability. Do not include user names, credentials or unrelated diagnostics.
2. As an actual standard user, run tests and capture `standard-user-before.json`.
3. Leave an owned synthetic sentinel process alive, with no user data or tokens.
   Capture another receipt immediately before each authorized transition.
4. After ordinary suspend/resume and hibernate/resume, capture a fresh receipt
   with the **same binary on the same host**, and record whether the same sentinel
   survived. If it survived, a changed boot ID is a blocking counterexample to
   using this API as proof that descendants died.
5. For an authorized real restart, preserve before/after receipts and independent
   evidence that a restart actually occurred. Expect the old sentinel to be gone
   and a different GUID. Different GUIDs on unrelated hosts do not prove this.
6. Separately record shutdown/start with Fast Startup and sentinel survival.
   Do not infer ordinary-reboot semantics from this event. An unchanged ID while
   all user processes died is a conservative false negative; a changed ID while
   an old descendant survives is unsafe.
7. Clock-adjustment stability also needs an approved isolated host experiment;
   the probe does not perform it. No wall-clock estimate is used in the query.

Run `node validate.mjs ./boot-identity-spike.exe ./after-<event>.json` after each
event. Compare the actual `snapshots[*].bootId` values, binary SHA-256, OS build
and event evidence; writing a phase name alone cannot certify a lifecycle event.
Keep all pending cases explicit. Until these cases pass on supported clients,
do not enable automatic previous-boot recovery in the product.

## Sources and support limitation

- [Microsoft API and compatibility warning](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntquerysysteminformation).
- [System Informer/phnt: information class and native ABI](https://github.com/winsiderss/phnt/blob/master/ntexapi.h).

The ABI is an internal Windows interface. Successful experiments establish
behavior on the tested builds, not a future Microsoft support guarantee.
