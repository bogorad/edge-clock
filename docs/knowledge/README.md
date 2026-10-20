---
type: Knowledge Bundle
title: Edge clock
description: Source and operating boundaries for the single clock Worker.
resource: ../../wrangler.toml
tags: [edge-clock, cloudflare, rust, emscripten]
timestamp: 2026-10-20T17:25:00Z
status: active
---

# Scope

This root bundle maps one Worker and its static assets. They share a release,
source tree, recovery path, and HTTP verifier. Separate child bundles would add
no independent operating boundary.

Primary parent: none. Owned children: none.

# Source ownership

- [AGENTS.md](../../AGENTS.md) owns agent rules and the live-local test limit.
- [Wrangler configuration](../../wrangler.toml) owns the declared Worker, route,
  assets binding, and compatibility settings.
- [The flake](../../flake.nix) and [its lock](../../flake.lock) own command-line
  tool inputs. [Cargo.toml](../../rust/clock-wasm/Cargo.toml) and
  [Cargo.lock](../../rust/clock-wasm/Cargo.lock) own Rust dependencies.
- [Rust source](../../rust/clock-wasm/src/lib.rs) owns request planning,
  rendering, and cookie decisions. [The Worker adapter](../../src/worker.mjs)
  owns host response objects, asset access, and SSE transport.
- [README.md](../../README.md) is the operator entrypoint.
  [The Emscripten reference](../bindgen/emscripten.md) explains the binding
  boundary.
- Beads owns execution state. [Activity](../activity/) owns time-bounded evidence.
  Hindsight is supporting context, not source authority.

# Lifecycle and dependencies

Build, local validation, and publication are separate stages. The project
shell supplies the tools. Generated Emscripten modules belong to the Worker
artifact. Browser assets are served through the same deployment.

External dependencies are Cloudflare Workers and the pinned compiler/library
inputs. HTMX owns browser interaction; custom browser JavaScript is limited to
the timezone hint allowed by `AGENTS.md`.

# State and recovery

Preferences live in browser cookies. The app declares no persistent server
database. Beads storage is a separate task ledger.

[Encrypted secrets](../../secrets.yaml) supply deploy credentials through the
[command wrapper](../../scripts/with-cloudflare-env.sh). Do not copy values into
this bundle, activity notes, or generated files.

The README owns release and rollback commands. Record the prior provider
version before publishing. Source branches and live Worker versions have
separate recovery boundaries.

# Verification and maintenance

Use the [live HTTP gate](../../README.md#testing). Test only a server owned by
the test run on explicit port `5656`. Compilation alone does not prove endpoint
behavior. Provider deployment results are distinct from local HTTP evidence.

Update this map when source ownership, build targets, runtime bindings, state,
secrets loading, test entrypoints, or recovery procedures change. Keep deployment
results and health observations in Beads/activity.
