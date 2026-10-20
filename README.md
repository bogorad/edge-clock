# Rust-First Server Clock (HTMX + SSE)

A server-rendered clock app that displays user-local time as `HH.MM.SS` (24-hour format). Rust owns routing, request context, and HTTP response planning through `typed_handle_http`. A thin Cloudflare Worker adapter handles HTTP objects, assets, and SSE transport.

Rust builds with `wasm32-unknown-emscripten`. This is Cloudflare's experimental Emscripten integration for Rust Workers. It still produces WebAssembly. The migration replaces the old `wasm32-unknown-unknown` build and custom module bootstrap; it does not change the browser UI model.

## Documentation Scope

- Canonical project documentation is maintained in `README.md` and `AGENTS.md`.
- `docs/bindgen/` is kept in-repo as reference guidance for Rust/WASM interop work.
- Keep `README.md` as the runtime source of truth; update `docs/bindgen/` when bindgen guidance changes.
- [The knowledge map](docs/knowledge/README.md) links source ownership and recovery boundaries.
- [The Emscripten contract](docs/bindgen/emscripten.md) defines the binding boundary.
- Activity notes for the migration are in `docs/activity/`; Beads owns task state.

## Stack at a Glance

- **Build orchestration:** `pnpm` scripts and `just` targets
- **Server host runtime:** Cloudflare Workers via Wrangler
- **Core app logic:** Rust compiled to `wasm32-unknown-emscripten`
- **Client update mechanism:** HTMX + SSE extension from pinned local static assets
- **Rendering model:** Server-side only

## What Runs Where

### Build layer

- `pnpm run build:worker` compiles the Rust binary in `rust/clock-wasm`.
- The project dev shell supplies the pinned Rust target, Emscripten, and matching wasm-bindgen CLI.
- Emscripten invokes wasm-bindgen during linking and emits `clock_wasm.js` plus `clock_wasm.wasm` in `rust/clock-wasm/pkg/`.
- The Worker imports the generated named exports. There is no separate bindgen generation step or handwritten WebAssembly instantiation.

### Host runtime (V8 / JavaScript)

- Worker host (`src/worker.mjs`) loads wasm and stays host-only:
  - pass raw request inputs into Rust `typed_handle_http` for non-stream HTTP planning
  - emit `Response` objects from Rust planner outputs (`status`, `body`, `content_type`, `location`, `set_cookies`)
  - serve `/static/*` through the Cloudflare `ASSETS` adapter, including its canonical redirects and missing-file responses
  - manage SSE transport lifecycle (open/close/timer/cancel) for `/clock-stream`

### Rust/WASM runtime

- Rust owns:
  - route matching (`method + path`) and request-context resolution
  - timezone hint normalization and query/header/cookie precedence resolution
  - timezone-aware clock conversion from unix seconds via `chrono` + `chrono-tz`
  - strict clock formatting contract (`HH.MM.SS`)
  - home HTML generation from static `public/index.html` template
  - SSE payload generation via `typed_render_sse`
  - non-asset not-found payload generation
  - non-stream HTTP response planning via `typed_handle_http`

### Browser runtime

- Browser receives server-rendered HTML and CSS.
- Browser runs `/static/timezone-bootstrap.js` to detect timezone via `Intl.DateTimeFormat().resolvedOptions().timeZone`.
- On first load, browser sends timezone in `?tz=<IANA zone>` so host can persist it for the session.
- Browser loads HTMX scripts from pinned local static assets:
  - `/static/vendor/htmx.org@1.9.12/htmx.min.js`
  - `/static/vendor/htmx.org@1.9.12/dist/ext/sse.js`
- Browser opens SSE and swaps incoming `<time>` fragments using HTMX attributes.
- Theme switching is HTMX-only and server-side; no client-side JavaScript is used for theme state.

## Architecture

The host adapter in the repository is:

1. **Worker adapter** (`src/worker.mjs`)
   - Wrangler/Cloudflare Worker runtime entrypoint.
   - Imports the generated Emscripten JavaScript module and serves `/static/*` via `env.ASSETS`.
   - Emits non-stream HTTP responses from Rust `typed_handle_http` plans.
   - Owns SSE stream transport lifecycle for `/clock-stream`.

## Request Flow

1. Browser loads `GET /`; bootstrap script detects browser timezone.
2. Browser sends timezone (`?tz=<IANA zone>`) on initial request when needed.
3. For non-stream endpoints, host forwards request transports (`method`, `path`, query, cookies, `x-timezone`, `HX-Request`) plus unix seconds to Rust `typed_handle_http`.
4. Rust returns the response plan as JSON (`status`, `body`, optional `content_type`, optional `location`, and `set_cookies` directives).
5. Host emits the HTTP response from that plan; no per-route business branching or cookie decision logic is implemented in JS.
6. For `/clock-stream`, host manages SSE transport lifecycle and calls Rust `typed_render_sse` for payload frames.
7. Theme toggle still returns `200` swap payloads for HTMX and `303 Location: /` for non-HTMX, based on Rust planner output.

