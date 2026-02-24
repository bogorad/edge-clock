use chrono::{DateTime, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use cookie::time::Duration;
use cookie::{Cookie, SameSite};
use handlebars::Handlebars;
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use serde::Serialize;
use url::form_urlencoded;
use wasm_bindgen::prelude::*;

const INVALID_RESULT: u64 = 0;

const ROUTE_NOT_FOUND: u32 = 0;
const ROUTE_HOME: u32 = 1;
const ROUTE_CLOCK_STREAM: u32 = 2;
const ROUTE_STATIC_ASSET: u32 = 3;
const ROUTE_THEME: u32 = 4;
const TIME_ZONE_MAX_LEN: usize = 100;
const COOKIE_MAX_AGE_SECONDS: u32 = 30 * 24 * 60 * 60;
const TIMEZONE_COOKIE_NAME: &str = "clock_tz";
const THEME_COOKIE_NAME: &str = "clock_theme";
const THEME_LIGHT: &str = "light";
const THEME_DARK: &str = "dark";
const HOME_TEMPLATE: &str = include_str!("../../../public/index.html");
const REQUEST_CONTEXT_ERROR_JSON: &str = r#"{"session_time_zone":null,"supplied_time_zone":null,"session_theme":"light","requested_theme":null,"should_set_time_zone_cookie":false,"should_set_theme_cookie":false,"once":false,"hx":false}"#;
const HTTP_PLAN_ERROR_JSON: &str = r#"{"route":0,"status":500,"body":"serialization error","content_type":"text/plain; charset=utf-8","location":null,"set_cookies":[],"session_time_zone":null,"once":false}"#;
const URI_COMPONENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

#[derive(Serialize)]
struct HomeTemplateContext<'a> {
    theme: &'a str,
    theme_toggle_target: &'a str,
    theme_toggle_label: &'a str,
    clock_element: &'a str,
}

#[derive(Serialize)]
struct RequestContextValues {
    session_time_zone: Option<String>,
    supplied_time_zone: Option<String>,
    session_theme: String,
    requested_theme: Option<String>,
    should_set_time_zone_cookie: bool,
    should_set_theme_cookie: bool,
    once: bool,
    hx: bool,
}

#[derive(Serialize)]
struct RenderResponse {
    status: u16,
    body: String,
}

#[derive(Serialize)]
struct HttpResponsePlan {
    route: u32,
    status: u16,
    body: String,
    content_type: Option<String>,
    location: Option<String>,
    set_cookies: Vec<String>,
    session_time_zone: Option<String>,
    once: bool,
}

thread_local! {
    static HOME_RENDERER: Handlebars<'static> = {
        let mut renderer = Handlebars::new();
        renderer
            .register_template_string("home", HOME_TEMPLATE)
            .expect("home template should compile");
        renderer
    };
}

pub fn format_clock_bytes(hour: u32, minute: u32, second: u32) -> Result<[u8; 8], &'static str> {
    let clock_text = format_clock_string(hour, minute, second)?;
    let clock_bytes = clock_text.as_bytes();
    if clock_bytes.len() != 8 {
        return Err("clock text should be fixed width");
    }

    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(clock_bytes);
    Ok(bytes)
}

fn format_clock_string(hour: u32, minute: u32, second: u32) -> Result<String, &'static str> {
    let clock_time = NaiveTime::from_hms_opt(hour, minute, second).ok_or("value out of range")?;
    Ok(clock_time.format("%H.%M.%S").to_string())
}

fn parse_time_zone_candidate(time_zone_value: &str) -> Option<Tz> {
    let trimmed = time_zone_value.trim();
    if trimmed.is_empty() || trimmed.len() > TIME_ZONE_MAX_LEN {
        return None;
    }

    trimmed.parse::<Tz>().ok()
}

fn normalize_time_zone_value(time_zone_value: &str) -> Option<String> {
    parse_time_zone_candidate(time_zone_value).map(|time_zone| time_zone.to_string())
}

#[cfg(test)]
fn resolve_time_zone_context_values(
    query_time_zone_value: Option<&str>,
    header_time_zone_value: Option<&str>,
    cookie_time_zone_value: Option<&str>,
) -> (Option<String>, Option<String>, bool) {
    let normalized_query = query_time_zone_value.and_then(normalize_time_zone_value);
    let normalized_header = header_time_zone_value.and_then(normalize_time_zone_value);
    let normalized_cookie = cookie_time_zone_value.and_then(normalize_time_zone_value);

    let supplied_time_zone = normalized_query.or(normalized_header);
    let session_time_zone = supplied_time_zone.clone().or(normalized_cookie.clone());

    let should_set_cookie = match (supplied_time_zone.as_deref(), normalized_cookie.as_deref()) {
        (Some(supplied), Some(cookie)) => supplied != cookie,
        (Some(_), None) => true,
        _ => false,
    };

    (session_time_zone, supplied_time_zone, should_set_cookie)
}

