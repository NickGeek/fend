#![allow(unused_unsafe)]

use instant::Instant;
use std::fmt::Write;
use wasm_bindgen::prelude::*;

struct TimeoutInterrupt {
    start: Instant,
    timeout: u128,
}

impl TimeoutInterrupt {
    fn new_with_timeout(timeout: u128) -> Self {
        Self {
            start: Instant::now(),
            timeout,
        }
    }
}

impl fend_core::Interrupt for TimeoutInterrupt {
    fn should_interrupt(&self) -> bool {
        Instant::now().duration_since(self.start).as_millis() > self.timeout
    }
}

#[wasm_bindgen]
pub fn initialise() {}

// These two functions should be merged at some point, but that would be a breaking
// API change.

#[wasm_bindgen(js_name = evaluateFendWithTimeout)]
pub fn evaluate_fend_with_timeout_2(input: &str, timeout: u32) -> String {
    evaluate_fend_with_timeout(input, timeout)
}

fn random_u32() -> u32 {
    let random_f64 = unsafe { js_sys::Math::random() };
    (random_f64 * f64::from(u32::MAX)) as u32
}

fn create_context() -> fend_core::Context {
    let mut ctx = fend_core::Context::new();
    let date = js_sys::Date::new_0();
    ctx.set_current_time_v1(
        date.get_time() as u64,
        date.get_timezone_offset() as i64 * 60,
    );
    ctx.set_random_u32_fn(random_u32);
    ctx
}

#[wasm_bindgen]
pub fn evaluate_fend_with_timeout(input: &str, timeout: u32) -> String {
    let mut ctx = create_context();
    let interrupt = TimeoutInterrupt::new_with_timeout(u128::from(timeout));
    match fend_core::evaluate_with_interrupt(input, &mut ctx, &interrupt) {
        Ok(res) => {
            if res.is_unit_type() {
                return "".to_string();
            }
            res.get_main_result().to_string()
        }
        Err(msg) => format!("Error: {msg}"),
    }
}

/// Takes a '\0'-separated string of inputs, and returns a '\0'-separated string of results
#[wasm_bindgen(js_name = evaluateFendWithTimeoutMultiple)]
pub fn evaluate_fend_with_timeout_multiple(inputs: &str, timeout: u32) -> String {
    let mut ctx = create_context();
    let mut result = String::new();
    for input in inputs.split('\0') {
        if !result.is_empty() {
            result.push('\0');
        }
        let interrupt = TimeoutInterrupt::new_with_timeout(u128::from(timeout));
        match fend_core::evaluate_with_interrupt(input, &mut ctx, &interrupt) {
            Ok(res) => {
                if !res.is_unit_type() {
                    result.push_str(res.get_main_result());
                }
            }
            Err(msg) => {
                result.push_str("Error: ");
                result.push_str(&msg);
            }
        };
    }
    result
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, std::num::ParseIntError>>()
        .map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = evaluateFendWithVariablesJson)]
pub fn evaluate_fend_with_variables_json(input: &str, timeout: u32, variables: &str) -> String {
    let mut ctx = create_context();
    if !variables.is_empty() {
        if let Ok(variables) = decode_hex(variables) {
            let _ = ctx.deserialize_variables(&mut variables.as_slice());
        }
    }
    let interrupt = TimeoutInterrupt::new_with_timeout(u128::from(timeout));
    match fend_core::evaluate_with_interrupt(input, &mut ctx, &interrupt) {
        Ok(res) => {
            let escaped_result = {
                let mut escaped_result = String::new();
                if !res.is_unit_type() {
                    fend_core::json::escape_string(res.get_main_result(), &mut escaped_result);
                }
                escaped_result
            };
            let variables = {
                let mut vars_vec = vec![];
                // if we can't serialize variables just ignore it and return an empty string
                let _ = ctx.serialize_variables(&mut vars_vec);
                let mut hex = String::new();
                for b in &vars_vec {
                    write!(hex, "{:02x}", b).unwrap();
                }
                hex
            };
            format!(
                r#"{{"ok":true,"result":"{}","variables":"{}"}}"#,
                escaped_result, variables
            )
        }
        Err(msg) => {
            let mut escaped = String::new();
            fend_core::json::escape_string(&msg, &mut escaped);
            format!(r#"{{"ok":false,"message":"{}"}}"#, escaped)
        }
    }
}

#[wasm_bindgen(js_name = substituteInlineFendExpressions)]
pub fn substitute_inline_fend_expressions(input: &str, timeout: u32) -> String {
    let mut ctx = create_context();
    let interrupt = TimeoutInterrupt::new_with_timeout(u128::from(timeout));
    let res = fend_core::substitute_inline_fend_expressions(input, &mut ctx, &interrupt);
    res.to_json()
}
