# Emscripten release

Bead: `edge-clock-3oh`. Source branch: `cf-wk-rust`.

The clock now builds with `wasm32-unknown-emscripten`. The existing Rust planner
and renderer remain behind generated wasm-bindgen exports. The Worker imports
the generated module. Browser assets, HTMX behavior, and ADR files were not
changed.

The pinned build uses Rust 1.97.0, Emscripten 6.0.10 with its matching LLVM 24
SDK, bindgen 0.2.129, and Wrangler 4.148.0. The binary SDK build host is
`x86_64-linux`. This integration remains an experimental WebAssembly workflow.

## Verification

- The locked release build passed. Generated imports and both required Rust
  exports were checked.
- ShellCheck and the source whitespace check passed.
- `pnpm test` passed through real HTTP on `just dev 5656`. It covered home and
  HTMX responses, redirects, cookie policy, timezone precedence, static file
  hashes, missing routes, single-event SSE, and an advancing continuous stream.
- The wrong-status probe returned 1 for expected HTTP 201 versus actual 200.
  A second server start on occupied port 5656 also returned 1. The original
  server continued serving the subsequent endpoint checks.
- The complete harness stopped its own server and returned 0. Port 5656 was
  free afterward. The dev child's termination status 143 is expected during
  harness cleanup.
- The deployment dry run passed: 1249.16 KiB total, 302.96 KiB compressed.
- Rust unit, clippy, and JavaScript test runners were not used under the live
  HTTP rule. Production HTTP was not exercised under the local-only rule.

## Deployment evidence

Cloudflare accepted the deployment at `2026-10-20T18:38:00.224776Z` and reported
a 36 ms Worker startup. The provider API then confirmed:

- Route: `clock.abc-p.com/*`.
- Active version: `4a855d4d-d18b-4cda-88ec-a5817b159cbb`, at 100 percent.
- Deployment: `f9568bf5-6548-42a0-b6dc-ff313ef362c5`.
- Prior version for rollback: `9ffdca91-f1c0-4a40-9673-9dc69f4f87c8`.

Generated build-input SHA-256 values:

```text
clock_wasm.js   a1bee231118d1d9a469aee8e7b59594707c636b021aec9c9f288dc3336b9e11b
clock_wasm.wasm 36ef7f84bc9d08ee927c9b6610a2f81db06389358417171cd05d8f1dbabbc32f
```

The `proj-edge-clock` Hindsight bank contains the verified migration findings.
Delegates reported public-only Jev classification; it was not counted as build
or runtime proof. Git publication is recorded in the parent Bead and Git
history. The pre-existing `.beads.gate.lock` remains outside the change.

The build command explicitly sets its target directory to the directory from
which it packages artifacts. This prevents an inherited `CARGO_TARGET_DIR`
from causing stale output to be copied. The full HTTP gate passed with that
environment override. The alternate directory was not created, port 5656 was
free after cleanup, and both generated file hashes remained equal to the
deployment inputs recorded above.
