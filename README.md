# Rust-First Server Clock (HTMX + SSE)

A server-rendered clock app that displays user-local time as `HH.MM.SS` (24-hour format), with Rust/WASM owning routing, request-context resolution, and non-stream HTTP response planning (`status`, `body`, `content-type`, `location`, `set-cookie`) via `typed_handle_http`; the Worker JavaScript host remains a thin runtime adapter.

## Documentation Scope

- Canonical project documentation is maintained in `README.md` and `AGENTS.md`.
- The `docs/` directory is intentionally removed to avoid drift with implementation.
- Update these two files when architecture or runtime behavior changes.

## Stack at a Glance

- **Build orchestration:** `pnpm` scripts and `just` targets
- **Server host runtime:** Cloudflare Workers via Wrangler
- **Core app logic:** Rust compiled to `wasm32-unknown-unknown`
- **Client update mechanism:** HTMX + SSE extension from CDN
- **Rendering model:** Server-side only

## What Runs Where

### Build layer

- `pnpm run build:wasm` compiles Rust crate `rust/clock-wasm` to `clock_wasm.wasm`.
- Link step uses `wasm-ld` from `PATH` (provided by the flake dev shell).

### Host runtime (V8 / JavaScript)

- Worker host (`src/worker.mjs`) loads wasm and stays host-only:
  - pass raw request inputs into Rust `typed_handle_http` for non-stream HTTP planning
  - emit `Response` objects from Rust planner outputs (`status`, `body`, `content_type`, `location`, `set_cookies`)
  - serve `/static/*` through the Cloudflare `ASSETS` adapter
  - manage SSE transport lifecycle (open/close/timer/cancel) for `/clock-stream`

### Rust/WASM runtime

- Rust owns:
  - route matching (`method + path`) and request-context resolution
  - timezone hint normalization and query/header/cookie precedence resolution
  - timezone-aware clock conversion from unix seconds via `chrono` + `chrono-tz`
  - strict clock formatting contract (`HH.MM.SS`)
  - home HTML generation from static `public/index.html` template
  - SSE payload generation via `typed_render_sse`
  - not-found payload generation
  - non-stream HTTP response planning via `typed_handle_http`

### Browser runtime

- Browser receives server-rendered HTML and CSS.
- Browser runs `/static/timezone-bootstrap.js` to detect timezone via `Intl.DateTimeFormat().resolvedOptions().timeZone`.
- On first load, browser sends timezone in `?tz=<IANA zone>` so host can persist it for the session.
- Browser loads HTMX scripts from CDN:
  - `https://unpkg.com/htmx.org@1.9.12`
  - `https://unpkg.com/htmx.org@1.9.12/dist/ext/sse.js`
- Browser opens SSE and swaps incoming `<time>` fragments using HTMX attributes.
- Theme switching is HTMX-only and server-side; no client-side JavaScript is used for theme state.

## Architecture

The host adapter in the repository is:

1. **Worker adapter** (`src/worker.mjs`)
   - Wrangler/Cloudflare Worker runtime entrypoint.
   - Imports `.wasm` module directly and serves `/static/*` via `env.ASSETS`.
   - Emits non-stream HTTP responses from Rust `typed_handle_http` plans.
   - Owns SSE stream transport lifecycle for `/clock-stream`.

## Request Flow

1. Browser loads `GET /`; bootstrap script detects browser timezone.
2. Browser sends timezone (`?tz=<IANA zone>`) on initial request when needed.
3. For non-stream endpoints, host forwards request transports (`method`, `path`, query, cookies, `x-timezone`, `HX-Request`) plus unix seconds to Rust `typed_handle_http`.
4. Rust returns the response plan (`status`, `body`, optional `content_type`, optional `location`, and `set_cookies` directives).
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

- Node.js + pnpm
- Rust/Cargo toolchain
- Rust target `wasm32-unknown-unknown`
- `wasm-ld` available on `PATH` (included in the flake dev shell)

If a tool is missing:

```bash
nix run nixpkgs#<tool> -- <args>
```

## Quick Start

```bash
pnpm install
just dev 5656
```

Open `http://localhost:5656`.

## Commands

### just targets

```bash
just --list
just install
just wasm
just rust-test
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
pnpm run build:wasm
pnpm run test:rust
pnpm test
pnpm run dev
```

`pnpm run dev` uses Wrangler defaults; for protocol-compliant local debugging and verification use `just dev 5656`.

## Cloudflare Deployment

- Worker config: `wrangler.toml`
- Worker entrypoint: `src/worker.mjs`
- Route target: `clock.abc-p.com/*`
- Static assets: `public/` via Wrangler assets binding (`ASSETS`)

Secrets are loaded transiently for deploy commands via `scripts/with-cloudflare-env.sh`, which runs:

```bash
sops -d secrets.yaml | yq ... -> export CLOUDFLARE_* in-process only
```

No secrets are stored in repository files.

## Endpoints

### `GET /`

Returns fully server-rendered HTML including:

- semantic `<time id="clock-time" ...>`
- HTMX SSE attributes
- HTMX theme-toggle form (`POST /theme?theme=...`)
- Rust/WASM indicator text
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

4. **WASM boundary glue (`src/worker.mjs`)**
   - Required to marshal UTF-8 strings and primitive inputs/outputs across JS <-> WASM memory.

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

- Rust tests: `rust/clock-wasm/src/lib.rs`
- Automated live regression gate: `pnpm test` runs wasm build, Rust tests, and `just test-live` endpoint checks on port `5656` (fails fast if `5656` is already in use)
- Live checks assert HTTP semantics for status/content-type/location/set-cookie plus SSE transport behavior: `just verify-theme-redirect 5656` and `just verify-worker-flows 5656`

Run all gates:

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
|  `- worker.mjs         # Wrangler/Cloudflare Worker entrypoint
|- rust/clock-wasm/
|  |- Cargo.toml
|  `- src/lib.rs         # Rust planner/router + chrono/chrono-tz clock renderer + HTML/SSE exports
|- public/
|  |- favicon.ico
|  |- index.html
|  |- styles.css
|  `- timezone-bootstrap.js
|- scripts/
|  `- with-cloudflare-env.sh
|- wrangler.toml
|- .justfile
`- package.json
```

## Troubleshooting

### `linker 'lld' not found`

Build requires `wasm-ld` on `PATH`. Run `nix develop` to use the flake-provided `lld` toolchain.

### Rust target missing

Install `wasm32-unknown-unknown` for the active Rust toolchain.

### SSE not updating

Check stream output directly:

```bash
curl -fsS "http://127.0.0.1:5656/clock-stream?once=1"
```

Expected: one `event: clock` frame with a `<time>` payload.

### Wrong timezone displayed

Check timezone negotiation inputs:

```bash
curl -fsS "http://127.0.0.1:5656/?tz=America/New_York" | rg "07\.05\.07"
curl -fsS "http://127.0.0.1:5656/clock-stream?once=1" -H "Cookie: clock_tz=America%2FNew_York"
```

If browser timezone detection is unavailable or invalid, no `tz` hint is sent and Rust falls back to `UTC`.
