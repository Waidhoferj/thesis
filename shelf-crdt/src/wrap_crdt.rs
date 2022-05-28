use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value as JSON};

use crate::temporal::Temporal;
use crate::traits::{DeltaCRDT, Incrementable, Mergeable, TypeOrd};

use std::cmp::Ordering;
use std::fmt::{Display, write};
use std::{collections::HashMap, fmt::Debug};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Atomic {
    String(String),
    Int(isize),
    Float(f32),
    Bool(bool),
    Array(Vec<Atomic>),
}

impl TypeOrd for Atomic {
    fn type_cmp(&self, other: &Self) -> std::cmp::Ordering {
        Atomic::type_rank(self).cmp(&Atomic::type_rank(other))
    }
}
impl Atomic {
    #[inline(always)]
    fn type_rank(atom: &Atomic) -> u8 {
        match atom {
            Atomic::Array(_) => 5,
            Atomic::String(_) => 4,
            Atomic::Int(_) => 3,
            Atomic::Float(_) => 2,
            Atomic::Bool(_) => 1,
        }
    }
}

impl Display for Atomic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = serde_json::to_string(self).unwrap();
        write!(f,"{}",repr )
    }
}

impl PartialOrd for Atomic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.type_cmp(&other) {
            Ordering::Equal => match (self, other) {
                (Atomic::Bool(b1), Atomic::Bool(b2)) => b1.partial_cmp(b2),
                (Atomic::Int(v1), Atomic::Int(v2)) => v1.partial_cmp(v2),
                (Atomic::Float(v1), Atomic::Float(v2)) => v1.partial_cmp(v2),
                (Atomic::String(v1), Atomic::String(v2)) => v1.partial_cmp(v2),
                (Atomic::Array(v1), Atomic::Array(v2)) => v1.partial_cmp(v2),
                _ => unreachable!("Should be of the same type"),
            },
            ord => Some(ord),
        }
    }
}

