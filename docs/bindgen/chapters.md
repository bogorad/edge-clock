# wasm-bindgen Guide — Chapter Summaries

Source book: https://wasm-bindgen.github.io/wasm-bindgen/ (same content is also mirrored at https://rustwasm.github.io/docs/wasm-bindgen/)

---

## Introduction
`wasm-bindgen` is a Rust crate + CLI that generates the “glue” needed for rich, ergonomic interop between WebAssembly (`wasm32-unknown-unknown`) and JavaScript: strings, objects, classes, closures, and more—rather than only integers/floats.

---

## 1. Examples
### 1. Examples (index)
A curated set of runnable examples using `wasm-bindgen`, `js-sys`, and `web-sys`, mostly built with `wasm-pack` + a bundler, plus a few “no bundler” patterns.

### 1.1. Hello, World!
Shows the minimal export/import loop: export a Rust function to JS, call it from a tiny JS entry, and bundle/run it.

### 1.2. Using console.log
Demonstrates importing `console.log` into Rust via an `extern "C"` block (or using helpers) and logging from Rust.

### 1.3. Small Wasm files
Techniques for keeping output small: avoid unused features, consider size-oriented tooling, and keep the JS↔Rust boundary coarse.

### 1.4. Without a Bundler
How to load the generated JS + `.wasm` without webpack/rollup: use `--target web`-style outputs and explicit instantiation.

### 1.5. Synchronous Instantiation
A variant of “no bundler” where you instantiate the module synchronously when your environment allows it.

### 1.6. Importing functions from JS
Shows importing JS functions into Rust with `#[wasm_bindgen] extern "C"` and calling them from exported Rust code.

### 1.7. Working with char
Covers Rust `char` interop conventions (typically represented as a JS string of length 1).

### 1.8. js-sys: WebAssembly in WebAssembly
Uses `js-sys` bindings to interact with JS’s `WebAssembly.*` APIs from Rust itself.

### 1.9. web-sys: DOM hello world
Minimal DOM manipulation via `web-sys` (e.g., `document`, `Element`, `Node`).

### 1.10. web-sys: Closures
Event listeners/callbacks using `Closure<dyn FnMut(...)>` and the “keep the closure alive” rule.

### 1.11. web-sys: performance.now
Uses `web_sys::Performance` for timing and performance measurement.

### 1.12. web-sys: using fetch
Uses `window.fetch` with `web-sys` overloads and `wasm-bindgen-futures` to await a `Promise`.

### 1.13. web-sys: Weather report
A more complete `fetch` example: query a service, parse results, update DOM.

### 1.14. web-sys: canvas hello world
Basic `CanvasRenderingContext2d` drawing.

### 1.15. web-sys: canvas Julia set
Canvas pixel manipulation + number-crunching in Rust; often uses typed arrays / clamped buffers.

### 1.16. web-sys: WebAudio
Interacting with audio contexts/nodes; highlights browser differences and sometimes vendor prefixes.

### 1.17. web-sys: WebGL
Creating a WebGL context, compiling shaders, uploading buffers, drawing.

### 1.18. web-sys: WebSockets
WebSocket creation, message handlers, and data transfer patterns.

### 1.19. web-sys: WebRTC DataChannel
Peer connections + data channel messaging, usually with a chunk of JS glue.

### 1.20. web-sys: requestAnimationFrame
Animation loop patterns and borrowing/lifetime patterns for callbacks.

### 1.21. web-sys: A Simple Paint Program
A larger canvas app emphasizing input events, state handling, and redraw loops.

### 1.22. web-sys: Wasm in Web Worker
How to run wasm in a worker context (different global objects, message passing).

### 1.23. Parallel Raytracing
A larger demo: splitting work, using workers, and passing typed-array buffers efficiently.

### 1.24. Wasm Audio Worklet
Audio worklets have a constrained environment; emphasizes avoiding APIs not available there (e.g., TextEncoder/Decoder sometimes).

