use crate::traits::{ClockGenerator, Mergeable, UpdateStrategy};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value as JSON};

use crate::clock::ShelfClock;
use crate::json::Value;
use std::clone::Clone;
use std::cmp::Ordering;
use std::fmt::Display;
use std::{collections::HashMap, fmt::Debug};
use uuid;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum Shelf<T, MapClock, ValueClock = MapClock>
where
    T: PartialEq + PartialOrd,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    Value {
        value: T,
        clock: ValueClock,
    },
    Map {
        shelves: HashMap<String, Shelf<T, MapClock, ValueClock>>,
        clock: MapClock,
    },
}

impl<MapClock, ValueClock> Shelf<Value, MapClock, ValueClock>
where
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    pub fn from_json_values<'a, CG>(json: JSON, context: &mut CG) -> Result<Self, String>
    where
        CG: ClockGenerator<MapClock> + ClockGenerator<ValueClock>,
    {
        match json {
            JSON::Object(obj) => {
                let mut shelves: HashMap<String, Shelf<Value, MapClock, ValueClock>> =
                    HashMap::new();
                for (k, v) in obj {
                    shelves.insert(k, Shelf::from_json_values(v, context)?);
                }
                Ok(Shelf::Map {
                    shelves,
                    clock: context.new_clock(),
                })
            }
            val => Ok(Shelf::Value {
                value: val.try_into()?,
                clock: context.new_clock(),
            }),
        }
    }

    pub fn to_json_values(self) -> JSON {
        match self {
            Shelf::Value { value, .. } => value.into(),
            Shelf::Map { shelves, .. } => {
                let json_map: serde_json::Map<String, JSON> = shelves
                    .into_iter()
                    .map(|(k, shelf)| (k, shelf.to_json_values()))
                    .collect();
                JSON::Object(json_map)
            }
        }
    }
}

impl<T, MapClock, ValueClock> Display for Shelf<T, MapClock, ValueClock>
where
    T: Display + PartialOrd + PartialEq + Clone,
    MapClock: Display + PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: Display + PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Shelf::Value { value, clock } => format!("[{value}, {clock}]"),
            Shelf::Map { shelves, clock } => {
                let strs: Vec<String> = shelves
                    .into_iter()
                    .map(|(k, shelf)| format!("\"{k}\": {shelf}"))
                    .collect();
                format!("[{{{}}}, {}]", strs.join(", "), clock)
            }
        };
        write!(f, "{}", repr)
    }
}

impl<T, MapClock, ValueClock> Debug for Shelf<T, MapClock, ValueClock>
where
    T: Display + PartialOrd + Clone,
    MapClock: Display + PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: Display + PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return std::fmt::Display::fmt(&self, f);
    }
}

impl<T, MapClock, ValueClock> PartialOrd for Shelf<T, MapClock, ValueClock>
where
    T: PartialOrd,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (
                Shelf::Map {
                    clock: this_clock, ..
                },
                Shelf::Map {
                    clock: other_clock, ..
                },
            ) => {
                let clock_order = this_clock.partial_cmp(other_clock);
                if let Some(Ordering::Equal) = clock_order {
                    None // Cannot order two maps with the same clock value
                } else {
                    clock_order
                }
            } // Cannot order 2 shelf maps.
            (
                Shelf::Map {
                    clock: this_clock, ..
                },
                Shelf::Value {
                    clock: other_clock, ..
                },
            ) => {
                let clock_order = this_clock.partial_cmp(other_clock);
                if let Some(Ordering::Equal) | None = clock_order {
                    Some(Ordering::Greater)
                } else {
                    clock_order
                }
            }
            (
                Shelf::Value {
                    clock: this_clock, ..
                },
                Shelf::Map {
                    clock: other_clock, ..
                },
            ) => {
                let clock_order = this_clock.partial_cmp(other_clock);
                if let Some(Ordering::Equal) | None = clock_order {
                    Some(Ordering::Less)
                } else {
                    clock_order
                }
            }
            (
                Shelf::Value {
                    value: this_value,
                    clock: this_clock,
                },
                Shelf::Value {
                    value: other_value,
                    clock: other_clock,
                },
            ) => {
                let clock_order = this_clock.partial_cmp(other_clock);
                if let Some(Ordering::Equal) | None = clock_order {
                    this_value.partial_cmp(other_value)
                } else {
                    clock_order
                }
            }
        }
    }
}

