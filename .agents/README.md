# Fusion Framework — Agent Skills

Project-local skills for Cursor agents working on this repository.

## Layout

```
.agents/skills/<skill-name>/SKILL.md
```

Each skill teaches the agent domain-specific workflows for Fusion (Rust core + Python / Node / C# bindings).

## Available skills

| Skill | Use when |
|-------|----------|
| `fusion-architecture` | Understanding repo layout, binding layers, where logic belongs |
| `fusion-bindings-parity` | Changing behavior that must stay aligned across Python, Node, C# |
| `fusion-http-routes` | Routes, `@http_get` / `[HttpGet]`, `[module]`, `[action]`, Swagger |
| `fusion-release` | Version bumps, manifests, publish prep |
| `fusion-testing` | Running checks and binding-specific tests |

Skills are loaded when the task matches the skill description (or when you name the skill explicitly).