### 1.25. web-sys: A TODO MVC App
A full “app-sized” example: state management, DOM updates, events, and glue hygiene.

---

## 2. Reference
### 2. Reference (index)
Reference material: answers “How do I …?” questions (types, CLI flags, deployment targets), not a linear tutorial.

### 2.1. Deployment
Explains the build artifacts (`.wasm`, JS wrapper module, optional `.d.ts`) and the major JS output targets:
- `bundler` (default; ESM for bundlers), `web` (native ESM), `nodejs`, `no-modules` (global), `deno`, and an experimental Node ESM mode.

### 2.2. JS snippets
How to bundle “helper JS” alongside your Rust crate:
- `#[wasm_bindgen(module = "/js/foo.js")]` imports from an external JS module.
- `#[wasm_bindgen(inline_js = "...")]` embeds JS directly.
- `raw_module` is like `module` but avoids rewriting/normalization that some pipelines do.

### 2.3. Static JS Objects
How to bind a JS-exported *static value/object* via `static` in an extern block:
- Use `#[wasm_bindgen(thread_local_v2)] static NAME: JsValue;` and access with `NAME.with(JsValue::clone)`.
- `Option<T>` statics let you handle “may not exist” globals.
- `static_string` enables static `JsString` imports without TextEncoder/Decoder (useful in restricted contexts like audio worklets).

### 2.4. Passing Rust Closures to JS
Closures can cross the boundary, but lifetimes are the trap:
- If JS *does not store* the closure (calls it immediately), you can pass a borrowed closure.
- If JS *stores* it (event handlers, callbacks), use `Closure::wrap`/`Closure::new`, and keep it alive until JS is done with it.

### 2.5. Receiving JS Closures in Rust
Accept a `js_sys::Function` from JS and invoke it via `call0/call1/...` (Rust has no overloading, so you pick the arity).

### 2.6. Promises and Futures
Bridging async:
- Convert JS `Promise` → Rust `Future` with `wasm_bindgen_futures::JsFuture`.
- Convert Rust `Future` → JS `Promise` with `wasm_bindgen_futures::future_to_promise`.
- You can’t directly export an `async fn` as a stable ABI boundary without a shim; use the Promise conversions.

### 2.7. Iterating over JS Values
Use `js_sys::Iterator` and helpers:
- Many JS collections expose iterators (`Map::values`, `Set::keys`, …).
- `js_sys::try_iter` can turn any “iterable” value into an iterator; yielded items are `Result<JsValue>` to represent thrown exceptions.

### 2.8. Arbitrary Data with Serde
Serialize complex Rust data structures into `JsValue` with `serde-wasm-bindgen`:
- `serde_wasm_bindgen::to_value(&data)` for Rust→JS.
- `serde_wasm_bindgen::from_value(js)` for JS→Rust.
Also mentions a JSON-string approach as an alternative performance tradeoff.

### 2.9. Accessing Properties of Untyped JS Values
For ad-hoc property access on arbitrary `JsValue`, use `js_sys::Reflect::{get,set,has}`.

### 2.10. Working with Duck-Typed Interfaces
Use `#[wasm_bindgen(structural)]` to define “if it quacks” interfaces:
- Declare an extern `type` and mark methods/getters/setters as `structural` so calls become property lookups.

### 2.11. Command Line Interface
Covers `wasm-bindgen` CLI usage and key flags (notably output targets, TypeScript generation, module-splitting flags). Mentions that `wasm-pack` wraps much of this.

### 2.12. Optimizing for Size
Size strategies at a high level: make fewer boundary calls, enable size-oriented Rust settings, and apply wasm post-processing (e.g., `wasm-opt`) where appropriate.

### 2.13. Supported Rust Targets
`wasm-bindgen` primarily targets `wasm32-unknown-unknown` and JS host environments (browser, Node, Deno, workers).