impl<MapClock, ValueClock> TryFrom<JSON> for Shelf<Value, MapClock, ValueClock>
where
    MapClock: PartialEq + PartialOrd + TryFrom<JSON> + PartialOrd<ValueClock>,
    ValueClock: PartialEq + PartialOrd + TryFrom<JSON> + PartialOrd<MapClock>,
{
    type Error = String;

    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        match json {
            JSON::Array(mut array) => {
                if array.len() != 2 {
                    return Err("Array did not have 2 dimensions".to_string());
                }

                let (value, clock) = (array.remove(0), array.remove(0));
                let mut shelf = match value {
                    JSON::Object(obj) => {
                        let mut shelves: HashMap<String, Shelf<Value, MapClock, ValueClock>> =
                            HashMap::new();
                        for (k, v) in obj {
                            shelves.insert(k, v.try_into()?);
                        }
                        let clock =
                            MapClock::try_from(clock).map_err(|_| "Could not parse MapClock")?;
                        Shelf::Map { shelves, clock }
                    }
                    value => {
                        let value: Value = value.try_into()?;
                        let clock: ValueClock =
                            clock.try_into().map_err(|_| "Could not parse ValueClock")?;
                        Shelf::Value { value, clock }
                    }
                };
                // shelf.prune();

                Ok(shelf)
            }
            val => Err(format!("Could not covert JSON into a shelf: {:?}", val)),
        }
    }
}

impl<MapClock, ValueClock> From<Shelf<Value, MapClock, ValueClock>> for JSON
where
    MapClock: PartialEq + PartialOrd + Into<JSON> + PartialOrd<ValueClock>,
    ValueClock: PartialEq + PartialOrd + Into<JSON> + PartialOrd<MapClock>,
{
    fn from(content: Shelf<Value, MapClock, ValueClock>) -> Self {
        // Include clocks?
        match content {
            Shelf::Value { value, clock } => {
                let clock: JSON = clock.into();
                json!([value, clock])
            }
            Shelf::Map { shelves, clock } => {
                let shelves: HashMap<_, _> = shelves
                    .into_iter()
                    .map(|(k, shelf)| (k, JSON::from(shelf)))
                    .collect();
                let clock: JSON = clock.into();
                json!([shelves, clock])
            }
        }
    }
}

impl<T, MapClock, ValueClock> Shelf<T, MapClock, ValueClock>
where
    T: PartialEq + PartialOrd,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    /// Compares clock values
    pub fn get_clock(&self) -> ShelfClock<MapClock, ValueClock> {
        match &self {
            Shelf::Value { clock, .. } => ShelfClock::ValueClock(clock),
            Shelf::Map { clock, .. } => ShelfClock::MapClock(clock),
        }
    }

    /// Determines whether there are more shelves nested inside this one.
    pub fn contains_shelves(&self) -> bool {
        match &self {
            Self::Map { .. } => true,
            _ => false,
        }
    }
    ///  Gets a Value out of the Shelf
    pub fn get(&self, key: &str) -> Option<&Self> {
        match &self {
            Self::Map { shelves, .. } => shelves.get(key),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        match self {
            Self::Map { shelves, .. } => shelves.get_mut(key),
            _ => None,
        }
    }
    // TODO: Implement update strategies
    // pub fn set<Val: Into<Self>>(
    //     &mut self,
    //     key: String,
    //     value: Val,
    // ) -> Option<Self> {
    //     let mut new_shelf: Self = value.into();
    //     match &mut self.content {
    //         Self::Map { shelves, clock } => {
    //             if let Some(old_shelf) = shelves.get(&key) {
    //                 // TODO: If the new value being set is a map, then take the max child id and increment it
    //                 new_shelf.clock = old_shelf.clock.increment(client_id)
    //             }
    //             shelves.insert(key, new_shelf)
    //         }
    //         _ => None,
    //     }
    // }
    /// Deletes any ShelfMap children with a lower clock value than the parent.
    pub fn prune(&mut self) {
        match self {
            Self::Map { shelves, clock } => {
                let clock = ShelfClock::MapClock(clock);
                shelves.retain(|_, shelf| match shelf.get_clock().partial_cmp(&clock) {
                    Some(Ordering::Greater | Ordering::Equal) | None => true, // None included because clocks could
                    _ => false,
                })
            }
            _ => return,
        };
    }

    /// Recursively prunes the shelf tree
    pub fn garbage_collect(&mut self) {
        self.prune();
        let shelf_map = match self {
            Self::Map { shelves, .. } => shelves,
            _ => return,
        };
        shelf_map.iter_mut().for_each(|(_, shelf)| {
            shelf.garbage_collect();
        });
    }
}

