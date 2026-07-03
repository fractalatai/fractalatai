# Nanoclaw Patterns vs Fractalaw Micro-Apps

## What Nanoclaw Is

Nanoclaw is a personal AI assistant (WhatsApp bot) built on Claude Agent SDK. Single Node.js process, SQLite persistence, containers for isolation. Each chat group gets its own container + `CLAUDE.md` memory file. "Skills" are Claude Code instruction files (`.claude/skills/add-X/SKILL.md`) that teach Claude how to modify the codebase — not plugins, but AI-guided code transforms.

Architecture: `WhatsApp -> SQLite -> Polling loop -> Container (Claude Agent SDK) -> Response`

## Key Nanoclaw Patterns

| Pattern | How Nanoclaw Does It |
|---------|---------------------|
| **App isolation** | Linux containers (Docker/Apple Container) per group |
| **Extensibility** | "Skills" = markdown instructions that Claude Code follows to modify the codebase |
| **State** | Per-group CLAUDE.md memory + SQLite |
| **IPC** | Filesystem watchers between host and containers |
| **Concurrency** | Per-group message queue with global concurrency cap |
| **Philosophy** | Code-first, minimal, "modify the code not the config" |

## Pattern Translation to Fractalaw

### Good Idea: Skills as Micro-App Generators

**Nanoclaw pattern:** Skills aren't runtime plugins — they're instructions for Claude Code to add capabilities by modifying source.

**Translation:** We could have `.claude/skills/new-microapp/SKILL.md` files that teach Claude Code how to scaffold a new guest component. Instead of a CLI `fractalaw new-app`, the skill would know: create `guests/<name>/`, set up `Cargo.toml` with `cargo-component`, wire the WIT imports needed, add build target. Claude Code becomes the app generator.

**Verdict: Worth exploring.** This is low-effort (just markdown files) and plays to the strength of having Claude Code as the dev environment. No runtime cost, just better DX.

### Good Idea: Per-App Memory / CLAUDE.md

**Nanoclaw pattern:** Each group gets its own `CLAUDE.md` that accumulates context over time.

**Translation:** Each micro-app guest directory could have a `CLAUDE.md` describing its purpose, WIT interfaces it uses, known quirks, deployment tier. This already kind of exists in comments but formalising it means Claude Code can work on any guest with full context.

**Verdict: Worth exploring.** Practically free, improves Claude Code's ability to maintain individual micro-apps.

### Not So Good: Container-per-App (replacing WASM)

**Nanoclaw pattern:** Docker/Linux containers for isolation.

**Translation:** We already have something strictly better for our use case — WASM components with Wasmtime. Our sandboxing gives us:
- Microsecond instantiation (vs seconds for containers)
- Fuel metering for deterministic auditing (containers can't do this)
- Capability-based security via WIT (vs ambient authority in containers)
- Sub-10MB binaries (vs 100MB+ container images)
- Runs on edge devices (tablets, field laptops)

**Verdict: Not needed.** WASM components are the right isolation boundary for Fractalaw. Containers would be a step backwards for this domain.

### Interesting: Single-Process Hub with Queue

**Nanoclaw pattern:** One Node.js process, polling loop, per-group queue with concurrency limits.

**Translation:** Our planned App Supervisor (Phase 3, Session 5) needs exactly this — a registry + scheduler + concurrency control. Nanoclaw's `group-queue.ts` pattern (per-entity queue, global concurrency cap) maps directly to per-micro-app execution slots within our Wasmtime pooling allocator (already configured: 16 instance slots). The polling loop maps to our scheduled task triggers.

**Verdict: Worth studying the implementation.** Our pooling allocator already handles the hard part, but the queue/scheduling logic in nanoclaw could inform our supervisor design.

### Interesting: Filesystem-Based IPC

**Nanoclaw pattern:** Host-to-container communication via filesystem watchers.

**Translation:** We use WIT function calls (synchronous, typed) for host-guest communication and Arrow IPC bytes for data. This is much more structured. However, for *inter-app* communication (app A triggers app B), we're planning `fractal:events/emit` which is essentially a typed version of the same idea — loose coupling via observable state changes.

**Verdict: Validates our event-driven composition approach**, but WIT interfaces are the right mechanism, not filesystem watches.

### Not So Good: "Modify the Code Not Config"

**Nanoclaw pattern:** No config files, just edit source directly since the codebase is small enough.

**Translation:** Doesn't scale for us. We have multiple guest components, each with its own `Cargo.toml`, WIT imports, build targets. A micro-app manifest/registry is needed for the supervisor to know what exists, what tier it runs at, what schedule it follows. Pure code-modification works for a WhatsApp bot; not for a fleet of regulatory AI agents.

**Verdict: Not applicable at our scale.** We need an app registry (Phase 3, Session 5).

## Summary

| Nanoclaw Pattern | Applicable? | Notes |
|-----------------|-------------|-------|
| Skills as Claude Code instructions | Yes | Great for scaffolding new guests |
| Per-app CLAUDE.md memory | Yes | Low-effort DX improvement |
| Container isolation | No | WASM components are strictly better here |
| Single-process queue/scheduler | Partially | Queue pattern useful for supervisor design |
| Filesystem IPC | No | WIT interfaces + events are more appropriate |
| Code-over-config | No | Need manifest/registry at our scale |

## Next Steps If Pursuing

1. Create `.claude/skills/new-guest/SKILL.md` — scaffold a new micro-app guest
2. Add `CLAUDE.md` to each guest directory with app-specific context
3. Study nanoclaw's `group-queue.ts` and `task-scheduler.ts` before designing the App Supervisor