### 2.14. Supported Browsers
High-level “what you need”: WebAssembly support, JS features required by the generated glue (typed arrays, modules depending on target).

### 2.15. Support for Weak References
Discusses taking advantage of JS weak references / finalization capabilities when available, to improve resource lifecycle behavior.

### 2.16. Support for Reference Types
Explains wasm “reference types” support and enabling it via target features; this affects how JS references can be represented more directly.

### 2.17. Supported Types (overview)
A map of what Rust types can cross the ABI boundary, and what they become in JS.

#### 2.17.1. Imported JavaScript Types
Use `js-sys` and `web-sys` types (e.g., `Array`, `Function`, `Map`, `Window`, `Element`, typed arrays) as “handles” to GC-managed JS values.

#### 2.17.2. Exported Rust Types
Rust structs/enums exported with `#[wasm_bindgen]` become JS classes/values with generated glue and (optionally) TypeScript declarations.

#### 2.17.3. JsValue
The universal “any JS value” carrier; use it when the type is dynamic, and then narrow via helpers (`is_string`, `as_f64`, casts, etc).

#### 2.17.4. Box<[T]> and Vec
Owned Rust buffers can be passed/returned; depending on element type, JS representation may be an `Array` or typed arrays, and copies may occur.

#### 2.17.5. *const T and *mut T
Raw pointers cross as JS numbers. This is powerful but unsafe: JS can only make sense of them by also reading wasm memory (`memory.buffer`) with typed arrays.

#### 2.17.6. NonNull<T>
Similar to raw pointers (JS numbers), with the extra guarantee that `NonNull<T>` itself is non-null; commonly paired with `Option<NonNull<T>>`.

#### 2.17.7. Numbers
Numeric primitives map to JS numeric representations; prefer using plain numbers at the boundary unless you have a strong reason otherwise.

#### 2.17.8. bool
Maps cleanly to JS `boolean`.

#### 2.17.9. char
Typically represented as a JS string (conceptually one Unicode scalar value).

#### 2.17.10. str
`&str` parameters map to JS strings via copies using `TextEncoder`/`TextDecoder`. Caveat: JS strings can contain unpaired UTF-16 surrogates that don’t roundtrip to UTF-8 cleanly—use `js_sys::JsString` if you must preserve exact JS string identity.

#### 2.17.11. String
Owned `String` parameters/returns also copy via `TextEncoder`/`TextDecoder` and support `Option<String>`.

#### 2.17.12. Number Slices
`&[T]`/`&mut [T]` numeric slices map to JS `TypedArray` *views* into wasm memory (zero-copy view semantics).

#### 2.17.13. Boxed Number Slices
`Box<[T]>` numeric slices map to JS `TypedArray` *with copies* when transferring ownership across the boundary.

#### 2.17.14. Result<T, E>
Returning `Result<T, E>` from Rust exports becomes “return T or throw” in JS. Importing a throwing JS function as `Result` requires `#[wasm_bindgen(catch)]`.

### 2.18. #[wasm_bindgen] Attributes (overview)
An exhaustive reference for tweaking imports/exports and generated glue.

#### 2.18.1. On JavaScript Imports (overview)
Attributes used inside `extern "C" { ... }` to correctly bind JS functions/classes.