## Behavior Contract

- Output format is exactly `HH.MM.SS`.
- Hours are `00`-`23`.
- Minutes and seconds are `00`-`59`.
- Values are zero-padded.
- Timezone source precedence is resolved in Rust: client-provided IANA timezone (`tz` query), then `x-timezone` header, then persisted cookie (`clock_tz`).
- Theme source precedence is resolved in Rust: `POST /theme?theme=<light|dark>` input, then persisted cookie (`clock_theme`), defaulting to `light`.
- Invalid theme values are normalized to `light`, and Rust decides whether a new cookie write is required.
- For non-stream endpoints, Rust planner output defines response status/body/content-type/location/set-cookie; host emits it.
- Theme switching is HTMX-only and server-side; no client-side theme state logic is used.
- If Rust formatter rejects invalid values, host does not silently substitute JS formatting.

## Prerequisites

- Use the committed Nix flake and lock file.
- Build host: `x86_64-linux`. The matching LLVM SDK binary is pinned for this platform.
- The shell supplies Node.js, pnpm, Wrangler, Rust with its Emscripten standard library, Emscripten 6.0.10, and wasm-bindgen 0.2.129.
- The wasm-bindgen crate and CLI versions must match.
- Deployment credentials remain encrypted in `secrets.yaml`.

Enter the declared shell:

```bash
nix develop --command bash
```

Do not substitute global installs or a package runner for the pinned tools.

## Quick Start

```bash
pnpm install
pnpm run build:worker
just dev 5656
```

Open `http://localhost:5656`.

## Commands

### just targets

```bash
just --list
just install
just test
just test-live 5656
just dev 5656
just verify-theme-redirect 5656
just verify-worker-flows 5656
just wrangler-deploy-dry-run
just wrangler-deploy
```

### pnpm scripts

```bash
pnpm run build:worker
pnpm test
just dev 5656
```

Use `just dev 5656` for local debugging. If port `5656` is occupied, stop; do not kill that server.

## Cloudflare Deployment

- Worker config: `wrangler.toml`
- Worker entrypoint: `src/worker.mjs`
- Route target: `clock.abc-p.com/*`
- Static assets: `public/` via Wrangler assets binding (`ASSETS`)
- The Emscripten loader uses `new_module_registry` and `nodejs_compat`; the manifest owns the compatibility date and flags.

Run deployment commands inside the project dev shell:

```bash
just wrangler-deploy-dry-run
scripts/with-cloudflare-env.sh wrangler deployments list
just wrangler-deploy
```

Record the current version ID before publishing. The dry run checks packaging;
it does not prove a successful deployment. Deployment must follow the live local
HTTP gate. Record the provider's deployment result in Beads or activity.

The wrapper supplies credentials transiently to the deployment command.
Never print credentials or write decrypted values to disk. The repository
contains encrypted configuration only.

For a failed rollout, redeploy the recorded prior Worker version with Wrangler's
rollback command:

```bash
scripts/with-cloudflare-env.sh wrangler rollback <version-id> --message "Restore the prior clock release"
```

Keep the source commit and Cloudflare version ID together in
the release evidence. A Git push and a Worker deployment are separate actions.
Cloudflare's [rollback documentation](https://developers.cloudflare.com/workers/configuration/versions-and-deployments/rollbacks/)
defines the version and binding limits.

## Endpoints

### `GET /`

Returns fully server-rendered HTML including:

- semantic `<time id="clock-time" ...>`
- HTMX SSE attributes
- HTMX theme-toggle form (`POST /theme?theme=...`)
- Rust/emscripten labels linked to the Cloudflare Emscripten article
- timezone bootstrap script (`/static/timezone-bootstrap.js`)

Optional timezone hints:

- `?tz=<IANA timezone>` (for example `?tz=America/New_York`)
- `x-timezone: <IANA timezone>` request header

Rust resolves supplied/session timezone precedence and returns cookie directives via `typed_handle_http`; host emits returned `Set-Cookie` headers and Rust uses the resolved timezone for rendering.
Theme is read from `clock_theme` cookie and defaults to `light`.

### `POST /theme`

Updates theme preference server-side and persists cookie.

