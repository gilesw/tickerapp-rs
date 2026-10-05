# Releasing the crate

## Release contract

Use `Cargo.toml` as the version source. Tag the corresponding commit `vVERSION`,
for example `v0.1.0`. `mise run bump` is the only task that edits the manifest
and changelog, `mise run tag` the only one that creates or pushes refs, and
neither publishes. The publishing script reads the tag and requires it to match
the manifest; it never rewrites a version or makes a commit.

The same commit must be tested, pushed to the public repository and published
to crates.io. Pushing a `v*` tag runs `.github/workflows/release.yml`, which
lints, tests and then publishes that commit with `mise run release:publish`.

This follows Cargo's recommendation to version the manifest, keep a changelog
and tag the published commit. [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html).

## Before the first release

- Keep Git remote `origin` and `package.repository` in `Cargo.toml` pointing
  to the public repository.
- Confirm the crate name is available. For authentication, Cargo reads
  `CARGO_REGISTRY_TOKEN` from the environment directly; no login step is needed.
  GitHub Actions obtains a short-lived token through crates.io
  [Trusted Publishing](https://crates.io/docs/trusted-publishing), so no
  long-lived secret is stored. Register the publisher once under the crate's
  Settings on crates.io with repository owner `gilesw`, repository name
  `tickerapp-rs`, workflow filename `release.yml` and environment `crates-io`.
  Trusted Publishing cannot create a crate, so publish the first version
  locally. For a local publish, export `CARGO_REGISTRY_TOKEN` in the invoking
  shell, or use `mise run release:login` or another Cargo credential provider.
- The manifest permits crates.io using `publish = ["crates-io"]`. An error
  saying the manifest does not permit publication is a manifest configuration
  problem, not a login failure. The script respects this restriction in dry runs too.
- Install the toolchains and tools declared in `mise.toml`, including git-cliff,
  plus Python 3 for parsing Cargo's JSON metadata in the Bash scripts.
- Prepare the first release by hand: `mise run bump` requires a tag for the
  current version, so it serves every release after the first.

The public remote and repository metadata must be configured before the first
release. Preparing the workflow does not publish a crate.

## Prepare a version

1. From a clean checkout, run `mise run bump major|minor|patch`. It raises
   `package.version`, refreshes the `Cargo.lock` root entry with
   `cargo update --workspace`, and inserts `## X.Y.Z — YYYY-MM-DD` at the top of
   `CHANGELOG.md`, rendered by git-cliff (`cliff.toml`) from the subject line of
   every commit since the previous tag. It refuses to run when the current
   version has no tag, that tag is not the newest one reachable from HEAD, there
   are no new commits, or the changelog is not topped by the dated current
   section. Nothing is committed; a failure restores the three files.
2. Review `git diff` and generated-code changes, then run `mise run check`.
3. Inspect `cargo package --list`. The crate should contain its generated Rust
   code and build without downloading the specification or running the generator.
4. Commit the complete release, including the lockfile and generated output,
   for example `git commit -am "Release 0.2.0"`.
5. Merge the release commit to `main`, through a pull request or a direct push,
   then check out `main` and pull so HEAD is the merged commit.
6. Run `mise run tag`. It refuses to run anywhere but `main` with HEAD at the tip
   of `origin/main`, so a pull-request branch can never be tagged. It confirms the
   tag exists neither locally nor on origin and that the first changelog section is
   `## X.Y.Z — YYYY-MM-DD`, then creates the annotated tag `vX.Y.Z` at HEAD and
   pushes it, which starts the release workflow and publishes the crate.
   `mise run tag --no-push` only creates the tag and prints the push command:

```sh
git push origin refs/tags/v0.2.0
```

## Verify and publish

Pushing the tag is the release. The `release` workflow runs lint and test,
then `mise run release:publish` in a job bound to the `crates-io` environment.
`rust-lang/crates-io-auth-action` exchanges the job's OIDC identity for a
crates.io token that expires after 30 minutes and is revoked when the job
ends, and crates.io only accepts it from that repository, workflow file and
environment. The environment can also require a reviewer before the upload.
The job fails if the version is already on crates.io, which also makes an
accidental second tag push harmless.

To check or publish locally instead, from the project root (or a subdirectory):

```sh
mise run release          # validate and dry-run; detect the tag at HEAD
mise run release:publish  # validate, dry-run, then upload
```

Both tasks run from the directory containing `mise.toml`, using `config_root`.
With no tag argument, the script detects the unique `v*` tag pointing at HEAD,
such as `v0.1.0`, then verifies it against `Cargo.toml`. An absent or ambiguous
tag is an error. It does not select a tag from an older commit.

To choose explicitly when HEAD has more than one version tag:

```sh
mise run release v0.1.0
mise run release:publish v0.1.0
```

The underlying `scripts/publish.sh [vVERSION] [--publish]` remains available.
Task working directories and argument forwarding follow
[mise's task conventions](https://mise.jdx.dev/tasks/toml-tasks.html).

The default command performs a dry run. The script requires a clean checkout,
checks that the tag points to HEAD, checks the tag against the manifest version,
and verifies the identical tag exists on `origin`. It runs `mise run check`
(release-script tests, formatting, Clippy, tests and generation freshness), followed by
`cargo publish --dry-run --locked --registry crates-io`.

`release:publish` repeats those gates, then uploads with Cargo. It never commits,
tags, pushes, edits the manifest or passes a token on the command line.
Cargo reads the inherited `CARGO_REGISTRY_TOKEN` through its standard token
credential provider. The task and script do not copy it into a configuration
file, print it or expand it into command-line arguments. If using a custom
credential-provider configuration, ensure it includes `cargo:token` to enable
environment-token authentication. See [Cargo environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
and [registry authentication](https://doc.rust-lang.org/cargo/reference/registry-authentication.html).
Both modes read `origin` and may access
package registries; only `release:publish` (or the script's `--publish`) uploads a crate.

Cargo's dry run packages the source and checks that the unpacked crate builds.
It does not prove upload authorization or reserve a crate name/version.
[Cargo publish reference](https://doc.rust-lang.org/cargo/commands/cargo-publish.html).

## After publication or failure

Confirm the expected version appears on crates.io. Keep its Git tag unchanged.
Registry versions cannot be overwritten; corrections require a new version.
If Cargo reports an upload/index timeout, check crates.io before retrying:
the upload may already have succeeded. If the upload did not happen and the
tagged code is unchanged, re-run the failed workflow job, or run the script
locally against the same tag.

If code must change after the tag was pushed, prepare a new version/tag.
Use yanking for an unsuitable published version when appropriate; it is not
a replacement for publishing a corrected version.

For this single crate, short scripts are sufficient. If releases later need
coordinated workspace releases or release PRs, evaluate the tools linked by
Cargo, such as `cargo-release` or `release-plz`, before expanding the scripts
into a release framework.

## Maintaining the scripts

Run `mise run lint:scripts` after changes. It runs ShellCheck on `scripts/*.sh`
and the tests in `scripts/test-bump.py`, `scripts/test-tag.py` and
`scripts/test-publish.py`. The tests use disposable local Git repositories and
replace Cargo, mise and git-cliff with stubs, so they verify the release gates
without making registry requests or uploading anything.
