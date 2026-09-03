---
name: fusion-coding-standards
description: >-
  Coding standards for Fusion Framework: function comments, clarifying
  comments for complex code, preferred tests, failure investigation, and when
  to update skills/docs. Use when writing or reviewing code in this repo.
---

# Coding standards

## Comments

### Functions / methods

Every **new** public or non-trivial private function, method, or exported helper must have a one-line (or short) comment/docstring that states **what it does**.

| Language | Prefer |
|----------|--------|
| Python | Docstring or `#` above `def` |
| JavaScript | `/** … */` or `//` above the function |
| C# | `/// <summary>…</summary>` or `//` above the member |
| Rust | `///` for public items; `//` for local helpers when non-obvious |

Do not restate the name alone (`// get user` on `get_user`). Say the behavior or contract.

### Dense / hard sections

When code becomes branching-heavy, protocol-sensitive, or easy to break (OpenAPI fill, middleware chain, route slot mounting, FFI):

- Add short **why** comments at the tricky points.
- Prefer extracting a named helper with a docstring over a wall of uncommented logic.

## Tests (prefer writing them)

When you add behavior:

1. Prefer a test under `tests/python/`, `tests/node/`, `tests/csharp/`, or Rust `#[cfg(test)]`.
2. Mirror coverage across bindings when the feature is cross-binding (see `fusion-bindings-parity`).
3. Run the relevant script from `fusion-testing` before claiming done.

### When a test fails

1. Read the failure output (assertion, path, expected vs actual).
2. Trace to implementation (wrong name stripping, async not awaited, header case, version prefix, etc.).
3. Tell the user **what broke and why** in plain language.
4. Fix the code or the incorrect expectation — never silently weaken assertions without saying so.

## Git

- Stage **individual files only**. Never `git add .` / `git add -A`.
- Only commit when the user asks.

## Docs & skills hygiene

After introducing something agents or developers must know later:

| Change type | Update |
|-------------|--------|
| New public API / middleware / route option | Binding parity + **examples in all three languages** (`examples/<feature>.py`, `.mjs`, `.cs`) + relevant skill |
| New test layout or runner | `tests/README.md` + `fusion-testing` |
| CLI-facing scaffold contract | `fusion-cli` skill; coordinate with fusion-tool if generators break |
| Entirely new workflow | New `.agents/skills/<name>/SKILL.md` + row in `.agents/README.md` |

### Examples (required for new features)

Always add side-by-side usage demos so humans/agents can see the API shape:

```text
examples/<feature>.py
examples/<feature>.mjs
examples/<feature>.cs
```

Follow existing trios (`custom_http_routes.*`, `pagination.*`). Do not leave a language without an example when the feature exists in that binding.

Keep skills concise; link to code paths instead of pasting large dumps.

## Style reminders

- Match existing naming (snake_case Python, camelCase Node, PascalCase C#).
- Minimal diffs; no drive-by refactors.
- Shared logic → `fusion-core` when possible.
