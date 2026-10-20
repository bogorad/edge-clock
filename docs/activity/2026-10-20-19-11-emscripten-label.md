# Emscripten page label

Bead: `edge-clock-3oh`. Branch: `cf-wk-rust`.

Changed the subtitle and footer from `Rust/WASM` to `Rust/emscripten`.
Both `emscripten` labels link to the Cloudflare article requested by the user:
<https://blog.cloudflare.com/rust-workers-emscripten-target/>.

The page uses its existing link style. This copy change names the new build
method and adds no browser script or interaction state.

Live local HTTP validation passed through `just dev 5656`. The home page and
dark/light HTMX responses each contained both exact links. Existing theme,
cookie, timezone, static asset, missing-route, and SSE checks passed. The owned
server stopped and port 5656 was free afterward. No production HTTP or browser
JavaScript tests were run.

Cloudflare deployed the revision at `2026-10-20T19:14:43.886725Z`. Provider
readback confirmed version `34a6e3e3-6e93-4cde-9771-6bdba8edde31` at 100 percent
on `clock.abc-p.com/*`, deployment `f9f7b80f-2229-4bb8-b431-a12e8561571a`.
The prior version for rollback is `4a855d4d-d18b-4cda-88ec-a5817b159cbb`.

Beads remote sync has a separate follow-up, `edge-clock-zu6`, documented in
[the tracker note](2026-10-20-19-14-beads-sync-limitation.md).
