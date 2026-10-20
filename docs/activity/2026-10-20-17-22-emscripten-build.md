# Emscripten Worker build

Use the Rust Emscripten binary target and the generated wasm-bindgen module.
Pin Rust 1.97.0 with its Emscripten standard library, Emscripten 6.0.10,
wasm-bindgen 0.2.129, Wrangler 4.93.0, and pnpm 10.28.0. Use the official
SDK 6.0.10 LLVM 24 and Binaryen bundle with a fixed release URL and SHA256.
LLVM 22 lacks the required WASM memory builtins. The SDK targets x86_64-linux.

Keep the Rust planner, typed exports, and Worker HTTP behavior. Replace the
manual WASM instance setup with the generated module import. Use compatibility
date 2026-08-04 and the new module registry for source-phase WASM imports.

Build with explicit Bash target settings. Package the generated JS, WASM, and
required snippet directories; exclude Cargo libraries and object files. Keep
the compiler's Node, Python, and Java dependencies on the base package set.
Declare the live-test tools and deploy wrapper's YAML processor in the shell.

Update the Rust lock to matching bindgen, JS, and test crates. Keep Rust tests
as explicit opt-in commands. Remove the obsolete Node WASM runner and make the
default test command call live HTTP checks only.

The build issue is `edge-clock-3oh.2`. Build proof and runtime proof are separate.
