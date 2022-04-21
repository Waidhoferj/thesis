use std::collections::HashMap;

use crate::temporal::Temporal;

/*
Example of Adjacent derive

#[shelf]
struct Person {
    name: String,
    age: usize
    #[shelf_exclude] // Mark something as excludable
    some_local_state: DeeplyNestedObject

}

Would generate get_state_vector, diff, and update functions:

get_state_vector() -> Vec<usize>

* Note problem with dynamic number of users making static vector clocks infeasible.
    * Perhaps perform SIMD operations on individual pairs of vector clocks?
    * Bartosz claimed that this is a bad optimization

This would be a lot more efficient because
    1. Structure is known so we could flatten timestamps into a SIMD array
    2. Way more ergonomic

There are a few downsides:
    1. Recursive structures would not be efficient
    2. Works best for compiled languages. Interpreted structures over an FFI might be slow


Questions:
    * Would it be possible to use the state vector as a bit-mask and write merge operations to receiving structure parallel?
    * Could the state vector be a buffer of pointers in memory? Not for sharing but comparing diffs

Ideas:
    * 1d linked list in memory buffer. Moving and compaction necessary.
    * levelwise hash where children can recompute without affecting parents
*/

// impl PartialOrd for VectorClock {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         if self.0.intersect(&other.0) {
//             Some(Ordering::Greater)
//         }
//     }
// }

//#[shelf]

// doc.update("foo", crdt) -> crdt

// struct Doc(HashMap<String, Box<dyn CRDT>>);
/*
    doc.set("foo", crdt) where crdt impl CRDT
    doc.update
    Proxy<type>
    proxy is the same thing as the type but with options in all spaces


*/
struct TestDs {
    // #[scoped]
    s: String,
    b: bool,
    i: isize,
    u: usize,
    f: f64,
    arr: Vec<u8>,
    map: HashMap<String, String>,
}

struct TestDsStateVector {
    s: Temporal,
    b: Temporal,
    i: Temporal,
    u: Temporal,
    f: Temporal,
}

// impl PartialOrd for TestDsStateVector {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         match self.s.partial_cmp(&other.s) {
//             Some(core::cmp::Ordering::Equal) => {}
//             ord => return ord,
//         }
//         match self.b.partial_cmp(&other.b) {
//             Some(core::cmp::Ordering::Equal) => {}
//             ord => return ord,
//         }
//         match self.i.partial_cmp(&other.i) {
//             Some(core::cmp::Ordering::Equal) => {}
//             ord => return ord,
//         }
//         match self.u.partial_cmp(&other.u) {
//             Some(core::cmp::Ordering::Equal) => {}
//             ord => return ord,
//         }
//         self.f.partial_cmp(&other.f)
//     }
// }

// TODOS:
//  - Dynamic with JSON
//  - Static with structs
