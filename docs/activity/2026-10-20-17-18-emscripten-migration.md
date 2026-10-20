# Emscripten migration baseline

The operator authorized migration to the Cloudflare Emscripten Rust workflow,
live local tests, deployment of the configured Worker, and a commit/push on
`cf-wk-rust`. The branch was created from `origin/dev`. Bead `edge-clock-3oh`
owns the work. The pre-existing `.beads.gate.lock` remains outside this change.

Activity notes for this work use `docs/activity/`. The existing Hindsight bank
is `proj-edge-clock`; its baseline and committed ADR index were refreshed.
Live Beads routing uses server mode. The April embedded-mode note is historical.

The app currently builds `wasm32-unknown-unknown`. The requested Emscripten
workflow still produces WebAssembly and is described by Cloudflare as an
experimental preview. Preserve the Rust planner, HTTP contracts, SSE transport,
static assets, and HTMX-only UI behavior. Do not change ADR files.

Source: <https://blog.cloudflare.com/rust-workers-emscripten-target/>.
This note records the baseline; it does not claim a build, test, or deployment.