- **2.18.1.1. catch**: import a JS function that may throw as `Result<_, JsValue>`.
- **2.18.1.2. constructor**: bind a JS constructor so Rust can call it as `Type::new(...)`.
- **2.18.1.3. extends**: express JS inheritance; enables generated `Deref` up the JS class chain (unless disabled).
- **2.18.1.4. getter and setter**: bind property accessors; setters must be named `set_...` by convention.
- **2.18.1.5. final**: mark an imported type as not extensible (affects generated trait impls / assumptions).
- **2.18.1.6. indexing_getter / indexing_setter / indexing_deleter**: bind dynamic `obj[prop]` operations (Proxy-like); used with `structural` + `method`.
- **2.18.1.7. js_class**: use when the Rust name differs from the JS class name (especially when renaming imports).
- **2.18.1.8. js_name**: bind a Rust identifier to a different JS identifier (camelCase, invalid identifiers, polymorphic binding by multiple Rust names).
- **2.18.1.9. js_namespace**: access namespaced globals (`console.log`, `WebAssembly.Module`, nested namespaces).
- **2.18.1.10. method**: make an import a JS method; first arg is `this: &Type`.
- **2.18.1.11. module**: import from a JS module path (`module = "/js/foo.js"`).
- **2.18.1.12. raw_module**: like `module`, but avoids rewriting/normalization.
- **2.18.1.13. no_deref**: prevent generated `Deref` impl (even if `extends` exists).
- **2.18.1.14. static_method_of**: bind static methods like `Date.now()` so Rust calls `Date::now()`.
- **2.18.1.15. structural**: duck-typing (property lookup rather than prototype method dispatch).
- **2.18.1.16. typescript_type**: declare a type whose TS shape is provided via `typescript_custom_section`.
- **2.18.1.17. variadic**: bind `...rest` style arguments using a slice as the last parameter.
- **2.18.1.18. vendor_prefix**: fall back to prefixed names (`webkitAudioContext`) when the unprefixed global doesn’t exist.

#### 2.18.2. On Rust Exports (overview)
Attributes used on Rust exports (functions, structs, impls) to influence JS API shape and TS output.

- **2.18.2.1. constructor**: export a Rust constructor so JS can call `new Foo(...)`.
- **2.18.2.2. js_name**: export a different JS name (often camelCase).
- **2.18.2.3. js_class**: attach an `impl` block’s methods to a specific JS class name.
- **2.18.2.4. readonly**: a `pub` field becomes read-only in JS (no setter; setting throws).
- **2.18.2.5. skip**: do not expose a `pub` field to JS.
- **2.18.2.6. skip_jsdoc**: stop wasm-bindgen from generating JSDoc annotations; you provide your own.
- **2.18.2.7. start**: run a function automatically when the module initializes (with some caveats).
- **2.18.2.8. main**: treat a function as the module’s main entry point in generated bindings.
- **2.18.2.9. typescript_custom_section**: append custom TS declarations into the generated `.d.ts`.
- **2.18.2.10. getter and setter**: export JS properties backed by Rust getter/setter methods.
- **2.18.2.11. inspectable**: add helpful `toJSON`/`toString` so exported classes display readable fields.
- **2.18.2.12. skip_typescript**: suppress TS generation for specific exports/fields.
- **2.18.2.13. getter_with_clone**: generate getters requiring `Clone` (not just `Copy`) for returned fields.
- **2.18.2.14. unchecked_return_type / unchecked_param_type**: relax type checking in generated bindings for specific items.
- **2.18.2.15. return_description / param_description**: control generated documentation text for params/returns.

---

## 3. web-sys
### 3. web-sys (index)
`web-sys` is “like libc for the Web”: raw, comprehensive bindings to Web APIs (DOM, fetch, WebGL, WebAudio, …). JS builtins (`Array`, `Map`, …) live in `js-sys`.

### 3.1. Using web-sys
Shows how to add `web-sys` and enable the exact cargo features for the interfaces/methods you call.

### 3.2. Cargo Features
Every `web-sys` type is behind a cargo feature; methods require features for the receiver type *and* argument types.

### 3.3. Function Overloads
Overloaded Web APIs become multiple Rust methods (e.g., `fetch_with_str`, `fetch_with_request_and_init`, …).

### 3.4. Type Translations
Notes WebIDL translation quirks:
- `BufferSource`/`ArrayBufferView` may map to either a JS object handle or a Rust slice.
- Callbacks are typically `js_sys::Function` (often produced from `Closure`).