impl TryFrom<JSON> for Atomic {
    type Error = String;
    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        match json {
            JSON::Bool(b) => Ok(Atomic::Bool(b)),
            JSON::Number(n) => Ok(Atomic::Int(n.as_i64().unwrap() as isize)),
            JSON::String(s) => Ok(Atomic::String(s)),
            JSON::Array(a) => {
                let array: Result<Vec<Atomic>, String> =
                    a.into_iter().map(|json_val| json_val.try_into()).collect();
                Ok(Atomic::Array(array?))
            }
            _ => Err("Should not be building with null or object values.".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Value {
    Atomic(Atomic),
    ShelfMap(HashMap<String, Shelf>),
}

impl Value {
    /// Returns the ordering of types, with higher values having more precedence than lower values.
    #[inline(always)]
    fn type_rank(value: &Value) -> u8 {
        match value {
            Value::Atomic(_) => 1,
            Value::ShelfMap(_) => 2,
        }
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = serde_json::to_string(self).unwrap();
        write!(f,"{}",repr )
    }
}

impl TypeOrd for Value {
    fn type_cmp(&self, other: &Self) -> std::cmp::Ordering {
        Value::type_rank(self).cmp(&Value::type_rank(other))
    }
}

impl TryFrom<JSON> for Value {
    type Error = String;

    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        match json {
            JSON::Object(obj) => {
                let mut shelves: HashMap<String, Shelf> = HashMap::new();
                for (k, v) in obj {
                    shelves.insert(k, v.try_into()?);
                }
                Ok(Value::ShelfMap(shelves))
            }
            val => Ok(Value::Atomic(val.try_into()?)),
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum StateVector {
    Node(HashMap<String, StateVector>, usize),
    Leaf(usize),
}

impl StateVector {
    fn set_clock(&mut self, val: usize) {
        match self {
            StateVector::Node(children, clock) => *clock = val,
            StateVector::Leaf(clock) => *clock = val,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Shelf {
    content: Option<Value>,
    clock: usize,
}

impl Shelf {
    /// Creates a new shelf around the value with a clock value of 0
    pub fn new(value: Value) -> Self {
        return Shelf {
            content: Some(value),
            clock: 0,
        };
    }
}

impl Display for Shelf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = serde_json::to_string(self).unwrap();
        write!(f,"{}",repr )
    }
}

impl Default for Shelf {
    fn default() -> Self {
        Shelf {
            content: None,
            clock: 0,
        }
    }
}

impl TryFrom<JSON> for Shelf {
    type Error = String;

    fn try_from(value: JSON) -> Result<Self, Self::Error> {
        match value {
            JSON::Array(mut array) => {
                if array.len() != 2 {
                    return Err("Array did not have 2 dimensions".to_string());
                }
                match (array.remove(0), array.remove(0)) {
                    (val, JSON::Number(clock)) => {
                        let clock = clock
                            .as_i64()
                            .ok_or("Could not convert clock into int".to_string())?
                            as usize;
                        let shelf = Shelf {
                            content: Some(val.try_into()?),
                            clock,
                        };
                        Ok(shelf)
                    }
                    _ => Err("Could not find clock in JSON".to_string()),
                }
            }
            val => Err(format!("Could not covert JSON into a shelf: {:?}", val)),
        }
    }
}

impl Mergeable<Self> for Shelf {
    /// Merges another shelf into the current one, returning the resulting union.
    fn merge(&mut self, mut other: Self) {
        let this_content = self.content.take();
        let other_content = other.content.take();
        let clock_order = Ord::cmp(&self.clock, &other.clock);
        // Merging two shelf maps

        match (this_content, other_content, clock_order) {
            (Some(Value::ShelfMap(mut this_value)), Some(Value::ShelfMap(other_value)), _) => {
                if self.clock < other.clock {
                    self.clock = other.clock;
                    this_value.retain(|_, shelf| shelf.clock >= other.clock);
                }

                for (key, val) in other_value.into_iter() {
                    if let Some(sub_shelf) = this_value.get_mut(&key) {
                        sub_shelf.merge(val);
                    } else {
                        this_value.insert(key, val);
                    }
                }
                self.content = Some(Value::ShelfMap(this_value));
                return;
            }
            (_, other_content, Ordering::Less) => {
                self.content = other_content;
                self.clock = other.clock;
            }, // Update is greater so take on that value
            (_, _, Ordering::Greater) => (),         // Self is greater so no need to update
            (Some(Value::Atomic(this_atom)), Some(Value::Atomic(other_atom)), Ordering::Equal) => {
                    let atom = if this_atom > other_atom {
                        this_atom
                    } else {
                        other_atom
                    };
                    self.content = Some(Value::Atomic(atom.clone()));
            }
            (Some(this_val), Some(other_val), Ordering::Equal) if Ordering::Less == this_val.type_cmp(&other_val) => self.content = Some(other_val),
            (this_content, other_content, Ordering::Equal) => {
                self.content = this_content.or(other_content)
            }
        };
    }
}

impl DeltaCRDT for Shelf {
    type Delta = Shelf;
    type StateVector = StateVector;
    fn get_state_vector(&self) -> StateVector {
        let children = self.content.as_ref().and_then(|value| match value {
            Value::Atomic(_) => None,
            Value::ShelfMap(shelf_map) => Some(
                shelf_map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.get_state_vector()))
                    .collect(),
            ),
        });

        if let Some(children) = children {
            StateVector::Node(children, self.clock)
        } else {
            StateVector::Leaf(self.clock)
        }
    }
    fn get_state_delta(&self, state_vector: &StateVector) -> Option<Self::Delta> {
        match state_vector {
            StateVector::Leaf(clock) if self.clock >= *clock => Some(self.clone()),
            StateVector::Node(sv_children, clock) => match self.content.as_ref()? {
                Value::Atomic(_) if self.clock > *clock => Some(self.clone()),
                Value::ShelfMap(shelf_map) if self.clock < *clock => {
                    let reduced_map = shelf_map.iter().filter(|(_, v)| v.clock >= *clock);
                    let updated_shelf_map: HashMap<String, Shelf> = reduced_map
                        .filter_map(|(k, v)| {
                            let delta = sv_children
                                .get(k)
                                .and_then(|sv_child| v.get_state_delta(sv_child))
                                .or_else(|| Some(v.clone()));
                            Some((k.clone(), delta?))
                        })
                        .collect();
                    let has_elements = !updated_shelf_map.is_empty();
                    has_elements.then(|| Shelf {
                        content: Some(Value::ShelfMap(updated_shelf_map)),
                        clock: self.clock,
                    })
                }
                Value::ShelfMap(shelf_map) => {
                    let updated_shelf_map = shelf_map
                        .iter()
                        .filter_map(|(k, v)| {
                            let delta = sv_children
                                .get(k)
                                .and_then(|sv_child| v.get_state_delta(sv_child))
                                .or_else(|| Some(v.clone()));
                            Some((k.clone(), delta?))
                        })
                        .collect();

                    Some(Shelf {
                        content: Some(Value::ShelfMap(updated_shelf_map)),
                        clock: self.clock,
                    })
                }
                _ => None,
            },
            _ => None,
        }
    }
}

impl Shelf {
    /// Determines whether there are more shelves nested inside this one.
    pub fn contains_shelves(&self) -> bool {
        match self.content {
            Some(Value::ShelfMap(_)) => true,
            _ => false,
        }
    }
    ///  Gets a Value out of the Shelf
    pub fn get(&self, key: &str) -> Option<&Shelf> {
        match self.content.as_ref()? {
            Value::ShelfMap(shelf_map) => shelf_map.get(key),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Shelf> {
        match self.content.as_mut()? {
            Value::ShelfMap(shelf_map) => shelf_map.get_mut(key),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::wrap_crdt::*;

    fn val(a: Atomic) -> Option<Value> {
        Some(Value::Atomic(a))
    }
    fn merge(branch: Shelf, mut main: Shelf) -> Shelf {
        let sv = main.get_state_vector();
        let delta = branch.get_state_delta(&sv).unwrap();
        main.merge(delta);
        main
    }

    fn array(elements: Vec<isize>) -> Option<Value> {
        val(Atomic::Array(
            elements.into_iter().map(|el| Atomic::Int(el)).collect(),
        ))
    }

    #[test]
    fn test_init() {
        Shelf {
            content: val(Atomic::Bool(true)),
            clock: 1,
        };
    }

    #[test]
    fn test_clock() {
        let mut shelf = Shelf {
            content: val(Atomic::Int(1)),
            clock: 1,
        };
        let y = val(Atomic::Int(2));
        let shelf2 = Shelf {
            content: y,
            clock: 2,
        };
        shelf.merge(shelf2);

        assert!(shelf.clock == 2);
    }
    #[test]
    fn test_object_override() {
        let x = Value::ShelfMap(HashMap::new());
        let shelf: Shelf = Shelf::new(x);
        let y = val(Atomic::Int(2)).unwrap();
        let shelf2 = Shelf::new(y);
        let shelf = merge(shelf, shelf2);

        if let Some(Value::ShelfMap(_)) = shelf.content {
        } else {
            panic!("Expected map to override the atomic value")
        }
    }
    #[test]
    fn test_vec_diff() {
        let shelf: Shelf = Shelf {
            content: array(vec![1]),
            clock: 1,
        };
        let y = array(vec![2]);
        let shelf2 = Shelf {
            content: y,
            clock: 2,
        };
        let shelf = merge(shelf2, shelf);

        if let Some(Value::Atomic(Atomic::Array(list))) = shelf.content {
            if let Atomic::Int(n) = list[0] {
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
        let sub_shelf = Shelf::new(val(Atomic::Int(1)).unwrap());

        let mut dict = HashMap::new();
        dict.insert("a".to_string(), sub_shelf);
        let shelf: Shelf = Shelf::new(Value::ShelfMap(dict));

        let sub_shelf2 = Shelf::new(val(Atomic::Int(2)).unwrap());

        let mut dict2 = HashMap::new();
        dict2.insert("b".to_string(), sub_shelf2);
        let shelf2 = Shelf::new(Value::ShelfMap(dict2));

        let shelf = merge(shelf2, shelf);

        if let Some(Value::ShelfMap(m)) = shelf.content {
            assert!(m.len() == 2)
        }
    }

    #[test]
    fn test_state_vector() {
        let shelf: Shelf = json!([{ 
            "user": [{
            "mouse_position": [[0, 1], 0],
            "cursor": [{"left": ["a", 0], "right": ["b", 0]}, 0]
        },0]  
    }, 0])
        .try_into()
        .unwrap();

        let state_vector = shelf.get_state_vector();
        match state_vector {
            StateVector::Node(map, clock) => {
                assert_eq!(clock, 0);
                match &map["user"] {
                    StateVector::Node(map, clock) => {
                        assert_eq!(*clock, 0);
                        match &map["mouse_position"] {
                            StateVector::Node(_, _) => panic!("Array should not be a node"),
                            StateVector::Leaf(i) => assert_eq!(i, &0),
                        };

                        match &map["cursor"] {
                            StateVector::Node(map, clock) => {
                                assert_eq!(*clock, 0);
                                if let StateVector::Leaf(i) = &map["left"] {
                                    assert_eq!(i, &0);
                                } else {
                                    panic!("left isn't a leaf");
                                }

                                if let StateVector::Leaf(i) = &map["right"] {
                                    assert_eq!(i, &0);
                                } else {
                                    panic!("right isn't a leaf");
                                }
                            }
                            StateVector::Leaf(_) => panic!("Should be a cursor map"),
                        }
                    }
                    StateVector::Leaf(_) => panic!("User should be an object"),
                }
            }
            StateVector::Leaf(_) => panic!("Top level should be a map"),
        }
    }
    #[test]
    fn test_delta_update() {
        let shelf1: Shelf = json!([{
            "user1": [{
                "username": ["waidhoferj", 0]
            }, 0]
        }, 0])
        .try_into()
        .unwrap();
        // number ,string, List*, Map<string, Shelf>
        let mut shelf2: Shelf = json!([{ "user2": [{"username": ["jwaidhof", 0]}, 0] }, 0])
            .try_into()
            .unwrap();
        let state_vec = shelf2.get_state_vector();
        let diff = shelf1.get_state_delta(&state_vec).unwrap();

        shelf2.merge(diff);

        let expected: Shelf = json!([{
            "user1": [{
                "username": ["waidhoferj",0]
            }, 0],
            "user2": [{"username": ["jwaidhof",0]}, 0]
        }, 0])
        .try_into()
        .unwrap();

        assert_eq!(shelf2, expected);

        let shelf1: Shelf = json!([{
            "user1": [{
                "username": ["waidhoferj", 0]
            }, 0]
        }, 0])
        .try_into()
        .unwrap();

        let shelf2: Shelf = json!([{
            "user1": [{
                "username": ["waidhoferj", 0]
            }, 0]
        }, 0])
        .try_into()
        .unwrap();
    }

    #[test]
    fn test_get() {
        let mut shelf: Shelf = json!([{ "user": [{
            "mouse_position": [[0, 1], 0],
            "cursor": [{"left": ["a",0], "right": ["b",0]},0]
        }, 0]  }, 0])
        .try_into()
        .unwrap();
        let res: Option<Value> =
            (|| shelf.get_mut("user")?.get_mut("cursor")?.get_mut("left").and_then(|s| s.content.take()))();
        if let Some(Value::Atomic(Atomic::String(s))) = res {
            assert_eq!(s, "a");
        } else {
            panic!("Unexpected value {:?}", res)
        }

        assert!(shelf.get("BOOM/goes/the/path").is_none())
    }

    #[test]

    fn test_adding_user() {
        let mut shelf1: Shelf = json!([{"1": [{"mouse_position": [[1, 2], 2]}, 2]}, 1])
            .try_into()
            .unwrap();
        let shelf2: Shelf = json!([{"2": [{"mouse_position": [[3, 4], 1]}, 1]}, 1])
            .try_into()
            .unwrap();

        let expected = json!([{
            "1": [{"mouse_position": [[1, 2], 2]}, 2],
            "2": [{"mouse_position": [[3, 4], 1]}, 1]
        }, 1])
        .try_into()
        .unwrap();
        let sv = shelf1.get_state_vector();
        let diff = shelf2.get_state_delta(&sv).unwrap();
        shelf1.merge(diff); // Mutate in place
        assert_eq!(shelf1, expected)
    }
}