// impl Shelf<Value> {
//     pub fn from_json_values(values: JSON, client_id: usize) -> Result<Self, String> {
//         let content = match values {
//             JSON::Object(obj) => {
//                 let mut children = HashMap::new();
//                 for (key, value) in obj {
//                     children.insert(key, Self::from_json_values(value, client_id)?);
//                 }
//                 ShelfContent::ShelfMap(children)
//             }
//             val => ShelfContent::Value(val.try_into()?),
//         };

//         Ok(Shelf {
//             content,
//             clock: ShelfClock::new(client_id),
//         })
//     }

//     pub fn to_json_values(self) -> JSON {
//         self.content.to_json_values()
//     }
// }

/*
Currently (x, 1) cmp (x,1) -> x must be in delta
    Because we don't know if clock was set by current client or another
We want (x, (1,1)) cmp (x, (1,1))
    Because we can prune this

*/

impl<T, MapClock, ValueClock> Mergeable<Self> for Shelf<T, MapClock, ValueClock>
where
    T: PartialOrd,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
{
    /// Merges another shelf into the current one, returning the resulting union.
    fn merge(self, other: Self) -> Self {
        let clock_order = self.get_clock().partial_cmp(&other.get_clock());
        // Merging two shelf maps

        match (self, other, clock_order) {
            (_, other, Some(Ordering::Less)) => other, // Update is greater so take on that value
            (this, _, Some(Ordering::Greater)) => this, // Self is greater so keep value
            (
                Self::Map {
                    shelves: mut these_shelves,
                    clock: this_clock,
                },
                Self::Map {
                    shelves: other_shelves,
                    clock: other_clock,
                },
                _,
            ) => {
                for (key, val) in other_shelves.into_iter() {
                    let updated_value = if let Some(sub_shelf) = these_shelves.remove(&key) {
                        sub_shelf.merge(val)
                    } else {
                        val
                    };
                    these_shelves.insert(key, updated_value);
                }
                let clock = if this_clock > other_clock {
                    this_clock
                } else {
                    other_clock
                };
                Self::Map {
                    shelves: these_shelves,
                    clock,
                }
            } // If there is no priority between maps, they should be merged recursively.
            (this, _, Some(Ordering::Equal)) => this, // Ruling out recursive map merges ^, if clocks are the same, then the value is unchanged.
            (this, other, None) => {
                // Try partial comparison of content and default to client_ids if this fails. Type compare will fail for things like floats that equal NaN.
                match this.partial_cmp(&other) {
                    Some(Ordering::Greater | Ordering::Equal) => this,
                    Some(Ordering::Less) => other,
                    None => panic!("Could not determine order of elements"),
                }
            } // In the case that both are different shelf content types, just take the type max.
        }
    }
}

struct Awareness<T, MapClock, ValueClock, Updater>
where
    T: PartialOrd,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock>,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
    Updater: UpdateStrategy<Target = Shelf<T, MapClock, ValueClock>>,
{
    clients: Shelf<T, MapClock, ValueClock>,
    update_strategy: Updater,
    client_id: String,
}

impl<T, MapClock, ValueClock, Updater> Awareness<T, MapClock, ValueClock, Updater>
where
    T: PartialOrd,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock> + Default,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
    Updater: UpdateStrategy<Target = Shelf<T, MapClock, ValueClock>>,
{
    fn new(update_strategy: Updater) -> Self {
        let client_id = uuid::Uuid::new_v4().to_string();
        Awareness {
            clients: Shelf::Map {
                shelves: HashMap::new(),
                clock: MapClock::default(),
            },
            client_id,
            update_strategy,
        }
    }

    fn get_peer_state(&self, key: &str) -> Option<&Shelf<T, MapClock, ValueClock>> {
        self.clients.get(key)
    }

    fn get_own_state_mut(&mut self) -> Option<&mut Shelf<T, MapClock, ValueClock>> {
        self.clients.get_mut(&self.client_id)
    }

    fn set_state(
        &mut self,
        value: Shelf<T, MapClock, ValueClock>,
    ) -> Option<Shelf<T, MapClock, ValueClock>> {
        self.update_strategy
            .set(&mut self.clients, self.client_id.to_owned(), value)
    }
}

