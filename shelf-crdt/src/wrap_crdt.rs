use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value as JSON};

use crate::temporal::Temporal;
use crate::{DeltaCRDT, Incrementable, Mergeable, TypeOrd};

use std::cmp::Ordering;
use std::fmt::Display;
use std::ops::Deref;
use std::{collections::HashMap, fmt::Debug};

// TODO:
//  - Add compression
//  - Add signing
//  - pubic/private keys
//  - Port to scripting language

// macro_rules! collection {
//     ($($k:expr => $v:expr),* $(,)?) => {{
//         core::convert::From::from([$(($k, $v),)*])
//     }}
// }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Atomic {
    String(String),
    Int(isize),
    Float(f32),
    Bool(bool),
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
            Atomic::String(_) => 1,
            Atomic::Int(_) => 2,
            Atomic::Float(_) => 3,
            Atomic::Bool(_) => 3,
        }
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
                _ => unreachable!("Should be of the same type"),
            },
            ord => Some(ord),
        }
    }
}

impl From<isize> for Atomic {
    fn from(i: isize) -> Self {
        Atomic::Int(i)
    }
}

impl From<f32> for Atomic {
    fn from(f: f32) -> Self {
        Atomic::Float(f)
    }
}

impl From<String> for Atomic {
    fn from(s: String) -> Self {
        Atomic::String(s)
    }
}

impl From<bool> for Atomic {
    fn from(b: bool) -> Self {
        Atomic::Bool(b)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Atomic(Atomic),
    List(Vec<Atomic>),
    Map(HashMap<String, Shelf<Value, Temporal>>),
}

impl<T: Into<Atomic>> From<T> for Value {
    fn from(v: T) -> Self {
        return Value::Atomic(v.into());
    }
}

impl From<Value> for JSON {
    fn from(v: Value) -> Self {
        match v {
            Value::Atomic(a) => serde_json::to_value(a).unwrap(),
            Value::List(l) => serde_json::to_value(l).unwrap(),
            Value::Map(m) => {
                let val_map: HashMap<String, JSON> =
                    m.into_iter().map(|(k, v)| (k, v.json_values())).collect();
                serde_json::to_value(val_map).unwrap()
            }
        }
    }
}

impl Value {
    /// Returns the ordering of types, with lower values having more precedence than higher values.
    #[inline(always)]
    fn type_rank(value: &Value) -> u8 {
        match value {
            Value::Atomic(_) => 1,
            Value::List(_) => 2,
            Value::Map(_) => 3,
        }
    }
}

impl TypeOrd for Value {
    fn type_cmp(&self, other: &Self) -> std::cmp::Ordering {
        Value::type_rank(self).cmp(&Value::type_rank(other))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.type_cmp(other) == Ordering::Equal
            && match (self, other) {
                (Value::Atomic(v1), Value::Atomic(v2)) => v1 == v2,
                (Value::List(a1), Value::List(a2)) => {
                    for (v1, v2) in a1.iter().zip(a2.iter()) {
                        if v1 != v2 {
                            return false;
                        }
                    }
                    true
                }
                (Value::Map(_), Value::Map(_)) => true,
                _ => unreachable!("Only matching types should exist at this point."),
            }
    }
}
// TODO: Should we look at value?
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.type_cmp(other))
    }
}

impl Mergeable for Value {
    fn merge(self, other: Self) -> Self {
        let x = match (self, other) {
            // Merge pairs of maps based on their keys
            (Value::Map(mut my_value), Value::Map(update_value)) => {
                for (key, val) in update_value.into_iter() {
                    if let Some(sub_shelf) = my_value.remove(&key) {
                        my_value.insert(key, sub_shelf.merge(val));
                    } else {
                        my_value.insert(key, val);
                    }
                }
                Value::Map(my_value)
            }

            (this, other) => match this.partial_cmp(&other) {
                Some(Ordering::Greater | Ordering::Equal) => this,
                Some(Ordering::Less) => other,
                _ => panic!("Compare should be deterministic at this point."),
            },
        };
        return x;
    }
}

