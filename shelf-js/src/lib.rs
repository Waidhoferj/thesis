mod utils;

use std::borrow::Borrow;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::ops::Range;
use std::rc::Rc;

use js_sys::{self, Array, Uint8Array};
use rand::prelude::StdRng;
use rand::SeedableRng;
use serde_json;
use serde_json::Value as JSON;
use shelf_crdt::shelf_fuzzer::ShelfFuzzer;
use shelf_crdt::wrap_crdt::Value;
use shelf_crdt::wrap_crdt::{Shelf as ShelfCRDT, ShelfContent};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
struct Fuzzer(ShelfFuzzer);

#[wasm_bindgen]
impl Fuzzer {
    #[wasm_bindgen(constructor)]
    pub fn new(config: &js_sys::Object) -> Self {
        let fuzzer = Fuzzer::from_config(config).unwrap_or_default();
        Self(fuzzer)
    }

    #[wasm_bindgen(js_name = "setSeed")]
    pub fn set_seed(&mut self, seed: u64) {
        self.0.set_seed(seed);
    }

    #[wasm_bindgen(js_name = "generateShelfContent")]
    pub fn generate_shelf_content(&mut self) -> Result<JsValue, JsValue> {
        JsValue::from_serde(&self.0.generate_json_values())
            .or_else(|_| Err(JsValue::from("Failed to convert shelf to JSON")))
    }

