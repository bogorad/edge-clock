#![cfg(target_arch = "wasm32")]

use clock_wasm::{typed_handle_http, typed_render_sse};
use serde_json::Value;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn typed_handle_http_returns_object_shape_for_home() {
    let output = typed_handle_http(
        "GET",
        "/",
        Some("tz=Asia%2FTokyo".to_string()),
        None,
        None,
        None,
        0,
    );
    let plan: Value = serde_json::from_str(output.as_str()).expect("output should be valid JSON");

    assert_eq!(plan.get("route").and_then(Value::as_u64), Some(1));
    assert_eq!(plan.get("status").and_then(Value::as_u64), Some(200));
    assert_eq!(
        plan.get("content_type").and_then(Value::as_str),
        Some("text/html; charset=utf-8")
    );
    assert!(plan
        .get("body")
        .and_then(Value::as_str)
        .expect("body should be a string")
        .contains("<time id=\"clock-time\" datetime=\"09:00:00\">09.00.00</time>"));
    assert_eq!(plan.get("session_time_zone").and_then(Value::as_str), None);
    assert!(plan
        .get("set_cookies")
        .and_then(Value::as_array)
        .expect("set_cookies should be an array")
        .iter()
        .any(|cookie| {
            cookie.as_str()
                == Some("clock_tz=Asia%2fTokyo; Path=/; Max-Age=2592000; SameSite=Lax")
        }));
}

#[wasm_bindgen_test]
fn typed_render_sse_returns_object_shape_and_clock_payload() {
    let output = typed_render_sse(0, Some("Asia/Tokyo".to_string()));
    let rendered: Value =
        serde_json::from_str(output.as_str()).expect("output should be valid JSON");

    assert_eq!(rendered.get("status").and_then(Value::as_u64), Some(200));
    let body = rendered
        .get("body")
        .and_then(Value::as_str)
        .expect("body should be a string");
    assert!(body.starts_with("event: clock\ndata:"));
    assert!(body.contains("<time id=\"clock-time\" datetime=\"09:00:00\">09.00.00</time>"));
}
