use std::collections::HashMap;
use std::io::Bytes;
use std::ops::Range;

use js_sys::{self, Array, Uint8Array};
use rand::prelude::StdRng;
use rand::SeedableRng;
use serde_json;
use serde_json::Value as JSON;
use shelf_crdt::json::Value;
use shelf_crdt::traits::{DeltaCRDT, Mergeable};

use shelf_crdt::clock::{LamportTimestamp, LamportTimestampGenerator};
use shelf_crdt::state_vector::{StateVector, StateVectorContext};
use wasm_bindgen::prelude::*;

type AwarenessClient =
    shelf_crdt::wrap_crdt::Awareness<Value, LamportTimestamp, LamportTimestamp, StateVectorContext>;

type ClientShelf = shelf_crdt::wrap_crdt::Shelf<Value, LamportTimestamp, LamportTimestamp>;

#[wasm_bindgen]
pub struct Awareness {
    inner: AwarenessClient,
}

#[wasm_bindgen]
impl Awareness {
    #[wasm_bindgen(constructor)]
    pub fn new(content: JsValue, client_id: usize) -> Self {
        let inner = if content.is_undefined() {
            AwarenessClient::new_for_client(client_id, StateVectorContext {})
        } else {
            let values = content.into_serde().unwrap_throw();
            AwarenessClient::from_json_values(values, client_id).unwrap_throw()
        };
        Self { inner }
    }
    pub fn get(&self, path: Array, client_id: Option<String>) -> JsValue {
        let mut shelf = client_id
            .map(|cid| self.inner.get_peer_state(&cid).unwrap_throw())
            .unwrap_or_else(|| self.inner.get_own_state().unwrap_throw());
        for key in path.iter() {
            if let Some(key) = key.as_string() {
                shelf = shelf
                    .get(&key)
                    .ok_or_else(|| format!("Key Error: {}", key))
                    .unwrap_throw()
            } else {
                Err(format!("Invalid key: {:?}", key)).unwrap_throw()
            }
        }

        let json = shelf.clone().to_json_values();
        JsValue::from_serde(&json).unwrap_throw()
    }

    pub fn set(&mut self, path: Array, contents: JsValue) {
        let path = Self::convert_path(path).unwrap_throw();
        let json = contents.into_serde().unwrap_throw();
        let shelf = ClientShelf::from_json_values(
            json,
            &mut LamportTimestampGenerator {},
            &mut LamportTimestampGenerator {},
        )
        .unwrap_throw();
        self.inner.set_state(path, shelf).unwrap_throw();
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        format!("Awareness({})", self.inner.clients)
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> JsValue {
        let json: JSON = self.inner.clients.clone().to_json_values(); // TODO, include clocks?
        JsValue::from_serde(&json).unwrap()
    }

    #[wasm_bindgen(js_name = "getStateVector")]
    pub fn get_state_vector(&self) -> JsValue {
        let sv = self.inner.clients.get_state_vector();
        let bytes = bincode::serialize(&sv);
        match bytes {
            Ok(bytes) => Uint8Array::from(&bytes[..]).into(),
            Err(err) => Err(err).unwrap_throw(),
        }
    }

    #[wasm_bindgen(js_name = "getStateDelta")]
    pub fn get_state_delta(&self, sv: Uint8Array) -> JsValue {
        let decoded_sv: StateVector<LamportTimestamp, LamportTimestamp> =
            bincode::deserialize(&sv.to_vec()[..]).unwrap_throw();
        let bytes = self
            .inner
            .clients
            .get_state_delta(&decoded_sv)
            .map(|delta| bincode::serialize(&delta));

        match bytes {
            Some(Ok(bytes)) => Uint8Array::from(&bytes[..]).into(),
            Some(Err(err)) => Err(err).unwrap_throw(),
            None => JsValue::null(),
        }
    }
    #[wasm_bindgen]
    pub fn merge(&mut self, delta: Uint8Array) {
        let delta: ClientShelf = bincode::deserialize(&delta.to_vec()[..]).unwrap_throw();
        self.inner.merge(delta);
    }

    /// Converts a JavaScript Array to a path of strings. Returns `None` on failure
    #[inline]
    fn convert_path(list: Array) -> Option<Vec<String>> {
        list.iter().map(|segment| segment.as_string()).collect()
    }
}
