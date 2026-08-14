# fusion-security-mod

Security headers middleware (demo)

## Naming

Recommended (not required):

- Python import package: `fusion_<name>_mod` (this package: `fusion_security_mod`)
- npm package: `fusion-<name>-mod` (this package: `fusion-security-mod`)

## Implementation

- Language: **rust**
- Manifest: `fusion.module.toml`

This is a normal library package. After `fusion add`, import it from your Fusion app.

## Usage in a Fusion app

```python
from fusion_security_mod import hello

print(hello("world"))
```

## Install into a Fusion app

```bash
fusion add --github YOUR_GITHUB_USERNAME/fusion-security-mod
```
