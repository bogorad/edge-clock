# Rust Emscripten build contract

The app uses Rust's `wasm32-unknown-emscripten` target. It still produces
WebAssembly. Cloudflare describes this integration as an experimental preview;
it is not a native machine-code Worker.

The build preserves the existing `#[wasm_bindgen]` exports, including
`typed_handle_http` and `typed_render_sse`. Their JSON response contracts remain
in the Rust source. The Worker adapter owns HTTP response objects, static asset
fetches, and SSE transport. Browser interactions remain HTMX-only.

## Build boundary

- Emscripten needs a Rust binary target, with an empty `main()`. A reusable
  `rlib` can hold the application code. Do not build a `cdylib` side module.
- The Emscripten compiler invokes wasm-bindgen during linking. The crate and
  CLI versions must match. Do not run the old separate bindgen generation step.
- The project flake and Cargo lock file own the tool and library versions.
  Use the project dev shell for builds and deployment.
- Generated JavaScript and WebAssembly form one artifact. Keep their relative
  paths together. Include generated snippets when the bindings need them.
- The Worker imports the generated named exports. Do not add a second
  `WebAssembly.Instance`, `initSync`, or `__wbg_set_wasm` bootstrap.
- Module initialization uses the supported Emscripten output mode. The Worker
  compatibility flags in `wrangler.toml` must support that mode.

The source owners are [flake.nix](../../flake.nix),
[Cargo.toml](../../rust/clock-wasm/Cargo.toml),
[package.json](../../package.json), and
[wrangler.toml](../../wrangler.toml). This reference does not replace their
settings.

## Verification and recovery

Use the commands in [README.md](../../README.md). Compile the actual artifact,
then validate the Worker through real HTTP requests on explicit port `5656`.
Never accept an artifact build as proof that the Worker loader, routes, cookies,
static assets, or SSE stream work.

Keep the prior Cloudflare version identifier before publishing. If the new
deployment fails, use the documented Wrangler rollback procedure. The source
branch is a separate publication boundary from the live Worker.

## Sources

- [Cloudflare announcement](https://blog.cloudflare.com/rust-workers-emscripten-target/)
- [wasm-bindgen 0.2.129 Emscripten guide](https://github.com/wasm-bindgen/wasm-bindgen/blob/0.2.129/guide/src/reference/emscripten.md)
- [Cloudflare worker-build 0.8.7 Emscripten implementation](https://github.com/cloudflare/workers-rs/blob/v0.8.7/worker-build/src/emscripten.rs)
- [Cloudflare compatibility flags](https://developers.cloudflare.com/workers/configuration/compatibility-flags/)