impl<MapClock, ValueClock, Updater> Awareness<Value, MapClock, ValueClock, Updater>
where
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock> + Default,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock>,
    Updater: UpdateStrategy<Target = Shelf<Value, MapClock, ValueClock>>
        + Default
        + ClockGenerator<MapClock>
        + ClockGenerator<ValueClock>,
{
    fn from_values(values: JSON) -> Result<Self, String> {
        let client_id = uuid::Uuid::new_v4().to_string();
        let mut update_strategy = Updater::default();
        let shelf: Shelf<Value, MapClock, ValueClock> =
            Shelf::from_json_values(values, &mut update_strategy)?;
        Ok(Awareness {
            clients: shelf,
            client_id,
            update_strategy,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::clock::{ClientClock, LamportTimestamp};
    use crate::traits::DeltaCRDT;

    type TestShelf = Shelf<Value, LamportTimestamp, ClientClock>;

    use rand::{prelude::StdRng, SeedableRng};

    use crate::{shelf_fuzzer::ShelfFuzzer, wrap_crdt::*};

    fn merge(branch: TestShelf, mut main: TestShelf) -> TestShelf {
        let sv = main.get_state_vector();
        if let Some(delta) = branch.get_state_delta(&sv) {
            return main.merge(delta);
        }
        return main;
    }

    fn val<T: Into<Value>>(value: T, c: impl Into<LamportTimestamp>) -> TestShelf {
        Shelf::Value {
            value: value.into(),
            clock: clock(c.into().0),
        }
    }

    fn array<T: Into<Value>>(arr: Vec<T>, c: impl Into<LamportTimestamp>) -> TestShelf {
        let arr: Vec<Value> = arr.into_iter().map(|v| v.into()).collect();
        Shelf::Value {
            value: arr.into(),
            clock: clock(c.into().0),
        }
    }

    fn shelf_map(
        contents: impl Iterator<Item = (String, TestShelf)>,
        c: impl Into<LamportTimestamp>,
    ) -> TestShelf {
        let shelves = HashMap::from_iter(contents);
        Shelf::Map {
            shelves,
            clock: c.into(),
        }
    }

    fn clock(c: usize) -> ClientClock {
        ClientClock {
            client_id: 0,
            clock: c,
        }
    }

    fn validate_crdt_properties(shelf: TestShelf, shelf2: TestShelf) -> TestShelf {
        // Forwards
        let mut receiver = shelf.clone();
        let sender = shelf2.clone();
        let sv = receiver.get_state_vector();

        let delta = sender.get_state_delta(&sv);
        let cached_delta = delta.clone();
        if let Some(delta) = delta {
            receiver = receiver.merge(delta);
        }

        // Backwards
        let mut receiver_back = shelf2.clone();
        let sender_back = shelf.clone();
        let sv_back = receiver_back.get_state_vector();
        let delta_back = sender_back.get_state_delta(&sv_back);
        let cached_delta_back = delta_back.clone();
        if let Some(delta) = delta_back {
            receiver_back = receiver_back.merge(delta);
        }
        let id_delta = cached_delta.clone();
        let report = || {
            format!("\nReceiver: {shelf}, \nSender {shelf2}, \nStateVector {sv:?}, \nDelta: {id_delta:?} \nStateVectorBACK: {sv_back:?}, \nDeltaBACK {cached_delta_back:?}")
        };

        // Ensure both forwards and backwards match
        assert_eq!(receiver, receiver_back, "Not commutative {}", report());
        if let Some(delta) = cached_delta {
            receiver = receiver.merge(delta);
            assert_eq!(receiver, receiver_back, "Not idempotent {}", report());
        }
        return receiver;
    }

    #[test]
    fn test_init() {
        let v: TestShelf = Shelf::Value {
            value: Value::Bool(true),
            clock: clock(0),
        };
        let m: TestShelf = Shelf::Map {
            shelves: HashMap::new(),
            clock: 1.into(),
        };
    }

    #[test]
    fn test_clock() {
        let shelf = Shelf::Value {
            value: 1.into(),
            clock: clock(1),
        };
        let shelf2 = val(2, 2);
        let result = shelf.merge(shelf2);

        if let Shelf::Value { value, clock: c } = result {
            assert!(c == clock(2));
            assert_eq!(value, Value::Int(2))
        } else {
            panic!("Should be a value");
        };
    }
    #[test]
    fn test_object_override() {
        let shelf: TestShelf = shelf_map([("foo".to_owned(), val(1, 0))].into_iter(), 0);
        let y = val(2, 0);
        let shelf2 = val(2, 0);
        let shelf = merge(shelf, shelf2);

        if let Shelf::Map { .. } = shelf {
        } else {
            panic!("Expected map to override the integer value")
        }
    }
    #[test]
    fn test_vec_diff() {
        let shelf: TestShelf = array(vec![1], 0);
        let shelf2 = array(vec![2], 2);
        let shelf = merge(shelf2, shelf);

        if let Shelf::Value {
            value: Value::Array(list),
            ..
        } = shelf
        {
            if let Value::Int(n) = list[0] {
                assert_eq!(n, 2)
            } else {
                panic!("not an int {:?}", list[0]);
            }
        } else {
            panic!("Didn't find list: {}", shelf)
        }
    }
    #[test]
    fn test_recursive_diff() {
        let shelf = shelf_map([("a".to_owned(), val(1, 0))].into_iter(), 0);
        let shelf2 = shelf_map(
            [(
                "b".to_owned(),
                Shelf::Value {
                    value: 2.into(),
                    clock: ClientClock {
                        client_id: 1,
                        clock: 0,
                    },
                },
            )]
            .into_iter(),
            0,
        );

        let shelf = merge(shelf2, shelf);

        if let Shelf::Map { shelves, .. } = shelf {
            assert!(shelves.len() == 2)
        }
    }

    #[test]
    fn test_get() {
        let mut shelf: TestShelf = json!([{ "user": [{
            "mouse_position": [[0, 1], [0,0]],
            "cursor": [{"left": ["a",[0,0]], "right": ["b",[0,0]]},0]
        }, 0]  }, 0])
        .try_into()
        .unwrap();
        let res = shelf
            .get_mut("user")
            .unwrap()
            .get_mut("cursor")
            .unwrap()
            .get_mut("left")
            .unwrap();
        if let Shelf::Value {
            value: Value::String(s),
            ..
        } = res
        {
            assert_eq!(s, "a");
        } else {
            panic!("Unexpected value {:?}", res)
        }

        assert!(shelf.get("BOOM/goes/the/path").is_none())
    }

    #[test]

    fn test_adding_user() {
        let mut shelf1: TestShelf = json!([{"1": [{"mouse_position": [[1, 2], [0,2]]}, 2]}, 1])
            .try_into()
            .unwrap();
        let shelf2: TestShelf = json!([{"2": [{"mouse_position": [[3, 4], [1,1]]}, 1]}, 1])
            .try_into()
            .unwrap();

        let expected = json!([{
            "1": [{"mouse_position": [[1, 2], [0,2]]}, 2],
            "2": [{"mouse_position": [[3, 4], [1,1]]}, 1]
        }, 1])
        .try_into()
        .unwrap();
        let sv = shelf1.get_state_vector();
        let diff = shelf2.get_state_delta(&sv).unwrap();
        shelf1 = shelf1.merge(diff); // Mutate in place
        assert_eq!(shelf1, expected)
    }
    /// Uses examples in assets/shelf_tests.json to test merges.
    /// Ensures that both shelves arrive at the expected state.
    #[test]
    fn test_merges() {
        let data: JSON = serde_json::from_str(include_str!("../assets/shelf_tests.json")).unwrap();

        let tests = match data {
            JSON::Array(tests) => tests,
            _ => panic!("Must be array of tests"),
        };

        let tests = tests.into_iter().enumerate().map(|(i, test)| {
            let mut test = match test {
                JSON::Object(obj) => obj,
                val => panic!("{val:?} is not an object"),
            };
            let shelves: Vec<TestShelf> = match test.remove("shelves") {
                Some(JSON::Array(shelves)) => {
                    let possible_shelves: Result<_, _> =
                        shelves.into_iter().map(Shelf::try_from).collect();
                    possible_shelves.unwrap()
                }
                val => panic!("{val:?} cannot be turned into shelves"),
            };
            let expected = test
                .remove("expected")
                .map(Shelf::try_from)
                .and_then(|v| v.ok());
            let description = test
                .remove("description")
                .and_then(|v| match v {
                    JSON::String(s) => Some(format!("Test {}: {}", i + 1, s)),
                    _ => None,
                })
                .unwrap_or_default();

            (shelves, expected, description)
        });
        for (mut shelves, expected, description) in tests {
            for i in 0..shelves.len() {
                for j in (i + 1)..shelves.len() {
                    // Forwards
                    let mut receiver = shelves[i].clone();
                    let sender = shelves[j].clone();
                    let sv = receiver.get_state_vector();
                    let delta = sender.get_state_delta(&sv).unwrap();
                    let cached_delta = delta.clone();
                    receiver = receiver.merge(delta);

                    // Backwards
                    let mut receiver_back = shelves[j].clone();
                    let sender_back = shelves[i].clone();
                    let sv = receiver_back.get_state_vector();

                    if let Some(delta) = sender_back.get_state_delta(&sv) {
                        receiver_back = receiver_back.merge(delta)
                    }

                    // Ensure both forwards and backwards match
                    assert_eq!(receiver, receiver_back, "Not commutative\n {description}");

                    // Ensure duplicate application of deltas has no effect
                    receiver = receiver.merge(cached_delta);
                    assert_eq!(receiver, receiver_back, "Not idempotent\n {description}");
                    shelves[i] = receiver;
                    shelves[j] = receiver_back;
                }
                // Since the first CRDT has now received updates from all others, it should have the expected value.
                if let Some(expected) = expected.as_ref() {
                    assert_eq!(
                        &shelves[i], expected,
                        "Did not match expected\n {description}"
                    );
                }
            }
        }
    }

    #[test]
    /// Test setting an empty object over a full object and ensure propagation of erasure.
    fn test_empty_replacement() {
        let shelf_map_with_value = shelf_map([("value".to_owned(), val(1, 0))].into_iter(), 0);

        let empty_shelf_map = shelf_map([].into_iter(), 0);

        let shelf_with_value = val(1, 0);

        let shelves = vec![shelf_with_value, empty_shelf_map, shelf_map_with_value];
        // test all combinations
        for i in 0..shelves.len() {
            for j in (i + 1)..shelves.len() {
                let shelf1 = shelves[i].clone();
                let shelf2 = shelves[j].clone();
                let shelf2 = match shelf2 {
                    Shelf::Value { value, clock } => Shelf::Value {
                        value,
                        clock: clock.increment(1),
                    },
                    Shelf::Map { shelves, clock } => Shelf::Map {
                        shelves,
                        clock: clock.increment(),
                    },
                };

                let res = validate_crdt_properties(shelf2, shelf1);
            }
        }
    }

    #[test]
    /// Procedurally generates sets shelves and ensures that they all converge.
    fn test_generated_shelves() {
        let mut fuzzer = ShelfFuzzer {
            rng: StdRng::seed_from_u64(1),
            depth_range: 1..4,
            branch_range: 1..5,
            value_range: 0..20,
        };
        let num_tests: usize = 2000;
        for i in 0..num_tests {
            let mut shelf: TestShelf = Shelf::try_from(fuzzer.generate_json_shelf(1)).unwrap();
            let mut shelf2: TestShelf = Shelf::try_from(fuzzer.generate_json_shelf(2)).unwrap();
            shelf.garbage_collect();
            shelf2.garbage_collect();

            if shelf == shelf2 {
                continue;
            }

            // Forwards
            let mut receiver = shelf.clone();
            let sender = shelf2.clone();
            let sv = receiver.get_state_vector();

            let delta = sender.get_state_delta(&sv);
            let cached_delta = delta.clone();
            if let Some(delta) = delta {
                receiver = receiver.merge(delta);
            }

            // Backwards
            let mut receiver_back = shelf2.clone();
            let sender_back = shelf.clone();
            let sv_back = receiver_back.get_state_vector();
            let delta_back = sender_back.get_state_delta(&sv_back);
            let cached_delta_back = delta_back.clone();
            if let Some(delta) = delta_back {
                receiver_back = receiver_back.merge(delta);
            }
            let id_delta = cached_delta.clone();
            let report = || {
                format!("\nTEST {i}\nReceiver: {shelf}, \nSender {shelf2}, \nStateVector {sv:?}, \nDelta: {id_delta:?} \nStateVectorBACK: {sv_back:?}, \nDeltaBACK {cached_delta_back:?}")
            };

            // Ensure both forwards and backwards match
            assert_eq!(receiver, receiver_back, "Not commutative {}", report());
            if let Some(delta) = cached_delta {
                receiver = receiver.merge(delta);
                assert_eq!(receiver, receiver_back, "Not idempotent {}", report());
            }
        }
    }
}
