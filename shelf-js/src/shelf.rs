use std::collections::hash_map::Entry;

use shelf_crdt::clock::{
    DotClock, DotClockGenerator, LamportTimestamp, LamportTimestampGenerator, LogicalClock,
};
use shelf_crdt::state_vector::StateVector;
use shelf_crdt::traits::{DeltaCRDT, Mergeable};

use js_sys::{self, Array, Uint8Array};
use serde_json;
use serde_json::Value as JSON;
use shelf_crdt::json::Value;
use shelf_crdt::wrap_crdt::Shelf as GeneralShelfCRDT;
use wasm_bindgen::prelude::*;

type ShelfCRDT = GeneralShelfCRDT<Value, LamportTimestamp, DotClock>;

#[wasm_bindgen]
pub struct DotShelf(ShelfCRDT);

#[wasm_bindgen]
impl DotShelf {
    #[wasm_bindgen(constructor)]
    pub fn new(content: JsValue, client_id: usize) -> Result<DotShelf, String> {
        if content.is_undefined() {
            return Err("Content must be provided".to_owned());
        }
        let inner = {
            let json: JSON = content.into_serde().unwrap_throw();
            ShelfCRDT::from_json_values(
                json,
                &mut LamportTimestampGenerator {},
                &mut DotClockGenerator::new(client_id),
            )
            .unwrap_throw()
        };
        Ok(Self(inner))
    }
    #[wasm_bindgen]
    pub fn get(&self, path: Array) -> JsValue {
        let mut shelf = &self.0;
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
    #[wasm_bindgen]
    pub fn set(&mut self, path: Array, contents: JsValue, client_id: usize) {
        let path = Self::convert_path(path).unwrap_throw();
        let (entry, parent_clock) = self.0.entry_from_path(path).unwrap_throw();
        let parent_clock = parent_clock.0;
        let new_ts = match &entry {
            Entry::Occupied(occupied_entry) => {
                let old_value = occupied_entry.get();
                match old_value {
                    ShelfCRDT::Value { clock, .. } => Some(clock.clock.max(parent_clock) + 1), // New clock must be
                    ShelfCRDT::Map {
                        shelves,
                        clock: LamportTimestamp(old_clock),
                    } => {
                        let highest_child_timestamp = shelves
                            .iter()
                            .map(|(_, shelf)| shelf.get_clock().get_logical_clock())
                            .max();
                        highest_child_timestamp.map(|ts| ts.max(parent_clock).max(*old_clock) + 1)
                    }
                }
            }
            Entry::Vacant(_) => None,
        };
        let new_ts = new_ts.unwrap_or(parent_clock + 1);
        let json = contents.into_serde().unwrap_throw();
        let contents = ShelfCRDT::from_json_values(
            json,
            &mut LamportTimestampGenerator {},
            &mut DotClockGenerator::new(client_id),
        )
        .unwrap_throw();
        let value = match contents {
            ShelfCRDT::Value { value, .. } => ShelfCRDT::Value {
                value,
                clock: DotClock {
                    client_id,
                    clock: new_ts,
                },
            },
            ShelfCRDT::Map { shelves, .. } => ShelfCRDT::Map {
                shelves,
                clock: new_ts.into(),
            },
        };
        match entry {
            Entry::Occupied(mut o) => Some(o.insert(value)),
            Entry::Vacant(v) => {
                v.insert(value);
                None
            }
        };
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        format!("Shelf({})", self.0)
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> JsValue {
        let json: JSON = self.0.clone().into();
        JsValue::from_serde(&json).unwrap()
    }

    #[wasm_bindgen(js_name = "getStateVector")]
    pub fn get_state_vector(&self) -> JsValue {
        let sv = self.0.get_state_vector();
        let bytes = bincode::serialize(&sv).unwrap_throw();
        Uint8Array::from(&bytes[..]).into()
    }

    #[wasm_bindgen(js_name = "getStateDelta")]
    pub fn get_state_delta(&self, sv: Uint8Array) -> JsValue {
        let decoded_sv: StateVector<LamportTimestamp, DotClock> =
            bincode::deserialize(&sv.to_vec()[..]).unwrap_throw();
        let bytes = self
            .0
            .get_state_delta(&decoded_sv)
            .map(|delta| bincode::serialize(&delta));

        match bytes {
            Some(Ok(bytes)) => Uint8Array::from(&bytes[..]).into(),
            Some(Err(err)) => Err(err).unwrap_throw(),
            None => JsValue::null(),
        }
    }
    #[wasm_bindgen]
    pub fn merge(self, delta_bytes: Uint8Array) -> Self {
        let delta: ShelfCRDT = bincode::deserialize(&delta_bytes.to_vec()[..]).unwrap_throw();
        Self(self.0.merge(delta))
    }

    /// Converts a JavaScript Array to a path of strings. Returns `None` on failure
    #[inline]
    fn convert_path(list: Array) -> Option<Vec<String>> {
        list.iter().map(|segment| segment.as_string()).collect()
    }
}

impl From<ShelfCRDT> for DotShelf {
    fn from(value: ShelfCRDT) -> Self {
        Self(value)
    }
}
