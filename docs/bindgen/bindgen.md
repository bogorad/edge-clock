# bindgen.md — How LLM agents should construct wasm-bindgen code

This is an “agent playbook” for generating *correct, maintainable* Rust↔JS glue with `wasm-bindgen`, `js-sys`, and `web-sys`.

---

## 0) Mental model (don’t skip)
- Wasm itself speaks mostly integers/floats + linear memory. `wasm-bindgen` builds a higher-level ABI on top.
- “JS values” are effectively *handles* managed by generated JS glue (a JS-side heap of references). Rust sees handles; JS sees real objects.
- Most bugs come from **lifetimes** (closures, typed-array views) and **type mismatches** (wrong imports/attributes/features).

---

## 1) Choose your build + runtime target first
`wasm-bindgen` can generate different JS wrappers depending on where you’ll run them.

Common `--target` values:
- `bundler` (default): ESM intended for bundlers (webpack/rollup/vite).
- `web`: native ESM for browsers without bundling.
- `nodejs`: CommonJS-ish Node bindings.
- `no-modules`: attaches exports to a global (legacy, but sometimes useful).
- `deno`: Deno-friendly output.
- an experimental Node ESM mode exists.

**Agent rule:** Every codegen task must state the intended target(s). If it’s not specified, assume `bundler` (modern front-end tooling).

---

## 2) Project skeleton (Cargo)
Minimum for a library crate:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

# Optional but common:
js-sys = "0.3"
web-sys = { version = "0.3", features = [] } # add features you use
wasm-bindgen-futures = "0.4"                 # async/promise glue
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.4"
```

**Agent rule:** If you use any `web_sys::Type` or method, include the needed `web-sys` cargo features explicitly.

---

## 3) “Good boundary hygiene” (API design)
### Prefer coarse-grained boundaries
Crossing JS↔Rust is not free. Prefer:
- fewer calls with richer payloads (structs, arrays, serialized objects),
- compute-heavy work in Rust,
- UI/event-heavy work in JS (calling into Rust when needed).

### Prefer stable-ish boundary types
Most robust boundary types:
- numbers, `bool`
- `String`/`&str` (copying; see caveat)
- `JsValue` (dynamic) + `js-sys`/`web-sys` wrappers
- typed arrays / numeric slices when performance matters

Avoid unless necessary:
- raw pointers (*const/*mut) and manual memory interpretation
- very chatty boundaries (lots of tiny function calls)

---

## 4) Exports (Rust → JS)
### Export a function
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
```

### Export errors as exceptions (Result)
Returning `Result<T, E>` becomes “return T or throw” in JS.
```rust
#[wasm_bindgen]
pub fn parse_u32(s: &str) -> Result<u32, JsValue> {
    s.parse::<u32>()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

### Export a class (Rust struct)
```rust
#[wasm_bindgen]
pub struct Counter {
    n: u32,
}

#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Counter {
        Counter { n: 0 }
    }

    pub fn inc(&mut self) { self.n += 1; }
    pub fn value(&self) -> u32 { self.n }
}
```

**Field exposure rules**
- `pub field: T` becomes a JS property.
- `#[wasm_bindgen(readonly)] pub field: T` makes it read-only in JS.
- `#[wasm_bindgen(skip)] pub field: T` hides it from JS.
- `#[wasm_bindgen(getter)]` / `#[wasm_bindgen(setter)]` lets you define property accessors via methods.
- `#[wasm_bindgen(inspectable)]` makes debugging nicer by defining `toJSON/toString` that expose readable fields.

### Control JS names
- `#[wasm_bindgen(js_name = doTheThing)]` exports a JS-friendly name.
- `#[wasm_bindgen(js_class = Foo)]` can force an `impl` block’s methods onto a chosen JS class name (useful when renaming).

### Module initialization hooks
- `#[wasm_bindgen(start)]` runs automatically at module initialization. Use for one-time setup (panic hooks, logging, etc).
- `#[wasm_bindgen(main)]` exposes a conventional entry point in generated bindings.

**Agent rule:** Keep `start` minimal and non-fallible unless you intentionally return `Result<(), JsValue>` and want JS-visible failures.

---

## 5) Imports (JS → Rust)
The core pattern is an `extern "C"` block.

### Import from a module file (recommended for your own JS)
```rust
#[wasm_bindgen(module = "/js/helpers.js")]
extern "C" {
    fn greet(name: &str);
}
```

### Import from a namespace (console, WebAssembly, etc.)
```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
```

`js_namespace` can be nested: `js_namespace = ["window", "document"]` binds `window.document.*`.

### Import a method on a JS class
```rust
#[wasm_bindgen]
extern "C" {
    type Set;

    #[wasm_bindgen(method)]
    fn has(this: &Set, value: &JsValue) -> bool;
}
```

### Import getters/setters as methods
```rust
#[wasm_bindgen]
extern "C" {
    type TheDude;

    #[wasm_bindgen(method, getter)]
    fn white_russians(this: &TheDude) -> u32;

    // setters must be named set_...
    #[wasm_bindgen(method, setter)]
    fn set_white_russians(this: &TheDude, val: u32);
}
```

### Catch JS exceptions in Rust
If a JS import can throw, use `catch` and return `Result`:
```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = JSON)]
    fn parse(s: &str) -> Result<JsValue, JsValue>;
}
```