fn parse_time_zone(time_zone_value: &str) -> Tz {
    parse_time_zone_candidate(time_zone_value).unwrap_or(chrono_tz::UTC)
}

fn clock_text_at_unix_seconds(unix_seconds: u32, time_zone_value: &str) -> Result<String, &'static str> {
    let utc_time = DateTime::<Utc>::from_timestamp(unix_seconds as i64, 0)
        .ok_or("value out of range")?;
    let time_zone = parse_time_zone(time_zone_value);
    let local_time = utc_time.with_timezone(&time_zone);
    format_clock_string(local_time.hour(), local_time.minute(), local_time.second())
}

fn pack_bytes(bytes: [u8; 8]) -> u64 {
    bytes
        .iter()
        .fold(0_u64, |acc, byte| (acc << 8) | u64::from(*byte))
}

pub fn unpack_bytes(packed: u64) -> [u8; 8] {
    let mut bytes = [0_u8; 8];
    let mut value = packed;
    for index in (0..8).rev() {
        bytes[index] = (value & 0xff) as u8;
        value >>= 8;
    }

    bytes
}

fn render_clock_element(clock_text: &str) -> String {
    let datetime = clock_text.replace('.', ":");
    format!(
        "<time id=\"clock-time\" datetime=\"{datetime}\">{clock_text}</time>",
    )
}

fn normalize_theme_value(theme_value: &str) -> &'static str {
    if theme_value.eq_ignore_ascii_case(THEME_DARK) {
        THEME_DARK
    } else {
        THEME_LIGHT
    }
}

fn next_theme_value(theme_value: &str) -> &'static str {
    if theme_value == THEME_DARK {
        THEME_LIGHT
    } else {
        THEME_DARK
    }
}

fn next_theme_label(theme_value: &str) -> &'static str {
    if theme_value == THEME_DARK {
        "Switch to light mode"
    } else {
        "Switch to dark mode"
    }
}

fn render_home_template(context: &HomeTemplateContext<'_>) -> Result<String, &'static str> {
    HOME_RENDERER.with(|renderer| {
        renderer
            .render("home", context)
            .map_err(|_| "home template rendering failed")
    })
}

fn render_home_html_text(clock_text: &str, theme_value: &str) -> Result<String, &'static str> {
    let clock_element = render_clock_element(clock_text);
    let normalized_theme = normalize_theme_value(theme_value);
    let context = HomeTemplateContext {
        theme: normalized_theme,
        theme_toggle_target: next_theme_value(normalized_theme),
        theme_toggle_label: next_theme_label(normalized_theme),
        clock_element: &clock_element,
    };

    render_home_template(&context)
}

fn render_clock_sse_event_text(clock_text: &str) -> String {
    format!(
        "event: clock\ndata: {}\n\n",
        render_clock_element(clock_text)
    )
}

fn write_home_html_response(clock_text: &str, theme_value: &str) -> RenderResponse {
    match render_home_html_text(clock_text, theme_value) {
        Ok(home_html) => RenderResponse {
            status: 200,
            body: home_html,
        },
        Err(_) => RenderResponse {
            status: 500,
            body: "Home template renderer unavailable. Check Rust/WASM build output.".to_string(),
        },
    }
}

#[cfg(test)]
fn render_home_html_response(hour: u32, minute: u32, second: u32, theme_value: &str) -> RenderResponse {
    match format_clock_string(hour, minute, second) {
        Ok(clock_text) => write_home_html_response(&clock_text, theme_value),
        Err(_) => RenderResponse {
            status: 500,
            body: "Clock formatter unavailable. Check Rust/WASM build output.".to_string(),
        },
    }
}

fn render_home_html_at_unix_seconds_response(
    unix_seconds: u32,
    time_zone_value: &str,
    theme_value: &str,
) -> RenderResponse {
    match clock_text_at_unix_seconds(unix_seconds, time_zone_value) {
        Ok(clock_text) => write_home_html_response(&clock_text, theme_value),
        Err(_) => RenderResponse {
            status: 500,
            body: "Clock formatter unavailable. Check Rust/WASM build output.".to_string(),
        },
    }
}

fn write_clock_sse_event_response(clock_text: &str) -> RenderResponse {
    RenderResponse {
        status: 200,
        body: render_clock_sse_event_text(clock_text),
    }
}

#[cfg(test)]
fn render_clock_sse_event_response(hour: u32, minute: u32, second: u32) -> RenderResponse {
    match format_clock_string(hour, minute, second) {
        Ok(clock_text) => write_clock_sse_event_response(&clock_text),
        Err(_) => RenderResponse {
            status: 500,
            body: "event: error\ndata: clock formatter unavailable\n\n".to_string(),
        },
    }
}

