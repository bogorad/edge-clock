import * as wasmModule from "../rust/clock-wasm/pkg/clock_wasm_bg.wasm";
import * as bindgen from "../rust/clock-wasm/pkg/clock_wasm_bg.js";

function isWasmExports(value) {
  return (
    Boolean(value) &&
    typeof value.__wbindgen_malloc === "function" &&
    typeof value.__wbindgen_realloc === "function"
  );
}

function resolveWasmExports(moduleLike) {
  const directExports = [
    moduleLike,
    moduleLike?.default,
    moduleLike?.instance?.exports,
    moduleLike?.default?.instance?.exports,
  ].find(isWasmExports);

  if (directExports) {
    return directExports;
  }

  const compiledModule = [
    moduleLike,
    moduleLike?.default,
    moduleLike?.module,
  ].find((value) => value instanceof WebAssembly.Module);

  if (!compiledModule) {
    throw new TypeError("Unable to initialize wasm module exports");
  }

  const importModules = new Set(
    WebAssembly.Module.imports(compiledModule).map((entry) => entry.module),
  );
  const importObject = {};
  for (const moduleName of importModules) {
    importObject[moduleName] = bindgen;
  }

  const { exports } = new WebAssembly.Instance(compiledModule, importObject);

  if (!isWasmExports(exports)) {
    throw new TypeError("Wasm exports missing bindgen allocation symbols");
  }

  return exports;
}

bindgen.__wbg_set_wasm(resolveWasmExports(wasmModule));
if (typeof bindgen.__wbindgen_init_externref_table === "function") {
  bindgen.__wbindgen_init_externref_table();
}

const { typed_handle_http, typed_render_sse } = bindgen;