---

## 6) Strings: correctness vs speed
`&str` and `String` copy through `TextEncoder`/`TextDecoder`.

Gotcha: JS strings can contain unpaired UTF-16 surrogates, which don’t roundtrip to UTF-8 cleanly (they get replaced). If you *must* preserve the exact JS string, use `js_sys::JsString` handles instead of copying.

**Agent rule:** Default to `String`/`&str` unless the caller explicitly requires exact JS string identity.

---

## 7) Buffers and typed arrays (performance)
### Zero-copy views: numeric slices
Numeric `&[T]` / `&mut [T]` can be seen as JS `TypedArray` *views into wasm memory*.
This is fast but has rules:
- The view is tied to wasm linear memory; if memory grows, old views can detach/become invalid.
- Lifetime: don’t let JS retain a view after Rust considers the borrow ended.

### Owned transfer: boxed numeric slices
`Box<[T]>` numeric slices become JS `TypedArray` with copies in/out. Safer ownership semantics; slower.

**Agent rule:** If JS needs to keep the data long-term, prefer ownership-transfer patterns (copy or explicit memory management). If it’s a tight loop and JS won’t hold onto it, use slice views.

---

## 8) Closures and callbacks (the #1 footgun)
### Passing Rust closures to JS
- If JS only calls the closure immediately and does not store it, a borrowed closure can work.
- If JS stores it (event listeners, timers, etc.), wrap it in `Closure` and keep it alive.

Canonical pattern for event listeners:
```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub fn install_click_handler(button: web_sys::HtmlElement) {
    let cb = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::MouseEvent| {
        // ...
    });

    button
        .add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())
        .unwrap();

    // IMPORTANT: keep it alive. Store it somewhere, or leak intentionally.
    cb.forget();
}
```

**Agent rule:** Never create a `Closure` and let it drop while JS still holds a reference. Either:
- store it (in a struct field, `thread_local!`, etc.), or
- explicitly `.forget()` with a comment explaining the leak/lifecycle.

### Receiving JS callbacks in Rust
Accept `&js_sys::Function` and call `call0/call1/...` with `this` (often `null`).

---

## 9) Async (Promises ↔ Futures)
Convert:
- `Promise` → `Future`: `JsFuture::from(promise).await`
- `Future` → `Promise`: `future_to_promise(async move { ... })`

Pattern: export a function that returns a `Promise`:
```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
pub fn fetch_something(url: String) -> js_sys::Promise {
    future_to_promise(async move {
        // ... await JsFuture ...
        Ok(JsValue::from_str("done"))
    })
}
```

**Agent rule:** Don’t try to export “raw Rust futures” directly. Always use the conversion shims.

---

## 10) Dynamic JS: Reflect + duck typing
### Reflect for ad-hoc property access
Use `js_sys::Reflect::get/set/has` to read/write arbitrary properties on unknown values.

### Duck-typed interfaces
If many different objects share “shape” but not prototype chain, define an extern `type` and use `#[wasm_bindgen(structural, method)]`.

**Agent rule:** Prefer duck-typed extern interfaces over lots of `Reflect` if you call the same property/method repeatedly.

---

## 11) Iteration over JS collections
Use `js_sys::Iterator` (and `try_iter`) to iterate JS iterables. Yields `Result<JsValue>` so exceptions can propagate.

---

## 12) TypeScript generation knobs
- wasm-bindgen can emit `.d.ts`.
- `#[wasm_bindgen(typescript_custom_section)]` appends custom TS declarations.
- `#[wasm_bindgen(typescript_type = "...")]` lets Rust refer to a TS-declared type.
- `#[wasm_bindgen(skip_typescript)]` disables TS output per item.

**Agent rule:** If your API is meant for TS consumers, design the exported surface with TS in mind (camelCase, classes, clear types).

---

## 13) web-sys usage rules
- Every interface is feature-gated. Enable features for:
  - the receiver type (`Window`, `Document`, `WebGlRenderingContext`, …)
  - each argument type used by the method
- Overloads are separate Rust methods (choose the correct one).

**Agent rule:** When generating `web-sys` code, also generate the `Cargo.toml` features list.

---

## 14) Testing rules (wasm-bindgen-test)
- Add `wasm-bindgen-test` under `[dev-dependencies]`.
- Tests must live at crate root or in `pub mod` (not inside private modules).
- Use `wasm-pack test --node/--chrome/--firefox/...` or environment variables to select runner context.
- Async tests: `#[wasm_bindgen_test] async fn ... { ... }`.

---

## 15) Final “agent checklist” before emitting code
1. Target decided (`bundler` / `web` / `nodejs` / `no-modules` / `deno`).
2. `Cargo.toml` includes **all** required deps and `web-sys` features.
3. Every JS import has correct attributes (`module`, `js_namespace`, `method`, `getter/setter`, `catch`, etc.).
4. Boundary types are supported; `Result` is used where exceptions are expected.
5. Closures: lifetime is handled (stored or `.forget()` with justification).
6. Async: uses `JsFuture`/`future_to_promise` shims.
7. Any typed-array views are not retained across invalid lifetimes.
8. If TS output matters: `js_name`, `typescript_custom_section`, and `skip_typescript` are applied deliberately.