fn render_clock_sse_event_at_unix_seconds_response(
    unix_seconds: u32,
    time_zone_value: &str,
) -> RenderResponse {
    match clock_text_at_unix_seconds(unix_seconds, time_zone_value) {
        Ok(clock_text) => write_clock_sse_event_response(&clock_text),
        Err(_) => RenderResponse {
            status: 500,
            body: "event: error\ndata: clock formatter unavailable\n\n".to_string(),
        },
    }
}

fn route_from(method: &[u8], path: &[u8]) -> u32 {
    match (method, path) {
        (b"GET", b"/") => ROUTE_HOME,
        (b"GET", b"/clock-stream") => ROUTE_CLOCK_STREAM,
        (b"POST", b"/theme") => ROUTE_THEME,
        (b"GET", static_path) if static_path.starts_with(b"/static/") => ROUTE_STATIC_ASSET,
        _ => ROUTE_NOT_FOUND,
    }
}

fn parse_query_param_values(query: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut time_zone = None;
    let mut theme = None;
    let mut once = None;

    for entry in query.split('&') {
        if entry.is_empty() {
            continue;
        }

        let (key, raw_value) = match entry.split_once('=') {
            Some((raw_key, raw_value)) => (raw_key, raw_value),
            None => (entry, ""),
        };

        let decoded_value = decode_query_value(raw_value);

        match key {
            "tz" => time_zone = decoded_value,
            "theme" => theme = decoded_value,
            "once" => once = decoded_value,
            _ => {}
        }
    }

    (time_zone, theme, once)
}

fn has_invalid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len()
            || !bytes[index + 1].is_ascii_hexdigit()
            || !bytes[index + 2].is_ascii_hexdigit()
        {
            return true;
        }

        index += 3;
    }

    false
}

fn decode_query_value(raw_value: &str) -> Option<String> {
    if has_invalid_percent_encoding(raw_value) {
        return None;
    }

    let mut encoded_value = String::with_capacity(raw_value.len() + 2);
    encoded_value.push_str("v=");
    for character in raw_value.chars() {
        if character == '+' {
            encoded_value.push_str("%2B");
        } else {
            encoded_value.push(character);
        }
    }

    let value_part = &encoded_value[2..];
    if percent_decode_str(value_part).decode_utf8().is_err() {
        return None;
    }

    form_urlencoded::parse(encoded_value.as_bytes())
        .next()
        .map(|(_, value)| value.into_owned())
}

fn decode_percent_component(value: &str) -> Option<String> {
    if has_invalid_percent_encoding(value) {
        return None;
    }

    percent_decode_str(value)
        .decode_utf8()
        .ok()
        .map(|decoded| decoded.into_owned())
}

fn parse_cookie_values(cookie_header: &str) -> (Option<String>, Option<String>) {
    let mut time_zone = None;
    let mut theme = None;

    for parsed_cookie in Cookie::split_parse(cookie_header) {
        let cookie = match parsed_cookie {
            Ok(cookie) => cookie,
            Err(_) => continue,
        };

        match cookie.name() {
            TIMEZONE_COOKIE_NAME => {
                time_zone = decode_percent_component(cookie.value());
            }
            THEME_COOKIE_NAME => {
                theme = decode_percent_component(cookie.value());
            }
            _ => {}
        }
    }

    (time_zone, theme)
}

fn resolve_request_context_values(
    query: &str,
    cookie: &str,
    header_time_zone: Option<&str>,
    hx_request: Option<&str>,
) -> RequestContextValues {
    let (query_time_zone, query_theme, query_once) = parse_query_param_values(query);
    let (cookie_time_zone, cookie_theme) = parse_cookie_values(cookie);

    let normalized_query_time_zone = query_time_zone.as_deref().and_then(normalize_time_zone_value);
    let normalized_header_time_zone = header_time_zone.and_then(normalize_time_zone_value);
    let normalized_cookie_time_zone = cookie_time_zone.as_deref().and_then(normalize_time_zone_value);

    let supplied_time_zone = normalized_query_time_zone.or(normalized_header_time_zone);
    let session_time_zone = supplied_time_zone.clone().or(normalized_cookie_time_zone.clone());

    let should_set_time_zone_cookie = match (
        supplied_time_zone.as_deref(),
        normalized_cookie_time_zone.as_deref(),
    ) {
        (Some(supplied), Some(cookie_value)) => supplied != cookie_value,
        (Some(_), None) => true,
        _ => false,
    };

    let requested_theme = query_theme.as_deref().map(normalize_theme_value);
    let normalized_cookie_theme = cookie_theme.as_deref().map(normalize_theme_value);
    let session_theme = requested_theme
        .or(normalized_cookie_theme)
        .unwrap_or(THEME_LIGHT)
        .to_string();

    let should_set_theme_cookie = match (requested_theme, normalized_cookie_theme) {
        (Some(requested), Some(cookie_value)) => requested != cookie_value,
        (Some(_), None) => true,
        _ => false,
    };

    RequestContextValues {
        session_time_zone,
        supplied_time_zone,
        session_theme,
        requested_theme: requested_theme.map(str::to_string),
        should_set_time_zone_cookie,
        should_set_theme_cookie,
        once: query_once.as_deref() == Some("1"),
        hx: hx_request == Some("true"),
    }
}

