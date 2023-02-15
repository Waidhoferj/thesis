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

mod awareness;
mod fuzzer;
mod secure_shelf;
mod shelf;
mod utils;
pub use awareness::Awareness;
pub use fuzzer::Fuzzer;
pub use secure_shelf::SecureShelf;
pub use shelf::DotShelf;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}
