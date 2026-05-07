# Changesets

This folder records pending releases for the npm packages in this repo
(currently just `@gentleduck/md`).

## Add a changeset

```sh
pnpm changeset
```

Pick the package(s), the bump type (patch / minor / major), and write a
short summary. The CLI drops a markdown file in `.changeset/`. Commit it
with the change that motivated the bump.

## Releasing

The `Changesets` GitHub workflow opens a "Version Packages" PR whenever
unconsumed changesets land on `master`. Merging that PR:

1. bumps versions + writes `CHANGELOG.md` (already done in the PR),
2. creates a `@gentleduck/md@<version>` git tag and pushes it,
3. the tag push triggers `napi-prebuilds.yml`, which builds the seven
   prebuilt `.node` binaries and runs `pnpm publish` to npm.

Crates.io is handled separately by `release.yml` (release-plz). The two
flows share `master` but use different tag namespaces and never collide.

## Ignored packages

`duck-mc` (workspace root) and `@gentleduck/md-sidecar` are private and
excluded in `config.json`. Examples under `examples/*` are not workspace
publish targets.
