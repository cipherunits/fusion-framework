# Secrets & publishing

## Branch flow

- **`dev`** — day-to-day development; the `CI` workflow (lint + tests) runs on push and PRs.
- **`main`** — release branch; merge `dev` → `main` after bumping versions with `scripts/set-version.sh`.
- **`publish.yml`** — runs on push to `main` and on `v*` tags; publishes PyPI, npm, and NuGet packages.

## Release

Bump versions, merge to `main`, or push a version tag:

```bash
./scripts/set-version.sh 1.2.7
git tag v1.2.7
git push origin main --tags
```

That triggers `publish.yml`, which calls:

- `publish-pypi.yml` → PyPI package `fusion-framework`
- `publish-npm.yml` → npm package `fusion-framework`
- `publish-nuget.yml` → NuGet package `Fusion-Framework`

## Required GitHub configuration

### PyPI

1. Preferred: Trusted Publishing for this repo, workflow `publish-pypi.yml`, environment `pypi`.
2. Optional fallback secret: `PYPI_API_TOKEN` (`pypi-...`).
3. Create GitHub Environment named `pypi`.

```bash
pip install fusion-framework
```

### npm

1. Create an npm Automation token.
2. Add repo secret `NPM_TOKEN`.
3. Create GitHub Environment named `npm`.

```bash
npm install fusion-framework
```

## Local dry-run

```bash
cd crates/fusion-py && maturin build --release
cd crates/fusion-node && npm install && npm run build
```