impl DeltaCRDT<Value, Option<StateVector>> for Value {
    fn get_state_vector(&self) -> Option<StateVector> {
        match self {
            Value::Map(shelves) => {
                let clocks = shelves
                    .iter()
                    .map(|(k, shelf)| (k.clone(), shelf.get_state_vector()))
                    .collect();
                Some(StateVector::Node(clocks, Temporal::LamportTS(0))) // TODO fix this clock hack, needs to be replaced in parent :(
            }
            _ => None,
        }
    }

    fn get_state_delta(&self, sv: &Option<StateVector>) -> Option<Value> {
        match (self, sv) {
            (Value::Map(map), Some(StateVector::Node(sender_clocks, _))) => {
                let mut delta = HashMap::new();
                for (k, current_shelf) in map.iter() {
                    if let Some(sv) = sender_clocks.get(k) {
                        if let Some(delta_state) = current_shelf.get_state_delta(sv.into()) {
                            delta.insert(k.clone(), delta_state);
                        }
                    } else {
                        delta.insert(k.clone(), current_shelf.clone());
                    }
                }
                Some(Value::Map(delta))
            }
            (Value::Map(map), Some(StateVector::Leaf(_))) => Some(Value::Map(map.clone())),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StateVector {
    Node(HashMap<String, StateVector>, Temporal),
    Leaf(Temporal),
}

// TODO Use something like a merkle tree
impl From<&Shelf<Value, Temporal>> for StateVector {
    fn from(shelf: &Shelf<Value, Temporal>) -> Self {
        let ts = shelf.clock.clone();
        let children: Option<HashMap<String, StateVector>> = match &shelf.value {
            Some(Value::Map(m)) => Some(m.iter().map(|(k, v)| (k.clone(), v.into())).collect()),
            _ => None,
        };
        if let Some(children) = children {
            StateVector::Node(children, ts)
        } else {
            StateVector::Leaf(ts)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shelf<V: PartialOrd, T: Incrementable + PartialOrd = usize> {
    value: Option<V>,
    clock: T,
}

impl<V, T> Shelf<V, T>
where
    V: PartialOrd,
    T: Incrementable + PartialOrd + Default,
{
    /// Creates a new shelf around the value with a clock value of 0
    pub fn new(value: V) -> Self {
        return Shelf {
            value: Some(value),
            clock: T::default(),
        };
    }
}

impl<V, T: Default> Default for Shelf<V, T>
where
    V: PartialOrd,
    T: Incrementable + PartialOrd,
{
    fn default() -> Self {
        Shelf {
            value: None,
            clock: T::default(),
        }
    }
}

impl<V, T> Mergeable<Self> for Shelf<V, T>
where
    V: PartialOrd + Mergeable,
    T: Incrementable + PartialOrd + Mergeable,
{
    /// Merges another shelf into the current one, returning the resulting union.
    fn merge(mut self, other: Self) -> Self {
        match PartialOrd::partial_cmp(&self.clock, &other.clock) {
            Some(Ordering::Greater) => self,
            Some(Ordering::Less) => other,
            Some(Ordering::Equal) | None => {
                self.value = match (self.value, other.value) {
                    (Some(value), Some(other_value)) => Some(value.merge(other_value)),
                    (Some(value), None) => Some(value),
                    (None, Some(value)) => Some(value),
                    (None, None) => None,
                };
                self.clock = self.clock.merge(other.clock);
                self
            }
        }
    }
}

impl DeltaCRDT<Shelf<Value, Temporal>, StateVector> for Shelf<Value, Temporal> {
    fn get_state_vector(&self) -> StateVector {
        let children = self
            .value
            .as_ref()
            .and_then(|value| value.get_state_vector());
        if let Some(child_state) = children {
            match child_state {
                StateVector::Node(map, _) => StateVector::Node(map, self.clock.clone()),
                StateVector::Leaf(_) => {
                    unreachable!("If children are found, there should be multiple in a HashMap.")
                }
            }
        } else {
            StateVector::Leaf(self.clock.clone())
        }
    }
    fn get_state_delta(&self, state_vector: &StateVector) -> Option<Shelf<Value, Temporal>> {
        match state_vector {
            StateVector::Leaf(clock) => match self.clock.partial_cmp(clock) {
                Some(Ordering::Greater) => Some(self.clone()),
                Some(_) => None,
                None => panic!(
                    "Should be LamportTS on Leaves, {:?}, {:?}",
                    self.clock, clock
                ),
            },
            StateVector::Node(_, clock) => match self.clock.partial_cmp(clock) {
                Some(Ordering::Greater) => Some(self.clone()),
                None | Some(Ordering::Equal) => self
                    .value
                    .as_ref()
                    .and_then(|value| value.get_state_delta(&Some(state_vector.clone()))) // Inefficient
                    .map(|delta| {
                        let updated_clock = self.clock.clone().merge(clock.clone());

                        Shelf {
                            value: Some(delta),
                            clock: updated_clock,
                        }
                    }),
                _ => None,
            },
        }
    }
}

impl Shelf<Value, Temporal> {
    ///  Gets a Value out of the Shelf
    ///  Path should be formatted like a file path ex. path/to/shelf
    ///  TODO: Make this a bit more ergonomic
    pub fn get(&self, path: &str) -> Option<&Value> {
        let path = path.split("/");
        let mut cur = self.value.as_ref();
        for prop in path {
            match cur {
                Some(Value::Map(map)) => {
                    cur = map
                        .get(prop)
                        .as_ref()
                        .and_then(|shelf| shelf.value.as_ref());
                }
                Some(_) => return None,
                None => return None,
            }
        }
        cur
    }

    /// Updates a Value in the Shelf
    /// Path should be formatted like a file path ex. path/to/shelf
    /// TODO: Make this a bit more ergonomic
    pub fn update(&mut self, path: &str, value: Value) -> Result<Option<Value>, String> {
        let path = path.split("/");
        let mut cur = Some(self);
        for prop in path {
            match cur.and_then(|shelf| shelf.value.as_mut()) {
                Some(Value::Map(map)) => {
                    cur = map.get_mut(prop);
                }
                Some(_) => return Err("Path DNE".to_string()),
                None => return Err("Path DNE".to_string()),
            }
        }
        let shelf = cur.unwrap();
        let old_val = shelf.value.replace(value);
        shelf.clock.increment();
        Ok(old_val)
    }

    fn wrap(json: JSON, clock: Temporal) -> Self {
        let value: Value = match json {
            JSON::Bool(b) => b.into(),
            JSON::Number(n) => (n.as_i64().unwrap() as isize).into(),
            JSON::String(s) => s.into(),
            JSON::Array(a) => Value::List(
                a.into_iter()
                    .map(|v| serde_json::from_value(v).unwrap())
                    .collect(),
            ),
            JSON::Object(o) => {
                let map: HashMap<String, _> = o
                    .into_iter()
                    .map(|(k, v)| (k, Shelf::wrap(v, Temporal::LamportTS(0))))
                    .collect();
                Value::Map(map)
            }
            _ => panic!("No corresponding type"),
        };

        Shelf {
            value: Some(value),
            clock,
        }
    }

    /// Recursively wraps a JSON data structure in the Shelf CRDT.
    pub fn from_template(json: JSON, user_id: String) -> Self {
        Shelf::wrap(
            json,
            Temporal::VectorClock {
                clocks: HashMap::from([(user_id.clone(), 0)]),
                user_id,
            },
        )
    }
    /// Extracts data in shelf as JSON, erasing the wrapped shelf data structure.
    pub fn json_values(self) -> JSON {
        if let Some(value) = self.value {
            value.into()
        } else {
            JSON::Null
        }
    }
}

impl<V: PartialOrd, T: Incrementable + PartialOrd> Deref for Shelf<V, T> {
    type Target = Option<V>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, V, T> Display for Shelf<V, T>
where
    V: Display + PartialOrd + Serialize + Deserialize<'a>,
    T: Display + Incrementable + PartialOrd + Serialize + Deserialize<'a>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = serde_json::to_string(self).unwrap();
        write!(f, "{}", &repr)
    }
}

#[cfg(test)]
mod tests {

    use crate::wrap_crdt::*;

    fn ts(num: u32) -> Temporal {
        Temporal::LamportTS(num)
    }

    fn vc(uid: String) -> Temporal {
        Temporal::VectorClock {
            clocks: HashMap::from([(uid.clone(), 0)]),
            user_id: uid,
        }
    }
    #[test]
    fn test_temporal_partial_cmp() {
        let u1 = vc("foo".to_string());
        let u2 = vc("bar".to_string());
        let cmp = u1.partial_cmp(&u2);
        println!("{:?}", cmp);
        assert_eq!(cmp, None)
    }

    #[test]
    fn test_init() {
        let x: Option<Value> = Some(1.into());
        let _ = Shelf {
            value: x,
            clock: Temporal::LamportTS(1),
        };
    }

    #[test]
    fn test_clock() {
        let x: Option<Value> = Some(1.into());
        let shelf = Shelf {
            value: x,
            clock: ts(1),
        };
        let y = Some(2.into());
        let shelf2 = Shelf {
            value: y,
            clock: ts(2),
        };
        let shelf = shelf.merge(shelf2);

        assert!(shelf.clock == ts(2));
    }
    #[test]
    fn test_object_override() {
        let x = Value::Map(HashMap::new());
        let shelf: Shelf<Value> = Shelf::new(x);
        let y = Value::Atomic(2.into());
        let shelf2 = Shelf::new(y);
        let result = shelf2.merge(shelf);

        if let Some(Value::Map(_)) = result.value {
        } else {
            panic!("Expected map to override the atomic value")
        }
    }
    #[test]
    fn test_vec_diff() {
        let x = Value::List(vec![1.into()]);
        let shelf = Shelf {
            value: Some(x),
            clock: ts(1),
        };
        let y = Value::List(vec![2.into()]);
        let shelf2 = Shelf {
            value: Some(y),
            clock: ts(2),
        };

        let shelf = shelf.merge(shelf2);

        if let Some(Value::List(list)) = shelf.value {
            if let Atomic::Int(n) = list[0] {
                assert_eq!(n, 2)
            }
        } else {
            panic!()
        }
    }
    #[test]
    fn test_recursive_diff() {
        let sub_shelf = Shelf::new(1.into());

        let mut dict = HashMap::new();
        dict.insert("a".to_string(), sub_shelf);
        let shelf: Shelf<Value> = Shelf::new(Value::Map(dict));

        let sub_shelf2 = Shelf::new(2.into());

        let mut dict2 = HashMap::new();
        dict2.insert("b".to_string(), sub_shelf2);
        let shelf2 = Shelf::new(Value::Map(dict2));

        let shelf = shelf.merge(shelf2);

        if let Some(Value::Map(m)) = shelf.value {
            assert!(m.len() == 2)
        }
    }

    #[test]
    fn test_state_vector() {
        let shelf = Shelf::from_template(
            json!({ "user": {
            "mouse_position": [0, 1],
            "cursor": {"left": "a", "right": "b"}
        }  }),
            1_u8.to_string(),
        );

        let state_vector = shelf.get_state_vector();
        match state_vector {
            StateVector::Node(map, clock) => {
                assert_eq!(clock, vc("1".into()));
                match &map["user"] {
                    StateVector::Node(map, clock) => {
                        assert_eq!(*clock, ts(0));
                        match &map["mouse_position"] {
                            StateVector::Node(_, _) => panic!("Array should not be a node"),
                            StateVector::Leaf(i) => assert_eq!(i, &ts(0)),
                        };

                        match &map["cursor"] {
                            StateVector::Node(map, clock) => {
                                assert_eq!(*clock, ts(0));
                                if let StateVector::Leaf(i) = &map["left"] {
                                    assert_eq!(i, &ts(0));
                                } else {
                                    panic!("left isn't a leaf");
                                }

                                if let StateVector::Leaf(i) = &map["right"] {
                                    assert_eq!(i, &ts(0));
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
        let shelf1 = Shelf::from_template(
            json!({
                "user1": {
                    "username": "waidhoferj"
                }
            }),
            1_u8.to_string(),
        );
        // number ,string, List*, Map<string, Shelf>
        let mut shelf2 = Shelf::from_template(
            json!({ "user2": {"username": "jwaidhof"} }),
            2_u8.to_string(),
        );
        let state_vec = shelf2.get_state_vector();
        let diff = shelf1.get_state_delta(&state_vec).unwrap();

        let resulting_shelf = shelf2.merge(diff);

        let res: JSON = resulting_shelf.json_values();
        let expected = json!({
            "user1": {
                "username": "waidhoferj"
            },
            "user2": {"username": "jwaidhof"}
        });

        assert_eq!(res, expected);

        let shelf1 = Shelf::from_template(
            json!({
                "user1": {
                    "username": "waidhoferj"
                }
            }),
            1_u8.to_string(),
        );

        let shelf2 = Shelf::from_template(
            json!({
                "user1": {
                    "username": "waidhoferj"
                }
            }),
            2_u8.to_string(),
        );
    }

    #[test]
    fn test_get() {
        let shelf = Shelf::from_template(
            json!({ "user": {
            "mouse_position": [0, 1],
            "cursor": {"left": "a", "right": "b"}
        }  }),
            1_u8.to_string(),
        );
        let res = shelf.get("user/cursor/left");
        if let Some(Value::Atomic(Atomic::String(s))) = res {
            assert_eq!(s, "a");
        } else {
            panic!("Unexpected value {:?}", res)
        }

        assert!(shelf.get("BOOM/goes/the/path").is_none())
    }

    #[test]
    fn test_update() {
        let mut shelf = Shelf::from_template(
            json!({ "user": {
            "mouse_position": [0, 1],
            "cursor": {"left": "a", "right": "b"}
        }  }),
            1_u8.to_string(),
        );
        let new_position = Value::List(vec![1.into(), 2.into()]);
        let mouse_position = "user/mouse_position";
        shelf
            .update(mouse_position, new_position.clone())
            .expect("Couldn't update");
        let output = shelf.get(mouse_position);

        if let (Some(Value::List(actual)), Value::List(expected)) = (output, new_position) {
            let actual = serde_json::to_value(actual).unwrap();
            let expected = serde_json::to_value(expected).unwrap();
            assert_eq!(actual, expected);
        } else {
            panic!("Didn't find lists");
        }
    }

    #[test]
    fn test_encode() {}

    #[test]

    fn test_adding_user() {
        let mut shelf1 =
            Shelf::from_template(json!({"1": {"mouse_position": [1, 2]}}), 1_u8.to_string());
        let shelf2 =
            Shelf::from_template(json!({"2": {"mouse_position": [3, 4]}}), 2_u8.to_string());

        let expected_json = json!({
            "1": {"mouse_position": [1, 2]},
            "2": {"mouse_position": [3, 4]}});
        let sv = shelf1.get_state_vector();
        let diff = shelf2.get_state_delta(&sv).unwrap();
        let resulting_shelf = shelf1.merge(diff);
        let resulting_json: JSON = resulting_shelf.json_values();
        assert_eq!(resulting_json, expected_json)
    }
}
