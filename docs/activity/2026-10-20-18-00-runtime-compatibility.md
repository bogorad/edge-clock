# Local runtime compatibility

The Emscripten release artifact built with the matching LLVM 24 SDK. The first
live HTTP gate then failed before readiness. Wrangler 4.93.0 contained a runtime
whose latest supported compatibility date was 2026-05-25. It rejected the
declared 2026-08-04 date. The test command returned 1 and stopped its owned
server. Port 5656 was free afterward.

Selected Wrangler 4.148.0 from the existing pinned toolchain input. Its upstream
package catalog pins workerd 1.20261006.1. Added explicit Node compatibility for
the generated Emscripten module. A passing HTTP run remains required.

The provider deployment read succeeded. The prior active version is
`9ffdca91-f1c0-4a40-9673-9dc69f4f87c8` at 100 percent, deployment
`9e4935dc-0991-421d-bfbe-589359b24d69`. Keep that version for rollback.
No new deployment was made during this check.

Bead: `edge-clock-3oh`.

The new runtime served the Rust home and theme responses. Live inspection also
confirmed that missing static files pass through the provider's empty 404,
while normal missing routes return Rust's `Not Found` text. The test now checks
the provider-owned status without imposing an unsupported body contract.

Both failure probes returned exit 1 as required: expecting HTTP 201 from the
healthy home route, and attempting a second start on occupied port 5656.
