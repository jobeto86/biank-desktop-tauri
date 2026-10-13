# CUR-128 — Private profiles in public native CI

The existing native build consumes an immutable private core SHA and stages the
same pinned Codex distribution used by the installer. After staging, the mandatory
profile step runs the core's direct-profile regression suite on the native runner
and `check-provider-candidate` against `src-tauri/runtime`, the packaged resources.
It fails the candidate if those commands fail. Both Windows and macOS use it.

This adds validation only. It does not publish an installer, relax the launcher,
add a model account or copy private core sources into this repository. The existing
read-only core grant remains scoped to checkout; this adds no credentials.

The gate verifies native launch with private CODEX_HOME and tests environment
sanitization and profile copying. Its `--version` probe detects a newly created
personal .codex directory; it does not detect writes within a pre-existing one.
It does not certify a live model turn or customer recovery. CP-A504's first real
installation still requires evidence of a conversation in Biank's profile rather
than the person's CLI profile. Existing native account and installed-app smoke
steps remain mandatory and retain their own scope.

Dev validation: parse workflow, require step after staging and before account
smoke, check both commands and resource path, independently review diff. Native
execution must be taken from the exact workflow/core SHAs, not previous builds.
