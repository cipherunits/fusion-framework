# Secrets & publishing

Release by pushing a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

That triggers:

- `publish-pypi.yml` → PyPI package `fusion-framework`
- `publish-npm.yml` → npm package `fusion-framework`

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
