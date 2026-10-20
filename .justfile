set shell := ["bash", "-c"]

# Show available recipes
default:
  @just --choose

# Install JS dependencies
install:
  pnpm install

# Build the Rust Worker and Emscripten output
build:
  pnpm run build:worker

# Compatibility alias for the Worker build
wasm: build

# Run Rust unit tests only when explicitly requested
rust-test:
  pnpm run test:rust

# Run live local HTTP checks
test:
  pnpm test

# Start local development with Wrangler + live reload
dev port="3000":
  wrangler dev --local --live-reload --show-interactive-dev-session=true --port {{port}}

# Verify one owned just dev start with a healthy HTTP response
verify-dev-start port="5656" timeout="300":
  @bash scripts/test-live.sh verify-dev-start {{port}} {{timeout}}

# Run live HTTP checks with one owned server on port 5656
test-live port="5656" timeout="300":
  @bash scripts/test-live.sh test-live {{port}} {{timeout}}

# Verify theme redirects against an already-running local target
verify-theme-redirect port="5656":
  @bash scripts/test-live.sh verify-theme-redirect {{port}}

# Verify page, HTMX, timezone, static asset, and SSE contracts on a local target
verify-worker-flows port="5656":
  @bash scripts/test-live.sh verify-worker-flows {{port}}

# Deploy to Cloudflare Workers (route configured in wrangler.toml)
wrangler-deploy:
  pnpm run build:worker
  @scripts/with-cloudflare-env.sh wrangler deploy

# Validate deploy bundle/config without publishing
wrangler-deploy-dry-run:
  pnpm run build:worker
  @scripts/with-cloudflare-env.sh wrangler deploy --dry-run
