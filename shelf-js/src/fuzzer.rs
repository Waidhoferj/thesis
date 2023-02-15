use std::ops::Range;

use wasm_bindgen::prelude::*;

use js_sys::{self, Array, JsString, Uint8Array};
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use serde_json;
use serde_json::Value as JSON;
use shelf_crdt::clock::LamportTimestamp;
use shelf_crdt::json::Value;
use shelf_crdt::shelf_fuzzer::ShelfFuzzer;

#[wasm_bindgen]
pub struct Fuzzer(ShelfFuzzer);

#[wasm_bindgen]
impl Fuzzer {
    #[wasm_bindgen(constructor)]
    pub fn new(config: &js_sys::Object) -> Self {
        let fuzzer = Fuzzer::from_config(config).unwrap_or_default();
        Self(fuzzer)
    }

    #[wasm_bindgen(js_name = "setSeed")]
    pub fn set_seed(&mut self, seed: u32) {
        self.0.set_seed(seed as u64);
    }

    #[wasm_bindgen(js_name = "generateContent")]
    pub fn generate_content(&mut self) -> Result<JsValue, JsValue> {
        JsValue::from_serde(&self.0.generate_json_values())
            .or_else(|_| Err(JsValue::from("Failed to convert shelf to JSON")))
    }

    fn extract_range(array: &JsValue) -> Option<Range<usize>> {
        let start = js_sys::Reflect::get(array, &JsValue::from(0_usize)).ok()?;
        let start: usize = start.as_f64()? as usize;
        let end = js_sys::Reflect::get(array, &JsValue::from(1_usize)).ok()?;
        let end: usize = end.as_f64()? as usize;
        Some(start..end)
    }

    fn from_config(config: &js_sys::Object) -> Option<ShelfFuzzer> {
        let branch_range = js_sys::Reflect::get(config, &JsValue::from("branchRange"))
            .ok()
            .and_then(|val| Self::extract_range(&val))
            .unwrap_or(1..5);
        let value_range = js_sys::Reflect::get(config, &JsValue::from("valueRange"))
            .ok()
            .and_then(|val| Self::extract_range(&val))
            .unwrap_or(2..6);
        let depth_range = js_sys::Reflect::get(config, &JsValue::from("depthRange"))
            .ok()
            .and_then(|val| Self::extract_range(&val))
            .unwrap_or(1..2);
        let seed = js_sys::Reflect::get(config, &JsValue::from("seed")).ok()?;
        let seed = seed.as_f64()? as u64;
        Some(ShelfFuzzer {
            rng: StdRng::seed_from_u64(seed),
            branch_range,
            value_range,
            depth_range,
        })
    }
}
