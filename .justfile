set shell := ["bash", "-c"]

# Show available recipes
default:
  @just --choose

# Install JS dependencies
install:
  pnpm install

# Build Rust/WASM output
wasm:
  pnpm run build:wasm

# Run Rust unit tests
rust-test:
  pnpm run test:rust

# Run complete quality gate (build + Rust + live worker checks)
test:
  pnpm test

# Start local development with Wrangler + live reload
dev port="3000":
  wrangler dev --local --live-reload --show-interactive-dev-session=true --port {{port}}

# Run automated live local worker checks (starts/stops port 5656 server)
test-live port="5656":
  @if ss -ltn "( sport = :{{port}} )" | rg -q LISTEN; then echo "Port {{port}} is already in use; refusing to start test-live server."; exit 1; fi
  @log=$(mktemp); \
  wrangler dev --local --live-reload --show-interactive-dev-session=true --port {{port}} >"$log" 2>&1 & pid=$!; \
  cleanup() { if kill -0 "$pid" 2>/dev/null; then kill "$pid"; fi; rm -f "$log"; }; \
  trap cleanup EXIT; \
  ready=0; \
  for i in $(seq 1 90); do \
    if curl -s -o /dev/null "http://127.0.0.1:{{port}}/"; then ready=1; break; fi; \
    sleep 1; \
  done; \
  if [ "$ready" -ne 1 ]; then echo "Timed out waiting for local worker on :{{port}}"; cat "$log"; exit 1; fi; \
  just verify-theme-redirect {{port}}; \
  just verify-worker-flows {{port}}

# Verify non-HTMX theme redirect/cookie behavior on live local target
verify-theme-redirect port="5656":
  @headers=$(mktemp); \
  curl -sS -D "$headers" -o /dev/null -X POST "http://127.0.0.1:{{port}}/theme?theme=dark"; \
  rg -qi '^HTTP/[0-9.]+ 303' "$headers"; \
  rg -qi '^location: /' "$headers"; \
  rg -qi '^set-cookie: clock_theme=dark' "$headers"; \
  curl -sS -D "$headers" -o /dev/null -X POST "http://127.0.0.1:{{port}}/theme?theme=sepia"; \
  rg -qi '^HTTP/[0-9.]+ 303' "$headers"; \
  rg -qi '^location: /' "$headers"; \
  rg -qi '^set-cookie: clock_theme=light' "$headers"; \
  curl -sS -D "$headers" -o /dev/null -X POST -H "Cookie: clock_theme=dark" "http://127.0.0.1:{{port}}/theme?theme=dark"; \
  rg -qi '^HTTP/[0-9.]+ 303' "$headers"; \
  rg -qi '^location: /' "$headers"; \
  if rg -qi '^set-cookie: clock_theme=' "$headers"; then echo "unexpected clock_theme cookie write when request matches cookie"; exit 1; fi; \
  rm -f "$headers"

# Verify HTMX theme and SSE behavior on live local target
verify-worker-flows port="5656":
  @tmp=$(mktemp -d); \
  curl -sS -D "$tmp/htmx.headers" -o "$tmp/htmx.body" -X POST -H "HX-Request: true" "http://127.0.0.1:{{port}}/theme?theme=dark"; \
  rg -qi '^HTTP/[0-9.]+ 200' "$tmp/htmx.headers"; \
  rg -qi '^content-type: text/html' "$tmp/htmx.headers"; \
  rg -qi '^set-cookie: clock_theme=dark' "$tmp/htmx.headers"; \
  rg -q 'id="page-root" data-theme="dark"' "$tmp/htmx.body"; \
  curl -sS -D "$tmp/htmx-same-theme.headers" -o "$tmp/htmx-same-theme.body" -X POST -H "HX-Request: true" -H "Cookie: clock_theme=dark" "http://127.0.0.1:{{port}}/theme?theme=dark"; \
  rg -qi '^HTTP/[0-9.]+ 200' "$tmp/htmx-same-theme.headers"; \
  rg -q 'id="page-root" data-theme="dark"' "$tmp/htmx-same-theme.body"; \
  if rg -qi '^set-cookie: clock_theme=' "$tmp/htmx-same-theme.headers"; then echo "unexpected clock_theme cookie write for matching HTMX theme request"; exit 1; fi; \
  curl -sS -D "$tmp/invalid.headers" -o "$tmp/invalid.body" -X POST -H "HX-Request: true" "http://127.0.0.1:{{port}}/theme?theme=sepia"; \
  rg -qi '^HTTP/[0-9.]+ 200' "$tmp/invalid.headers"; \
  rg -q 'id="page-root" data-theme="light"' "$tmp/invalid.body"; \
  curl -sS -D "$tmp/get-theme.headers" -o "$tmp/get-theme.body" "http://127.0.0.1:{{port}}/theme?theme=dark"; \
  rg -qi '^HTTP/[0-9.]+ 404' "$tmp/get-theme.headers"; \
  rg -q '^Not Found$' "$tmp/get-theme.body"; \
  curl -sS -D "$tmp/tz.headers" -o /dev/null "http://127.0.0.1:{{port}}/?tz=%20Asia%2FTokyo%20"; \
  rg -qi '^set-cookie: clock_tz=Asia%2FTokyo' "$tmp/tz.headers"; \
  curl -sS -D "$tmp/tz-invalid.headers" -o /dev/null "http://127.0.0.1:{{port}}/?tz=Mars%2FOlympus"; \
  if rg -qi '^set-cookie: clock_tz=' "$tmp/tz-invalid.headers"; then echo "unexpected clock_tz cookie for invalid timezone"; exit 1; fi; \
  curl -sS -D "$tmp/tz-header-fallback.headers" -o /dev/null -H "x-timezone: Europe/Paris" "http://127.0.0.1:{{port}}/?tz=Mars%2FOlympus"; \
  rg -qi '^set-cookie: clock_tz=Europe%2FParis' "$tmp/tz-header-fallback.headers"; \
  curl -sS -D "$tmp/tz-query-precedence.headers" -o /dev/null -H "x-timezone: Europe/Paris" "http://127.0.0.1:{{port}}/?tz=America%2FChicago"; \
  rg -qi '^set-cookie: clock_tz=America%2FChicago' "$tmp/tz-query-precedence.headers"; \
  curl -sS -D "$tmp/tz-same.headers" -o /dev/null -H "Cookie: clock_tz=Asia%2FTokyo" "http://127.0.0.1:{{port}}/?tz=Asia%2FTokyo"; \
  if rg -qi '^set-cookie: clock_tz=' "$tmp/tz-same.headers"; then echo "unexpected clock_tz cookie write when supplied timezone matches cookie"; exit 1; fi; \
  curl -sS -D "$tmp/sse.headers" -o "$tmp/sse.body" "http://127.0.0.1:{{port}}/clock-stream?once=1"; \
  rg -qi '^HTTP/[0-9.]+ 200' "$tmp/sse.headers"; \
  rg -qi '^content-type: text/event-stream' "$tmp/sse.headers"; \
  rg -q '^event: clock' "$tmp/sse.body"; \
  rm -rf "$tmp"

# Deploy to Cloudflare Workers (route configured in wrangler.toml)
wrangler-deploy:
  pnpm run build:wasm
  @scripts/with-cloudflare-env.sh wrangler deploy

# Validate deploy bundle/config without publishing
wrangler-deploy-dry-run:
  pnpm run build:wasm
  @scripts/with-cloudflare-env.sh wrangler deploy --dry-run