fn encode_uri_component(value: &str) -> String {
    utf8_percent_encode(value, URI_COMPONENT_ENCODE_SET).to_string()
}

fn format_cookie(name: &str, value: &str) -> String {
    let mut cookie = Cookie::new(name.to_string(), encode_uri_component(value));
    cookie.set_path("/");
    cookie.set_max_age(Duration::seconds(i64::from(COOKIE_MAX_AGE_SECONDS)));
    cookie.set_same_site(SameSite::Lax);

    format!(
        "{}={}; Path=/; Max-Age={}; SameSite=Lax",
        cookie.name(),
        cookie.value(),
        COOKIE_MAX_AGE_SECONDS
    )
}

fn format_time_zone_cookie(time_zone: &str) -> String {
    format_cookie(TIMEZONE_COOKIE_NAME, time_zone)
}

fn format_theme_cookie(theme: &str) -> String {
    format_cookie(THEME_COOKIE_NAME, theme)
}

fn finalize_cookie_directives(
    context: &RequestContextValues,
    include_time_zone_cookie: bool,
    include_theme_cookie: bool,
) -> Vec<String> {
    let mut cookies = Vec::new();

    if include_time_zone_cookie
        && context.should_set_time_zone_cookie
        && context.supplied_time_zone.is_some()
    {
        cookies.push(format_time_zone_cookie(
            context.supplied_time_zone.as_deref().unwrap_or_default(),
        ));
    }

    if include_theme_cookie
        && context.should_set_theme_cookie
        && context.requested_theme.is_some()
    {
        cookies.push(format_theme_cookie(
            context.requested_theme.as_deref().unwrap_or_default(),
        ));
    }

    cookies
}

fn empty_route_plan(route: u32) -> HttpResponsePlan {
    HttpResponsePlan {
        route,
        status: 0,
        body: String::new(),
        content_type: None,
        location: None,
        set_cookies: Vec::new(),
        session_time_zone: None,
        once: false,
    }
}

fn build_http_response_plan(
    method: &str,
    path: &str,
    query: &str,
    cookie: &str,
    header_time_zone: Option<&str>,
    hx_request: Option<&str>,
    unix_seconds: u32,
) -> HttpResponsePlan {
    let route = route_from(method.as_bytes(), path.as_bytes());
    let context = resolve_request_context_values(query, cookie, header_time_zone, hx_request);

    match route {
        ROUTE_STATIC_ASSET => empty_route_plan(route),
        ROUTE_CLOCK_STREAM => HttpResponsePlan {
            route,
            session_time_zone: context.session_time_zone.clone(),
            once: context.once,
            set_cookies: finalize_cookie_directives(&context, true, false),
            ..empty_route_plan(route)
        },
        ROUTE_HOME => {
            let rendered = render_home_html_at_unix_seconds_response(
                unix_seconds,
                context.session_time_zone.as_deref().unwrap_or("UTC"),
                context.session_theme.as_str(),
            );
            HttpResponsePlan {
                route,
                status: rendered.status,
                body: rendered.body,
                content_type: Some("text/html; charset=utf-8".to_string()),
                set_cookies: finalize_cookie_directives(&context, true, false),
                ..empty_route_plan(route)
            }
        }
        ROUTE_THEME => {
            if context.hx {
                let rendered = render_home_html_at_unix_seconds_response(
                    unix_seconds,
                    context.session_time_zone.as_deref().unwrap_or("UTC"),
                    context.session_theme.as_str(),
                );

                HttpResponsePlan {
                    route,
                    status: rendered.status,
                    body: rendered.body,
                    content_type: Some("text/html; charset=utf-8".to_string()),
                    set_cookies: finalize_cookie_directives(&context, true, true),
                    ..empty_route_plan(route)
                }
            } else {
                HttpResponsePlan {
                    route,
                    status: 303,
                    location: Some("/".to_string()),
                    set_cookies: finalize_cookie_directives(&context, false, true),
                    ..empty_route_plan(route)
                }
            }
        }
        _ => {
            let rendered = render_not_found_response();
            HttpResponsePlan {
                route,
                status: rendered.status,
                body: rendered.body,
                content_type: Some("text/plain; charset=utf-8".to_string()),
                set_cookies: finalize_cookie_directives(&context, true, false),
                ..empty_route_plan(route)
            }
        }
    }
}


fn render_not_found_response() -> RenderResponse {
    RenderResponse {
        status: 404,
        body: "Not Found".to_string(),
    }
}

pub fn format_time_packed(hour: u32, minute: u32, second: u32) -> u64 {
    match format_clock_bytes(hour, minute, second) {
        Ok(bytes) => pack_bytes(bytes),
        Err(_) => INVALID_RESULT,
    }
}

