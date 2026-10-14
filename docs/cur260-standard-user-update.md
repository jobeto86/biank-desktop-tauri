# CUR-260 — Windows update with a non-administrative token (DEV)

## Observed scope and diagnosis

The reported contrast is an unsuccessful 0.3.8 → 0.3.9 update for a normal user and a successful update for an administrator. It is an observation, not a reproduced root cause. The historical engine HTTP 409 is not yet tied to this updater failure or to the `profile_changing` response fixed in private core 9d1370f.

Reviewed public base: 0fe3274c2371a1d01da50ac1baa2ea898f62e5c3. NSIS configuration is `currentUser`; the signed-update configuration uses quiet mode. The locked Tauri updater 2.13.2 invokes the installer with the Windows `open` verb, not `runas`, and exits its own process after successful launch. That launch success does not prove completion of installation: a later NSIS guard refusal is not collected by the caller. Do not change privilege requirements or weaken the process/file guard based on this observation.

The existing Windows gate installs and opens a candidate in its runner context. It also starts a candidate with a synthetic older-profile marker; this is a data-migration startup check, not a binary upgrade from an installed 0.3.8. It does not assert absence of administrative privileges. Existing healthy gates therefore cannot settle this report.

## Narrow native experiment

Run the existing real process/file preparation and compiled NSIS hook tests with a restricted token derived from the runner. The child must prove effective Administrators membership is false, debug privileges are unavailable, and its own fixture remains writable. Do not create Windows accounts, passwords, customer data, or machine policies. Preserve the existing live-process and locked-DLL refusal assertions, unrelated-process preservation, clean-install acceptance and fixture cleanup. A missing assertion or receipt fails the gate; no skip is a PASS.

A restricted token preserves the runner SID and home environment. It tests a non-administrative execution context, not a newly provisioned user's profile, ACLs or a specific installation. Passing this experiment does not certify the actual 0.3.8 → 0.3.9 updater, installed application UI or customer recovery. Native execution is pending; Linux static inspection is not Windows evidence.

## Acceptance still required

After the narrow experiment, reproduce the signed binary update from 0.3.8 to 0.3.9 in a writable per-user destination, with a synthetic sentinel and final executable version checked. Exercise success, busy-runtime refusal and locked-file refusal, collect the installer exit code, and require no data replacement on refusal. Preserve backup and drain contracts. Identify the actual failing phase before implementing a product fix. Record any inability to reproduce rather than attributing the incident to authentication or ERP roles.

No installer/release is published and no production installation is modified by this DEV work.

## Prepared evidence, 2026-10-13

Harness/workflow implementation: `ee836da`; `tools/windows-standard-user.test.ps1` and `.github/workflows/windows-standard-user.yml`. PowerShell 7.6.7 ARM64 downloaded from the official PowerShell release with its published asset SHA256 verified. PowerShell syntax and embedded C# compilation passed; workflow YAML parsed. A countertest invoking native mode on Linux failed as required and created no native receipt. Receipt of static checks: `~/.marvin/scratch/cur260/static-validation.json`. These are static checks, not native Windows assertions.

Independent documentation review by Hooke: PASS with explicit Windows NOT_RUN. Independent harness review and hosted execution remain separate gates. The new workflow is manual-only and publishes a sanitized receipt with five-day retention, not an installer.

## First hosted run, 2026-10-13

PR #7 integrated the reviewed head `739a315` as `c569a192338ddf700223864b88757389d9773064`. Actions run `38507013061` failed at the child dependency stage, before either installer guard was executed. Its retained receipt proves effective administrator membership false, Administrators SID deny-only, SeDebugPrivilege absent, own SID preserved and writable synthetic fixture. Child exit code is 1; owned fixture removal, job closure and token-handle closure are reported true. This is a partial native result, not a guard PASS or an updater reproduction.

The parent log confirms setup-node selected Node 22.23.3 and NSIS installation completed. The child's aggregate dependency bucket cannot distinguish NSIS access, existing script availability, Node resolution, Node execution or version validation. Split that bucket into fixed, sanitized stages and retain availability booleans plus the native Node exit code. Do not retain exception text, paths, SIDs, environment values or child output, and do not relax any assertion. The diagnostic change requires another native run before attributing a cause.