### 3.5. Inheritance
`web_sys` types implement `Deref` and `AsRef` up the DOM inheritance chain (e.g., `Element` → `Node`).

### 3.6. Unstable APIs
Unstable WebIDL-based APIs are gated behind `cfg(web_sys_unstable_apis)` and may break outside semver guarantees.

---

## 4. Testing with wasm-bindgen-test
### 4. Testing with wasm-bindgen-test (index)
An (experimental) test harness to run Rust tests compiled to wasm in Node or browsers.

### 4.1. Usage
Add `wasm-bindgen-test` as a dev-dependency, write `#[wasm_bindgen_test]` tests, and run them via `wasm-pack test` or `cargo test --target wasm32-unknown-unknown` with the runner.

### 4.2. Writing Asynchronous Tests
Async tests work with `async fn` plus `wasm-bindgen-futures` (e.g., `JsFuture::from(promise).await`).

### 4.3. Testing in Headless Browsers
Configure environments via env vars or `wasm_bindgen_test_configure!`:
- Run in browser, dedicated/shared/service worker, Deno, or Node ESM experimental mode.

### 4.4. Continuous Integration
Example CI configurations (Travis/AppVeyor/GitHub Actions, etc.) for browser + Node testing.

### 4.5. Coverage (Experimental)
How to emit `.profraw` coverage data; requires nightly + special flags and currently has sharp edges.

---

## 5. Contributing to wasm-bindgen
### 5. Contributing (index)
How to set up the repo, run tests, and understand the internal architecture.

### 5.1. Testing
Explains the project’s major test suites (wasm tests on Node/headless browsers are common day-to-day).

### 5.2. Internal Design (index)
A deep-dive into wasm-bindgen internals (useful if you’re changing the tool itself).

#### 5.2.1. JS Objects in Rust
Explains the “polyfill”/representation strategy: JS objects are stored in a JS-side heap, passed to wasm as integer handles.

#### 5.2.2. Exporting a function to JS
Exported functions get JS shim wrappers that handle conversions, stack/heap management, and exceptions.

#### 5.2.3. Exporting a struct to JS
How exported Rust structs become JS classes with methods, constructors, and `free()` patterns.

#### 5.2.4. Importing a function from JS
How `extern "C"` imports become JS glue that routes calls to the right module/namespace.

#### 5.2.5. Importing a class from JS
How methods, constructors, and class inheritance are modeled from JS into Rust bindings.

#### 5.2.6. Rust Type conversions
Details the internal traits used to convert Rust→JS and JS→Rust values.

#### 5.2.7. Types in wasm-bindgen
How type metadata from the macro is communicated to the CLI tool for code generation.

### 5.3. js-sys (index)
`js-sys` contains raw bindings to JS standard builtins guaranteed by ECMAScript.

#### 5.3.1. Testing
How to run `js-sys` tests (generally via wasm-bindgen-test in Node).

#### 5.3.2. Adding More APIs
Guidelines for adding stage-4 ECMAScript APIs and filing issues for missing globals.

### 5.4. web-sys (contributing)
How `web-sys` is generated from WebIDL and how to extend it.

#### 5.4.1. Overview
Explains repo layout (`webidls/enabled`, `build.rs`, generated features, etc.).

#### 5.4.2. Testing
Run `web-sys` tests with `--all-features` on the wasm target.

#### 5.4.3. Logging
Enable `wasm_bindgen_webidl` logs with `RUST_LOG=wasm_bindgen_webidl cargo build -vv`.

#### 5.4.4. Supporting More Web APIs
Workflow: ensure the WebIDL is present/enabled, regenerate bindings via `wasm-bindgen-webidl`, and inspect diffs.

### 5.5. Publishing
Release checklist: bump crate versions, update changelog, run the publish script.

### 5.6. Team
Governance and contribution rules: PR review requirements and consensus for large decisions.