fn render_response_json(response: RenderResponse) -> String {
    serde_json::to_string(&response)
        .unwrap_or_else(|_| "{\"status\":500,\"body\":\"serialization error\"}".to_string())
}

fn to_json_string<T: Serialize>(value: &T, fallback_json: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback_json.to_string())
}

#[wasm_bindgen]
pub fn typed_route_id(method: &str, path: &str) -> u32 {
    route_from(method.as_bytes(), path.as_bytes())
}

#[wasm_bindgen]
pub fn typed_resolve_request_context(
    query: Option<String>,
    cookie: Option<String>,
    header_time_zone: Option<String>,
    hx_request: Option<String>,
) -> String {
    let query_text = query.unwrap_or_default();
    let cookie_text = cookie.unwrap_or_default();
    let values = resolve_request_context_values(
        query_text.as_str(),
        cookie_text.as_str(),
        header_time_zone.as_deref(),
        hx_request.as_deref(),
    );

    to_json_string(&values, REQUEST_CONTEXT_ERROR_JSON)
}

#[wasm_bindgen]
pub fn typed_render_home(unix_seconds: u32, time_zone: Option<String>, theme: Option<String>) -> String {
    let response = render_home_html_at_unix_seconds_response(
        unix_seconds,
        time_zone.as_deref().unwrap_or("UTC"),
        theme.as_deref().unwrap_or(THEME_LIGHT),
    );

    render_response_json(response)
}

#[wasm_bindgen]
pub fn typed_render_sse(unix_seconds: u32, time_zone: Option<String>) -> String {
    let response = render_clock_sse_event_at_unix_seconds_response(
        unix_seconds,
        time_zone.as_deref().unwrap_or("UTC"),
    );

    render_response_json(response)
}

#[wasm_bindgen]
pub fn typed_render_not_found() -> String {
    render_response_json(render_not_found_response())
}