    fn extract_range(array: &JsValue) -> Option<Range<usize>> {
        let start = js_sys::Reflect::get(array, &JsValue::from(0_usize)).ok()?;
        let start: usize = start.as_f64()? as usize;
        let end = js_sys::Reflect::get(array, &JsValue::from(1_usize)).ok()?;
        let end: usize = end.as_f64()? as usize;
        let x = end;
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
            .unwrap_or(2..6);
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
// #[wasm_bindgen]
// struct Shelf(ShelfCRDT);

// #[wasm_bindgen]
// impl Shelf {
//     #[wasm_bindgen(constructor)]
//     pub fn new(contents: &JsValue) -> Result<Shelf, String> {
//         let inner = ShelfCRDT::from_json_values(
//             contents.into_serde().or_else(|err| Err(err.to_string()))?,
//         )?;
//         Ok(Shelf(inner))
//     }
// }

#[wasm_bindgen]
pub struct Awareness {
    uid: String,
    users: HashMap<String, Rc<RefCell<ShelfCRDT>>>,
}

#[wasm_bindgen]
impl Awareness {
    #[wasm_bindgen(constructor)]
    pub fn new(uid: Option<String>) -> Self {
        let uid = uid.unwrap_or_else(|| "temp".to_string()); // TODO: Random gen
        let mut users = HashMap::new();
        let user_state = Rc::new(RefCell::new(ShelfCRDT::default()));
        users.insert(uid.clone(), user_state);

        Awareness { uid, users }
    }

    #[wasm_bindgen(js_name = "getUsers")]
    pub fn get_users(&self) -> js_sys::Object {
        let object = js_sys::Object::new();
        for (user, shelf) in self.users.iter() {
            let shelf_view = ShelfView::new(Rc::clone(shelf));
            js_sys::Reflect::set(&object, &JsValue::from(user), &JsValue::from(shelf_view))
                .unwrap();
        }
        return object;
    }
    #[wasm_bindgen(js_name = "getUser")]
    pub fn get_user(&self, uid: &str) -> Option<ShelfView> {
        self.users
            .get(uid)
            .map(|shelf| ShelfView::new(Rc::clone(shelf)))
    }
    #[wasm_bindgen(method, getter)]
    pub fn state(&self) -> Option<ShelfView> {
        self.get_user(&self.uid)
    }
    #[wasm_bindgen(method, setter)]
    pub fn set_state(&mut self, val: &JsValue) {
        let json = val.into_serde().unwrap();
        let shelf = ShelfCRDT::from_json_values(json).unwrap();
        self.users.get_mut(&self.uid).unwrap().replace(shelf);
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        let mut users = Vec::new();
        for (user, shelf) in self.users.iter() {
            users.push(format!("{user}: {}", RefCell::borrow(&*shelf))); // TODO: doesn't have to be mut
        }
        let users = users.join(", ");
        let uid = self.uid.as_str();

        format!("Awareness(uid:{uid}, users: {{{users}}})")
    }

    // #[wasm_bindgen(js_name = "toJson")]
    // pub fn to_json(&self) -> JsValue {
    //     let users = js_sys::Object::new();
    //     for (user, shelf) in self.users.iter() {
    //         let shelf = RefCell::borrow(&*shelf);
    //         let json: JSON = JSON::from(shelf.clone());
    //         js_sys::Reflect::set(
    //             &users,
    //             &JsValue::from(user),
    //             &JsValue::from_serde(&json).unwrap(),
    //         )
    //         .unwrap();
    //     }
    //     let awareness = js_sys::Object::new();
    //     js_sys::Reflect::set(&awareness, &JsValue::from("users"), &JsValue::from(users)).unwrap();
    //     JsValue::from(awareness)
    // }

    // pub fn encode_state_vector(&) -> js_sys::Uint8Array {

    //     js_sys::Uint8Array::from()
    // }
}

#[wasm_bindgen]
pub struct ShelfView {
    target: Rc<RefCell<ShelfCRDT>>,
    path: Vec<String>,
}
#[wasm_bindgen]
impl ShelfView {
    fn new(target: Rc<RefCell<ShelfCRDT>>) -> Self {
        ShelfView {
            target,
            path: Vec::new(),
        }
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string(&self) -> String {
        let shelf = RefCell::borrow(&*self.target);

        format!("ShelfView({})", shelf)
    }

    #[wasm_bindgen(js_name = "toJson")]
    pub fn to_json(&self) -> JsValue {
        let shelf = self.resolve_path(&self.path);

        shelf
            .map(|shelf| {
                let json: JSON = (shelf.clone()).into();
                JsValue::from_serde(&json).unwrap()
            })
            .unwrap_or(JsValue::null())
    }

    pub fn contents(&self) -> JsValue {
        JsValue::from_serde(&RefCell::borrow(&self.target).clone().to_json_values()).unwrap()
    }

    #[wasm_bindgen(method)]
    pub fn get(&self, path: Vec<JsValue>) -> Option<ShelfView> {
        let path: Option<Vec<String>> = path.into_iter().map(|key| key.as_string()).collect();
        path.map(|path| ShelfView {
            path: self.path.iter().cloned().chain(path.into_iter()).collect(),
            target: Rc::clone(&self.target),
        })
    }
    #[wasm_bindgen(method)]
    pub fn set(&mut self, key: &str, val: &JsValue) -> Result<(), String> {
        let shelf = self.resolve_path_mut(&self.path);
        if let Some(mut shelf) = shelf {
            let val = ShelfContent::from_json_values(JSON::String("foo".to_string()))?;
            shelf.set(key.to_string(), val).unwrap();
        }
        Ok(())
    }
    #[wasm_bindgen(method, getter)]
    pub fn value(&self) -> JsValue {
        self.resolve_path(&self.path)
            .and_then(|shelf| match shelf.content.as_ref() {
                Some(ShelfContent::Value(val)) => {
                    let json: JSON = val.clone().into();
                    Some(json)
                }
                Some(ShelfContent::ShelfMap(shelf_map)) => {
                    let contents: serde_json::Map<String, JSON> = shelf_map
                        .iter()
                        .map(|(k, shelf)| (k.clone(), shelf.clone().to_json_values()))
                        .collect();
                    Some(JSON::Object(contents))
                }
                _ => None,
            })
            .as_ref()
            .and_then(|v| JsValue::from_serde(v).ok())
            .unwrap_or(JsValue::null())
    }

    #[wasm_bindgen(method, setter)]
    pub fn set_value(&mut self, val: &JsValue) -> Result<(), String> {
        let json: JSON = val.into_serde().or_else(|e| Err(e.to_string()))?;
        let new_content = ShelfContent::from_json_values(json)?;
        let mut shelf = self
            .resolve_path_mut(&self.path)
            .ok_or("Element does not exist".to_string())?;
        shelf.set_content(new_content);
        Ok(())
    }

    // #[wasm_bindgen(method, structural, indexing_setter)]
    // pub fn set(&mut self, prop: &str, val: &JsValue) -> Result<(), String> {
    //     let json: JSON = val.into_serde().or_else(|e| Err(e.to_string()))?;
    //     let new_content = ShelfContent::from_json_values(json)?;
    //     let mut shelf = self.target.borrow_mut();
    //     shelf.set(prop.to_string(), new_content);
    //     Ok(())
    // }
    // #[wasm_bindgen(method, structural, indexing_deleter)]
    // pub fn delete(&mut self, prop: &str) {
    //     println!("Delete");
    // }

    fn resolve_path(&self, path: &Vec<String>) -> Option<Ref<ShelfCRDT>> {
        let target = RefCell::borrow(&*self.target);
        let mut cur = target;
        for key in path.iter() {
            cur = Ref::map(cur, |shelf| shelf.get(&key).unwrap());
        }
        Some(cur)
    }

    fn resolve_path_mut(&self, path: &Vec<String>) -> Option<RefMut<ShelfCRDT>> {
        let target = self.target.borrow_mut();
        let mut cur = target;
        for key in path.iter() {
            cur = RefMut::map(cur, |shelf| shelf.get_mut(&key).unwrap());
        }
        Some(cur)
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}