## Second hosted run, 2026-10-14

PR #10 merged the reviewed diagnostic head `e0fd82d` as `2f2250ddddc5fe2a5e4ae855797b5d5a77f554be`. The single authorized Actions run `38602815923` checked out `e0fd82d` and failed after 48 seconds in `dependency-node-execution`. Its receipt confirms NSIS and both harnesses available, Node command resolution, the restricted-token checks, writable synthetic fixture and cleanup. There is no `node_exit_code`: it does not prove that Node started or returned a non-zero code. Neither installer guard ran. This receipt certifies the branch checkout, not a runtime built from the merged revision.

The next local diagnostic records only whether the resolved command is an Application and whether its source names a file. A caught Node-launch error projects the fixed PowerShell category, numeric HResult and a Win32 native error code only when present in a bounded exception chain. It retains no error message, target, command text, path, environment or exception data. The lookup, invocation, version requirement and installer assertions remain unchanged. Direct/wrapped native-error and non-native error fixtures verify sanitization locally; C# static validation also passes. This is not Windows execution and does not identify the actual launch failure, binary-update outcome or historical HTTP409 cause.

The previous one-run authorization is consumed. No new PR, merge or hosted run for this candidate is implied by its local preparation. The actual installed update and customer recovery remain unverified; CUR-260 stays open.

## Third hosted run and local follow-up, 2026-10-14

PR #11 integrated the explicitly authorized head `fddd7f1` as `26a157c7080a3fc3d46b12f9e45883e37a0145f6`, preserving concurrent main changes. The single authorized run `38605104043` checked out `fddd7f1` and failed in 40 seconds before recording a Node exit code or executing the installer guards. Node resolved as an Application and its source was an existing file. The sanitized launch error was ResourceUnavailable, HResult -2146233087, Win32 code null. All owned-resource cleanup flags were true. These results do not identify the historical HTTP409 cause or certify an installed update.

Null meant no native code was found in eight InnerException levels. PowerShell exceptions can also expose an ErrorRecord. The local follow-up walks both links, bounds depth to eight exception levels and work to sixteen distinct exceptions, and rejects cycles. It adds an allowlisted top-level exception classification and an exact NativeCommandFailed boolean. Fixture tests cover nested native codes, cycles, depth boundaries, private-data exclusion and identifiers with misleading suffixes. No exception type name outside the allowlist, error text, target or path is serialized. The command invocation, token, guards and workflow remain unchanged.

This follow-up is local QA preparation only: native Windows execution NOT_RUN, no further hosted run or public integration authorized by the consumed `fddd7f1` decision. Console/standard-handle behavior remains an untested alternative hypothesis; no launcher correction is claimed.
# Direct launch diagnostic candidate (local only)

Run 38614661444 at bad90b691976e4d046cd527364d113dccbd4ba58 failed in 35 seconds at dependency-node-execution. The bounded nested projection still returned NativeCommandFailed with no Win32 code; both installer guards were not reached. Owned fixture, job and token handles were cleaned. PR12 integrated this diagnostic at 58812728cab9cb52ef3272a74314550149a98267; it does not establish the client HTTP409 cause.

The next local candidate changes approach: only after the original PowerShell Node invocation fails, launch that same resolved binary via System.Diagnostics.Process.Start with --version, redirected streams and a five-second process deadline. Preserve the original FAIL; record only a fixed outcome, numeric exit/Win32 code, Node22 boolean and cleanup boolean. Kill only the diagnostic child by its owned process handle on timeout, with a bounded exit wait. Never retain stdout, paths or exception text. Capture is capped at 64 stdout characters, with no accumulated stderr. Cleanup confirms only the owned process, not descendants; the Linux slow fixture uses exec and has no child tree. Process.Start itself is synchronous and has no helper-enforced deadline; the outer Windows job has a 12-minute ceiling. Local fixtures cover direct success, missing executable and the owned-process wait timeout; Windows restricted-token execution remains NOT_RUN. Publication and another Windows run require their own scope decision; the bad90b6 one-run authorization is consumed.
