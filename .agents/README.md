# Fusion Framework — Agent Skills

Project-local skills for Cursor agents working on this repository.

## Layout

```
.agents/skills/<skill-name>/SKILL.md
.cursor/rules/*.mdc          # always-on / scoped project rules
```

Each skill teaches domain-specific workflows for Fusion (Rust core + Python / Node / C# bindings) and the companion `fusion` CLI.

## Available skills

| Skill | Use when |
|-------|----------|
| `fusion-architecture` | Repo layout, binding layers, where logic belongs |
| `fusion-bindings-parity` | Feature must land in Python **and** Node **and** C# |
| `fusion-coding-standards` | Comments, tests preference, git staging, skill/doc hygiene |
| `fusion-cli` | `fusion init` / commands / scaffold tree / CLI ↔ framework |
| `fusion-http-routes` | Routes, `http_get` / `[HttpGet]`, `[module]`, `[action]`, Swagger |
| `fusion-release` | Version bumps, manifests, publish prep |
| `fusion-testing` | Running checks; investigating failed tests |
| `fusion-cache` | Application cache (moka default; Redis later) |

## Always-on rules

`.cursor/rules/fusion-engineering.mdc` applies every session: parity across bindings, **examples in all three languages for new features**, function comments, prefer tests, never `git add .`, investigate failures, update skills when needed.

## Hygiene

When you add a new concept agents must remember, either extend an existing skill or add `.agents/skills/<name>/SKILL.md` and a row in this table.