#[wasm_bindgen]
pub fn typed_handle_http(
    method: &str,
    path: &str,
    query: Option<String>,
    cookie: Option<String>,
    header_time_zone: Option<String>,
    hx_request: Option<String>,
    unix_seconds: u32,
) -> String {
    let plan = build_http_response_plan(
        method,
        path,
        query.as_deref().unwrap_or_default(),
        cookie.as_deref().unwrap_or_default(),
        header_time_zone.as_deref(),
        hx_request.as_deref(),
        unix_seconds,
    );

    to_json_string(&plan, HTTP_PLAN_ERROR_JSON)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_percent_component, decode_query_value, format_clock_bytes, format_time_packed,
        normalize_time_zone_value, parse_query_param_values,
        build_http_response_plan,
        render_clock_sse_event_at_unix_seconds_response, render_clock_sse_event_response,
        render_home_html_at_unix_seconds_response, render_home_html_response, render_not_found_response,
        resolve_request_context_values, resolve_time_zone_context_values, route_from, unpack_bytes,
        ROUTE_CLOCK_STREAM, ROUTE_HOME, ROUTE_NOT_FOUND, ROUTE_STATIC_ASSET, ROUTE_THEME,
        THEME_LIGHT,
    };

    fn unpack_to_string(packed: u64) -> String {
        let bytes = unpack_bytes(packed);
        String::from_utf8(bytes.to_vec()).expect("clock bytes are valid ASCII")
    }

    #[test]
    fn formats_single_digits_with_zero_padding() {
        let formatted = format_clock_bytes(9, 5, 7).expect("valid time should format");
        assert_eq!(&formatted, b"09.05.07");
    }

    #[test]
    fn formats_boundary_values() {
        let midnight = format_clock_bytes(0, 0, 0).expect("midnight should format");
        let max_time = format_clock_bytes(23, 59, 59).expect("max time should format");

        assert_eq!(&midnight, b"00.00.00");
        assert_eq!(&max_time, b"23.59.59");
    }

    #[test]
    fn rejects_invalid_hour() {
        assert!(format_clock_bytes(24, 0, 0).is_err());
    }

    #[test]
    fn rejects_invalid_minute() {
        assert!(format_clock_bytes(12, 60, 0).is_err());
    }

    #[test]
    fn rejects_invalid_second() {
        assert!(format_clock_bytes(12, 59, 60).is_err());
    }

    #[test]
    fn wasm_export_returns_packed_ascii_clock_text() {
        let packed = format_time_packed(9, 5, 7);
        assert_eq!(unpack_to_string(packed), "09.05.07");
    }

    #[test]
    fn wasm_export_returns_zero_for_invalid_values() {
        let packed = format_time_packed(99, 0, 0);
        assert_eq!(packed, 0);
    }

    #[test]
    fn rust_router_matches_supported_routes() {
        assert_eq!(route_from(b"GET", b"/"), ROUTE_HOME);
        assert_eq!(route_from(b"GET", b"/clock-stream"), ROUTE_CLOCK_STREAM);
        assert_eq!(route_from(b"POST", b"/theme"), ROUTE_THEME);
        assert_eq!(route_from(b"GET", b"/static/styles.css"), ROUTE_STATIC_ASSET);
    }

    #[test]
    fn rust_router_rejects_unknown_routes() {
        assert_eq!(route_from(b"POST", b"/"), ROUTE_NOT_FOUND);
        assert_eq!(route_from(b"GET", b"/missing"), ROUTE_NOT_FOUND);
        assert_eq!(route_from(b"GET", b"/theme"), ROUTE_NOT_FOUND);
    }

    #[test]
    fn rust_renders_home_html_with_light_theme_by_default() {
        let response = render_home_html_response(9, 5, 7, THEME_LIGHT);

        assert_eq!(response.status, 200);
        assert!(response
            .body
            .contains("<time id=\"clock-time\" datetime=\"09:05:07\">09.05.07</time>"));
        assert!(response.body.contains("data-theme=\"light\""));
        assert!(response.body.contains("/theme?theme=dark"));
        assert!(response.body.contains("Switch to dark mode"));
        assert!(response.body.contains("hx-post=\"/theme?theme=dark\""));
        assert!(response.body.contains("hx-target=\"#page-root\""));
        assert!(response.body.contains("hx-select=\"#page-root\""));
        assert!(response.body.contains("hx-ext=\"sse\""));
        assert!(response.body.contains("timezone-bootstrap.js"));
    }

    #[test]
    fn rust_renders_home_html_with_dark_theme() {
        let response = render_home_html_response(9, 5, 7, "dark");

        assert_eq!(response.status, 200);
        assert!(response.body.contains("data-theme=\"dark\""));
        assert!(response.body.contains("/theme?theme=light"));
        assert!(response.body.contains("Switch to light mode"));
        assert!(response.body.contains("hx-post=\"/theme?theme=light\""));
    }

    #[test]
    fn rust_normalizes_invalid_theme_to_light() {
        let response = render_home_html_response(9, 5, 7, "sepia");

        assert_eq!(response.status, 200);
        assert!(response.body.contains("data-theme=\"light\""));
        assert!(response.body.contains("/theme?theme=dark"));
        assert!(response.body.contains("Switch to dark mode"));
    }

    #[test]
    fn rust_renders_sse_clock_event() {
        let response = render_clock_sse_event_response(9, 5, 7);

        assert_eq!(response.status, 200);
        assert!(response.body.starts_with("event: clock"));
        assert!(response
            .body
            .contains("<time id=\"clock-time\" datetime=\"09:05:07\">09.05.07</time>"));
    }

    #[test]
    fn rust_renders_home_html_for_unix_seconds_and_time_zone() {
        let response = render_home_html_at_unix_seconds_response(0, "Asia/Tokyo", "dark");

        assert_eq!(response.status, 200);
        assert!(response
            .body
            .contains("<time id=\"clock-time\" datetime=\"09:00:00\">09.00.00</time>"));
        assert!(response.body.contains("data-theme=\"dark\""));
    }

    #[test]
    fn rust_falls_back_to_utc_for_unknown_time_zone() {
        let response = render_home_html_at_unix_seconds_response(0, "Mars/Olympus", "light");

        assert_eq!(response.status, 200);
        assert!(response
            .body
            .contains("<time id=\"clock-time\" datetime=\"00:00:00\">00.00.00</time>"));
    }

    #[test]
    fn rust_normalizes_valid_time_zone_values() {
        assert_eq!(
            normalize_time_zone_value("Asia/Tokyo"),
            Some("Asia/Tokyo".to_string())
        );
        assert_eq!(
            normalize_time_zone_value("  Asia/Tokyo  "),
            Some("Asia/Tokyo".to_string())
        );
    }

    #[test]
    fn rust_rejects_invalid_time_zone_values() {
        assert_eq!(normalize_time_zone_value("Mars/Olympus"), None);
    }

    #[test]
    fn rust_rejects_overlong_time_zone_values() {
        let time_zone = format!("{}{}", "Asia/", "x".repeat(101));
        assert_eq!(normalize_time_zone_value(time_zone.as_str()), None);
    }

    #[test]
    fn rust_resolves_time_zone_context_with_query_precedence() {
        let (session, supplied, should_set_cookie) =
            resolve_time_zone_context_values(
                Some("  Asia/Tokyo  "),
                Some("Europe/Paris"),
                Some("America/New_York"),
            );

        assert_eq!(session, Some("Asia/Tokyo".to_string()));
        assert_eq!(supplied, Some("Asia/Tokyo".to_string()));
        assert!(should_set_cookie);
    }

    #[test]
    fn rust_resolves_time_zone_context_with_header_fallback() {
        let (session, supplied, should_set_cookie) =
            resolve_time_zone_context_values(Some("Mars/Olympus"), Some("Europe/Paris"), None);

        assert_eq!(session, Some("Europe/Paris".to_string()));
        assert_eq!(supplied, Some("Europe/Paris".to_string()));
        assert!(should_set_cookie);
    }

    #[test]
    fn rust_resolves_time_zone_context_with_cookie_only() {
        let (session, supplied, should_set_cookie) =
            resolve_time_zone_context_values(None, None, Some("America/New_York"));

        assert_eq!(session, Some("America/New_York".to_string()));
        assert_eq!(supplied, None);
        assert!(!should_set_cookie);
    }

    #[test]
    fn rust_resolves_time_zone_context_without_cookie_update_when_unchanged() {
        let (session, supplied, should_set_cookie) =
            resolve_time_zone_context_values(None, Some("Europe/Paris"), Some("Europe/Paris"));

        assert_eq!(session, Some("Europe/Paris".to_string()));
        assert_eq!(supplied, Some("Europe/Paris".to_string()));
        assert!(!should_set_cookie);
    }

    #[test]
    fn rust_resolves_request_context_time_zone_precedence() {
        let values = resolve_request_context_values(
            "tz=Asia/Tokyo&tz=Europe/Paris",
            "clock_tz=America%2FNew_York; clock_theme=dark",
            Some("Australia/Sydney"),
            None,
        );

        assert_eq!(values.session_time_zone, Some("Europe/Paris".to_string()));
        assert_eq!(values.supplied_time_zone, Some("Europe/Paris".to_string()));
        assert!(values.should_set_time_zone_cookie);
        assert_eq!(values.session_theme, "dark");
        assert_eq!(values.requested_theme, None);
    }

    #[test]
    fn rust_resolves_request_context_theme_precedence_and_cookie_decisions() {
        let values =
            resolve_request_context_values("theme=dark&theme=light", "clock_theme=dark", None, None);

        assert_eq!(values.session_time_zone, None);
        assert_eq!(values.supplied_time_zone, None);
        assert_eq!(values.session_theme, "light");
        assert_eq!(values.requested_theme, Some("light".to_string()));
        assert!(values.should_set_theme_cookie);

        let unchanged_values =
            resolve_request_context_values("theme=dark", "clock_theme=dark", None, None);

        assert_eq!(unchanged_values.session_theme, "dark");
        assert_eq!(unchanged_values.requested_theme, Some("dark".to_string()));
        assert!(!unchanged_values.should_set_theme_cookie);
    }

    #[test]
    fn rust_treats_invalid_percent_encoded_cookie_as_missing() {
        let values =
            resolve_request_context_values("", "clock_tz=Asia%2F; clock_theme=%E0%A4%A", None, None);

        assert_eq!(values.session_time_zone, None);
        assert_eq!(values.supplied_time_zone, None);
        assert_eq!(values.session_theme, "light");
        assert_eq!(values.requested_theme, None);
    }

    #[test]
    fn rust_decodes_percent_encoded_query_values() {
        let values = resolve_request_context_values(
            "tz=Asia%2FTokyo&theme=da%72k&once=%31",
            "",
            None,
            None,
        );

        assert_eq!(values.session_time_zone, Some("Asia/Tokyo".to_string()));
        assert_eq!(values.supplied_time_zone, Some("Asia/Tokyo".to_string()));
        assert_eq!(values.session_theme, "dark");
        assert_eq!(values.requested_theme, Some("dark".to_string()));
        assert!(values.once);
    }

    #[test]
    fn rust_keeps_plus_literal_in_query_values() {
        let (time_zone, theme, once) =
            parse_query_param_values("tz=Asia+Tokyo&theme=da+rk&once=+1");

        assert_eq!(time_zone, Some("Asia+Tokyo".to_string()));
        assert_eq!(theme, Some("da+rk".to_string()));
        assert_eq!(once, Some("+1".to_string()));
    }

    #[test]
    fn rust_rejects_non_utf8_percent_sequences() {
        assert_eq!(decode_query_value("%FF"), None);
        assert_eq!(decode_percent_component("%FF"), None);
    }

    #[test]
    fn rust_treats_invalid_percent_encoded_query_values_as_missing() {
        let values = resolve_request_context_values(
            "tz=Asia%2F&theme=%E0%A4%A&once=%GG",
            "",
            None,
            None,
        );

        assert_eq!(values.session_time_zone, None);
        assert_eq!(values.supplied_time_zone, None);
        assert_eq!(values.session_theme, "light");
        assert_eq!(values.requested_theme, None);
        assert!(!values.once);
    }

    #[test]
    fn rust_sets_once_and_hx_bits_for_request_context() {
        let values = resolve_request_context_values("once=0&once=1", "", None, Some("true"));

        assert!(values.once);
        assert!(values.hx);

        let non_hx_values = resolve_request_context_values("once=1", "", None, Some("TRUE"));

        assert!(!non_hx_values.hx);
    }

    #[test]
    fn rust_renders_sse_event_for_unix_seconds_and_time_zone() {
        let response = render_clock_sse_event_at_unix_seconds_response(0, "Asia/Tokyo");

        assert_eq!(response.status, 200);
        assert!(response.body.starts_with("event: clock"));
        assert!(response
            .body
            .contains("<time id=\"clock-time\" datetime=\"09:00:00\">09.00.00</time>"));
    }

    #[test]
    fn rust_renders_not_found_payload() {
        let response = render_not_found_response();

        assert_eq!(response.status, 404);
        assert_eq!(response.body, "Not Found");
    }

    #[test]
    fn rust_plans_theme_route_with_hx_and_redirect_branches() {
        let htmx_plan = build_http_response_plan(
            "POST",
            "/theme",
            "theme=dark&tz=Asia%2FTokyo",
            "",
            None,
            Some("true"),
            0,
        );

        assert_eq!(htmx_plan.route, ROUTE_THEME);
        assert_eq!(htmx_plan.status, 200);
        assert_eq!(htmx_plan.content_type.as_deref(), Some("text/html; charset=utf-8"));
        assert!(htmx_plan.body.contains("data-theme=\"dark\""));
        assert!(htmx_plan.location.is_none());
        assert_eq!(htmx_plan.set_cookies.len(), 2);

        let redirect_plan = build_http_response_plan(
            "POST",
            "/theme",
            "theme=dark&tz=Asia%2FTokyo",
            "",
            None,
            None,
            0,
        );

        assert_eq!(redirect_plan.route, ROUTE_THEME);
        assert_eq!(redirect_plan.status, 303);
        assert_eq!(redirect_plan.location.as_deref(), Some("/"));
        assert_eq!(redirect_plan.body, "");
        assert!(redirect_plan.content_type.is_none());
        assert_eq!(redirect_plan.set_cookies.len(), 1);
        assert_eq!(
            redirect_plan.set_cookies,
            vec!["clock_theme=dark; Path=/; Max-Age=2592000; SameSite=Lax".to_string()]
        );
    }

    #[test]
    fn rust_plans_cookie_headers_per_route() {
        let home_plan = build_http_response_plan(
            "GET",
            "/",
            "tz=Asia%2FTokyo",
            "",
            None,
            None,
            0,
        );
        assert_eq!(
            home_plan.set_cookies,
            vec!["clock_tz=Asia%2FTokyo; Path=/; Max-Age=2592000; SameSite=Lax".to_string()]
        );

        let stream_plan = build_http_response_plan(
            "GET",
            "/clock-stream",
            "tz=Asia%2FTokyo&once=1",
            "",
            None,
            None,
            0,
        );
        assert_eq!(stream_plan.route, ROUTE_CLOCK_STREAM);
        assert_eq!(stream_plan.session_time_zone, Some("Asia/Tokyo".to_string()));
        assert!(stream_plan.once);
        assert_eq!(
            stream_plan.set_cookies,
            vec!["clock_tz=Asia%2FTokyo; Path=/; Max-Age=2592000; SameSite=Lax".to_string()]
        );

        let theme_htmx_plan = build_http_response_plan(
            "POST",
            "/theme",
            "theme=dark&tz=Asia%2FTokyo",
            "",
            None,
            Some("true"),
            0,
        );
        assert_eq!(
            theme_htmx_plan.set_cookies,
            vec![
                "clock_tz=Asia%2FTokyo; Path=/; Max-Age=2592000; SameSite=Lax".to_string(),
                "clock_theme=dark; Path=/; Max-Age=2592000; SameSite=Lax".to_string(),
            ]
        );

        let theme_redirect_plan = build_http_response_plan(
            "POST",
            "/theme",
            "theme=dark&tz=Asia%2FTokyo",
            "",
            None,
            None,
            0,
        );
        assert_eq!(
            theme_redirect_plan.set_cookies,
            vec!["clock_theme=dark; Path=/; Max-Age=2592000; SameSite=Lax".to_string()]
        );

        let not_found_plan = build_http_response_plan(
            "GET",
            "/missing",
            "tz=Asia%2FTokyo",
            "",
            None,
            None,
            0,
        );
        assert_eq!(not_found_plan.route, ROUTE_NOT_FOUND);
        assert_eq!(
            not_found_plan.set_cookies,
            vec!["clock_tz=Asia%2FTokyo; Path=/; Max-Age=2592000; SameSite=Lax".to_string()]
        );

        let static_plan = build_http_response_plan(
            "GET",
            "/static/site.css",
            "tz=Asia%2FTokyo",
            "",
            None,
            None,
            0,
        );
        assert_eq!(static_plan.route, ROUTE_STATIC_ASSET);
        assert_eq!(static_plan.status, 0);
        assert_eq!(static_plan.body, "");
        assert!(static_plan.set_cookies.is_empty());
    }
}