function parseJson(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function isNullableString(value) {
  return value === null || typeof value === "string";
}

function isRenderPayload(value) {
  return (
    Boolean(value) &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    Number.isInteger(value.status) &&
    value.status >= 100 &&
    value.status <= 599 &&
    typeof value.body === "string"
  );
}

function isHttpPlan(value) {
  return (
    Boolean(value) &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    Number.isInteger(value.route) &&
    value.route >= 0 &&
    Number.isInteger(value.status) &&
    value.status >= 0 &&
    value.status <= 599 &&
    typeof value.body === "string" &&
    isNullableString(value.content_type) &&
    isNullableString(value.location) &&
    Array.isArray(value.set_cookies) &&
    value.set_cookies.every((cookie) => typeof cookie === "string") &&
    isNullableString(value.session_time_zone) &&
    typeof value.once === "boolean"
  );
}

// SSE emits Uint8Array chunks through ReadableStream controllers. Encoding
// happens in Worker JS because stream controllers are host runtime APIs.
const encoder = new TextEncoder();

// Route ids are planned in Rust. Worker keeps these constants only so the
// host can branch into runtime-only behavior (static assets and SSE transport).
const ROUTE_CLOCK_STREAM = 2;
const ROUTE_STATIC_ASSET = 3;

/**
 * Build a Worker Response from a Rust HTTP plan.
 *
 * Why this stays in JS:
 * - Rust decides status/body/headers/cookies, but only Worker can construct the
 *   actual Response object and append platform headers.
 */
function responseFromPlan(plan) {
  const headers = new Headers();

  // Rust already selects content type when needed; Worker just applies it.
  if (plan.content_type) {
    headers.set("content-type", plan.content_type);
  }

  // Redirect target is also planned in Rust for non-HTMX theme flow.
  if (plan.location) {
    headers.set("location", plan.location);
  }

  // Rust now formats complete Set-Cookie header values. Worker only emits them.
  for (const cookie of plan.set_cookies ?? []) {
    headers.append("set-cookie", cookie);
  }

  return new Response(plan.body || null, {
    status: plan.status,
    headers,
  });
}

/**
 * Host-managed SSE transport.
 *
 * Why this stays in JS:
 * - Stream lifecycle, timers, abort handling, and controller enqueue/close are
 *   Worker runtime concerns and cannot be performed directly in Rust/WASM.
 * - Rust still owns payload rendering via typed_render_sse.
 */
function createSseResponse({ once, signal, timeZone, setCookies }) {
  let intervalId = null;

  const stream = new ReadableStream({
    start(controller) {
      const writeEvent = () => {
        // Use host time source for cadence; Rust converts to timezone and formats
        // the payload content.
        const unixSeconds = Math.max(0, Math.floor(Date.now() / 1000));
        const rendered = parseJson(typed_render_sse(unixSeconds, timeZone ?? ""));
        if (!isRenderPayload(rendered)) {
          controller.enqueue(encoder.encode("event: error\ndata: invalid sse payload\n\n"));
          return;
        }
        controller.enqueue(encoder.encode(rendered.body));
      };

      const cleanup = () => {
        if (intervalId !== null) {
          clearInterval(intervalId);
          intervalId = null;
        }
      };

      // Emit immediately so clients get first event on connect.
      writeEvent();

      // once=1 behavior is planned in Rust; host enforces stream close.
      if (once) {
        controller.close();
        return;
      }

      // Keep live stream cadence at one event per second.
      intervalId = setInterval(writeEvent, 1000);

      // Request abort must terminate stream/timer from the host side.
      if (signal) {
        if (signal.aborted) {
          cleanup();
          controller.close();
          return;
        }

        signal.addEventListener(
          "abort",
          () => {
            cleanup();
            controller.close();
          },
          { once: true },
        );
      }
    },
    cancel() {
      // Client disconnect cleanup.
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    },
  });

  const headers = new Headers({
    "content-type": "text/event-stream; charset=utf-8",
    "cache-control": "no-cache, no-transform",
    connection: "keep-alive",
  });

  // Rust provides cookie directives; host emits them on SSE handshake response.
  for (const cookie of setCookies ?? []) {
    headers.append("set-cookie", cookie);
  }

  return new Response(stream, {
    status: 200,
    headers,
  });
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const unixSeconds = Math.max(0, Math.floor(Date.now() / 1000));

    // Single Rust planner call per request.
    // Rust resolves context, route, status/body, redirect location, content type,
    // cookie directives, session timezone, and once mode.
    const plan = parseJson(
      typed_handle_http(
        request.method,
        url.pathname,
        url.search.startsWith("?") ? url.search.slice(1) : url.search,
        request.headers.get("cookie"),
        request.headers.get("x-timezone"),
        request.headers.get("hx-request"),
        unixSeconds,
      ),
    );

    if (!isHttpPlan(plan)) {
      return new Response("Invalid planner payload", {
        status: 500,
        headers: {
          "content-type": "text/plain; charset=utf-8",
        },
      });
    }

    // Static assets are host-bound (Cloudflare ASSETS binding). Rust can decide
    // the route but cannot execute env.ASSETS.fetch itself.
    if (plan.route === ROUTE_STATIC_ASSET && env.ASSETS) {
      const assetUrl = new URL(request.url);
      assetUrl.pathname = assetUrl.pathname.replace(/^\/static/, "");
      const assetRequest = new Request(assetUrl.toString(), request);
      return env.ASSETS.fetch(assetRequest);
    }

    // If static route exists but ASSETS binding is unavailable, host returns the
    // equivalent not-found result for static lookup.
    if (plan.route === ROUTE_STATIC_ASSET) {
      return new Response("Not Found", {
        status: 404,
        headers: {
          "content-type": "text/plain; charset=utf-8",
        },
      });
    }

    // SSE route remains host transport logic, with Rust payload generation.
    if (plan.route === ROUTE_CLOCK_STREAM) {
      return createSseResponse({
        once: plan.once,
        signal: request.signal,
        timeZone: plan.session_time_zone,
        setCookies: plan.set_cookies,
      });
    }

    // Preserve existing fallback behavior: if planned response is 404, let ASSETS
    // try to satisfy the request before returning Rust-planned not-found.
    if (plan.status === 404 && env.ASSETS) {
      const assetResponse = await env.ASSETS.fetch(request);
      if (assetResponse.status !== 404) {
        return assetResponse;
      }
    }

    // All non-stream, non-static responses are emitted from Rust plan fields.
    return responseFromPlan(plan);
  },
};