- request: `POST /theme?theme=light|dark`
- invalid values are normalized to `light`
- cookie writes follow Rust planner directives (no rewrite when value already matches cookie)
- HTMX request (`HX-Request: true`) returns `200` HTML payload; page swaps `#page-root` via `hx-target="#page-root"`, `hx-select="#page-root"`, and `hx-swap="outerHTML transition:true"`
- `hx-sync="this:replace"` prevents stacked toggle requests when users click quickly
- `hx-preserve` keeps the SSE clock panel stable across theme container swaps
- non-HTMX request returns `303 Location: /`
- `GET /theme` is not supported and returns `404 Not Found`

## JavaScript Usage Justification

JavaScript remains only as Worker runtime glue around Rust/WASM planning and rendering.

1. **HTTP runtime bridge (`src/worker.mjs`)**
   - Required to receive requests in the Worker runtime, call Rust `typed_handle_http`, and emit `Response` objects from returned plan fields.

2. **Static assets adapter (`src/worker.mjs`)**
   - Required to route `/static/*` through Cloudflare `env.ASSETS`.

3. **SSE transport lifecycle (`src/worker.mjs`)**
   - Required to own stream transport concerns (open/close/timer/cancel) and write `text/event-stream` frames.

4. **Generated Emscripten boundary**
   - Emscripten initializes the module and exposes wasm-bindgen exports.
   - The Worker exchanges the existing JSON payload strings with Rust; it does not instantiate a second module.

5. **Browser timezone bootstrap (`public/timezone-bootstrap.js`)**
   - Explicitly allowed exception: browser timezone detection/hinting for first-request localization.

All UI behavior (theme switching + live time updates) is HTMX-driven and server-rendered; there is no client-side JS state management for theme/UI flows.

### `GET /clock-stream`

Returns `text/event-stream` frames:

```text
event: clock
data: <time id="clock-time" datetime="17:12:41">17.12.41</time>
```

Use `?once=1` for one event then close (test/smoke convenience).

Timezone source precedence for SSE rendering:

1. `?tz=<IANA timezone>` query
2. `x-timezone` request header
3. `clock_tz` cookie

## Testing

- `pnpm test` runs the live local HTTP gate on explicit port `5656`.
- The test harness starts the app through `just dev 5656`, waits for a successful home response, and checks real endpoints.
- Checks cover HTTP status, content type, redirects, theme state, cookie directives, timezone precedence, static assets, and SSE behavior.
- Each failed request or assertion fails the command. Cleanup does not replace the failure status.
- The harness monitors its server log and stops only its own server process tree. It refuses to start if the port is occupied.
- Existing Rust unit and boundary-test sources remain reference material. They are not part of the default debug gate. `cargo test`, `cargo check`, `cargo clippy`, and JavaScript test flows require the authorization stated in `AGENTS.md`.

Run the live HTTP gate:

```bash
pnpm test
```

Run live endpoint checks only:

```bash
just test-live 5656
```

## Project Layout

```text
.
|- src/
|  `- worker.mjs                    # Wrangler/Cloudflare Worker entrypoint
|- rust/clock-wasm/
|  |- Cargo.toml
|  |- src/lib.rs                    # Rust planner, renderer, and wasm-bindgen exports
|  |- src/main.rs                   # Emscripten binary entrypoint
|  `- tests/boundary_wasm.rs         # existing reference tests; not the default live gate
|- public/
|  |- favicon.ico
|  |- index.html
|  |- styles.css
|  |- timezone-bootstrap.js
|  `- vendor/htmx.org@1.9.12/       # pinned local HTMX + SSE extension assets
|- docs/bindgen/
|  |- bindgen.md
|  |- chapters.md
|  `- emscripten.md
|- scripts/
|  `- with-cloudflare-env.sh
|- wrangler.toml
|- .justfile
`- package.json
```

## Troubleshooting

### Missing or mismatched compiler tools

Enter the project dev shell. Check the declared Emscripten and wasm-bindgen
versions before rebuilding. Do not fall back to an older bindgen CLI.

### Rust target missing

The project toolchain must provide the `wasm32-unknown-emscripten` standard
library for its Rust compiler. A host-only Rust compiler is insufficient.

### SSE not updating

Check stream output directly:

```bash
curl -fsS "http://127.0.0.1:5656/clock-stream?once=1"
```

Expected: one `event: clock` frame with a `<time>` payload.

### Wrong timezone displayed

Check timezone negotiation inputs:

```bash
curl -fsS "http://127.0.0.1:5656/?tz=America/New_York" | rg '<time id="clock-time" datetime="[0-9]{2}:[0-9]{2}:[0-9]{2}">[0-9]{2}\.[0-9]{2}\.[0-9]{2}</time>'
curl -fsS "http://127.0.0.1:5656/clock-stream?once=1" -H "Cookie: clock_tz=America%2FNew_York" | rg '^event: clock'
```

If browser timezone detection is unavailable or invalid, no `tz` hint is sent and Rust falls back to `UTC`.
