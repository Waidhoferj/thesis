use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value as JSON};
use rand::{self, Rng};
use crate::traits::{DeltaCRDT, Mergeable};

use std::cmp::Ordering;
use std::fmt::{Display};
use std::{collections::HashMap, fmt::Debug};
use std::clone::Clone;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(isize),
    Float(f32),
    Bool(bool),
    Array(Vec<Value>),
    Null
}

impl Value {
    #[inline(always)]
    fn type_rank(value: &Value) -> u8 {
        match value {
            Value::Array(_) => 5,
            Value::String(_) => 4,
            Value::Int(_) => 3,
            Value::Float(_) => 2,
            Value::Bool(_) => 1,
            Value::Null => 0
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr =  match self {
            Value::String(s) => format!("\"{}\"",s),
            Value::Int(i) => format!("{}",i),
            Value::Float(f) => format!("{}",f),
            Value::Bool(b) => format!("{}",b),
            Value::Array(a) => format!("{:?}",a),
            Value::Null => "null".to_string()
        };
        write!(f, "{repr}")
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return std::fmt::Display::fmt(&self, f)
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match Value::type_rank(self).cmp(&Value::type_rank(other)) {
            Ordering::Equal => match (self, other) {
                (Value::Bool(b1), Value::Bool(b2)) => b1.partial_cmp(b2),
                (Value::Int(v1), Value::Int(v2)) => v1.partial_cmp(v2),
                (Value::Float(v1), Value::Float(v2)) => v1.partial_cmp(v2),
                (Value::String(v1), Value::String(v2)) => v1.partial_cmp(v2),
                (Value::Array(v1), Value::Array(v2)) => v1.partial_cmp(v2),
                (Value::Null, Value::Null) => Some(Ordering::Equal),
                _ => unreachable!("If type ranks match, they must be the same type."),
            },
            ord => Some(ord),
        }
    }
}

impl TryFrom<JSON> for Value {
    type Error = String;
    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        match json {
            JSON::Bool(b) => Ok(Value::Bool(b)),
            JSON::Number(n) if n.is_i64() => Ok(Value::Int(n.as_i64().unwrap() as isize)),
            JSON::Number(n) if n.is_f64() => Ok(Value::Float(n.as_f64().unwrap() as f32)),
            JSON::String(s) => Ok(Value::String(s)),
            JSON::Array(a) => {
                let array: Result<Vec<Value>, String> =
                    a.into_iter().map(|json_val| json_val.try_into()).collect();
                Ok(Value::Array(array?))
            }
            JSON::Null => Ok(Value::Null),
            _ => Err("Should not be building with null or object values.".to_string()),
        }
    }
}

impl From<Value> for JSON {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => json!(s),
            Value::Int(i) => json!(i),
            Value::Float(f) => json!(f),
            Value::Bool(b) => json!(b),
            Value::Array(a) => {
                let arr: Vec<JSON> = a.into_iter().map(JSON::from).collect();
                json!(arr)
            }
            Value::Null => JSON::Null
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum ShelfContent<T: PartialOrd + Clone> {
    Value(T),
    ShelfMap(HashMap<String, Shelf<T>>),
}


impl ShelfContent<Value> {

    pub fn from_json_values(json: JSON, client_id: usize) -> Result<Self, String> {
        match json {
            JSON::Object(obj) => {
                let mut shelves: HashMap<String, Shelf<Value>> = HashMap::new();
                for (k, v) in obj {
                    shelves.insert(k, Shelf::from_json_values(v, client_id)?);
                }
                Ok(ShelfContent::ShelfMap(shelves))
            }
            val => Ok(ShelfContent::Value(val.try_into()?)),
        }
    }

    pub fn to_json_values(self) -> JSON {
        match self {
            ShelfContent::Value(val) => val.into(),
            ShelfContent::ShelfMap(shelf_map) => {
                let json_map: serde_json::Map<String, JSON> = shelf_map.into_iter().map(|(k, shelf)| (k, shelf.to_json_values())).collect();
                JSON::Object(json_map)
            },
        }
    }
    }


impl<T : Display + PartialOrd + Clone> Display for ShelfContent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            ShelfContent::Value(a) => format!("{a}"),
            ShelfContent::ShelfMap(shelf_map) => {
                let strs: Vec<String> = shelf_map.into_iter().map(|(k, shelf)| format!("\"{k}\": {shelf}")).collect();
                format!("{{{}}}",strs.join(", "))
            },
        };
        write!(f, "{}", repr)
    }
}

impl<T: Display + PartialOrd + Clone> Debug for ShelfContent<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return std::fmt::Display::fmt(&self, f)
    }
}


impl<T: PartialOrd + Clone> PartialOrd for ShelfContent<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ShelfContent::ShelfMap(_), ShelfContent::ShelfMap(_)) => None, // Cannot order 2 shelf maps.
            (ShelfContent::ShelfMap(_), ShelfContent::Value(_)) => Some(Ordering::Greater),
            (ShelfContent::Value(_), ShelfContent::ShelfMap(_)) => Some(Ordering::Less),
            (ShelfContent::Value(v1), ShelfContent::Value(v2)) => v1.partial_cmp(v2)

        }
    }
}

impl TryFrom<JSON> for ShelfContent<Value> {
    type Error = String;

    fn try_from(json: JSON) -> Result<Self, Self::Error> {
        match json {
            JSON::Object(obj) => {
                let mut shelves: HashMap<String, Shelf<Value>> = HashMap::new();
                for (k, v) in obj {
                    shelves.insert(k, v.try_into()?);
                }
                Ok(ShelfContent::ShelfMap(shelves))
            }
            val => Ok(ShelfContent::Value(val.try_into()?)),
        }
    }
}

impl From<ShelfContent<Value>> for JSON {
    fn from(content: ShelfContent<Value>) -> Self {
        match content {
            ShelfContent::Value(a) => a.into(),
            ShelfContent::ShelfMap(shelf_map) => {
                let shelf_map: HashMap<_,_> = shelf_map.into_iter().map(|(k, shelf)| (k, JSON::from(shelf))).collect();
                json!(shelf_map)
            },
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum StateVector {
    Node(HashMap<String, StateVector>, ShelfClock),
    Leaf(ShelfClock),
}

impl Default for StateVector {
    fn default() -> Self {
        StateVector::Leaf(ShelfClock::default())
    }
}

impl Debug for StateVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Self::Node(node, clock) => {
                let strs: Vec<String> = node.into_iter().map(|(k, sv)| format!("{k}: {sv:?}")).collect();
                format!("[{{{}}}, {clock}]",strs.join(", "))
            }
            Self::Leaf(clock) => format!("{clock}"),
        };

        write!(f, "{repr}")
    }

}

impl StateVector {
    pub fn get_clock(&self) -> &ShelfClock {
        match self {
            StateVector::Node(_, clock) => clock,
            StateVector::Leaf(clock) => clock
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShelfClock {
    client_id: usize,
    clock: usize
}

impl Default for ShelfClock {
    fn default() -> Self {
        ShelfClock { client_id: 0, clock: 0 }
    }
}

impl ShelfClock {
    pub fn new(client_id: usize) -> Self {
        ShelfClock { client_id , clock: 0 }
    }

    pub fn increment(&self, client_id: usize) -> Self {
        ShelfClock { client_id, clock: self.clock + 1 }
    }
}

impl PartialOrd for ShelfClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.clock.cmp(&other.clock) {
            order @ (Ordering::Greater | Ordering::Less)  => Some(order),
            Ordering::Equal if self.client_id == other.client_id =>  Some(Ordering::Equal), // Ordering by client id is besides the point of shelf, but should we try it?
            _ => None

        }
    }
}

impl Display for ShelfClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "[{}, {}]", self.client_id, self.clock)
    }
}

impl TryFrom<JSON> for ShelfClock {
    type Error = String;

    fn try_from(value: JSON) -> Result<Self, Self::Error> {
        match value {
            JSON::Array(mut array) if array.len() == 2 => {
                match (array.remove(0), array.remove(0)) {
                    (JSON::Number(client_id), JSON::Number(clock)) => {
                        let client_id = client_id.as_u64().ok_or(format!("Could not parse client_id from {client_id}"))? as usize;
                        let clock = clock.as_u64().ok_or(format!("Could not parse clock from {clock}"))? as usize;
                        Ok(ShelfClock {
                            client_id, clock
                        })
                    }
                    v => Err(format!("Could not parse ShelfClock from {v:?}"))
                }
            }
            v => Err(format!("Could not extract ShelfClock from {v:?}"))
        }
    }
}

/**
 * context?
 * we need it for get and set at some top level
 * (Shelf, context).set(key, val, context)
 */



#[derive(Serialize, Deserialize, Clone)]
pub struct Shelf<T: PartialOrd + Clone> {
    pub content: ShelfContent<T>,
    clock: ShelfClock,
}
impl <T: PartialOrd + Clone> PartialEq for Shelf<T> {
    fn eq(&self, other: &Self) -> bool {
        let clock_eq = match (&self.content, &other.content) {
            // If both shelf maps, we don't care who the client is because we compare recursively anyway
            (ShelfContent::ShelfMap(_), ShelfContent::ShelfMap(_)) => self.clock.clock == other.clock.clock,
            _ => self.clock == other.clock
        };

        clock_eq && self.content == other.content
    }
}

impl<T: Clone + PartialOrd> Shelf<T> {

    /// Creates a new shelf around the value with a clock value of 0
    pub fn new(content: ShelfContent<T>, client_id: usize) -> Self {
        return Shelf {
            content: content,
            clock: ShelfClock::new(client_id),
        };
    }

    /// Determines whether there are more shelves nested inside this one.
    pub fn contains_shelves(&self) -> bool {
        match self.content {
            ShelfContent::ShelfMap(_) => true,
            _ => false,
        }
    }
    ///  Gets a Value out of the Shelf
    pub fn get(&self, key: &str) -> Option<&Self> {
        match &self.content {
            ShelfContent::ShelfMap(shelf_map) => shelf_map.get(key),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        match &mut self.content {
            ShelfContent::ShelfMap(shelf_map) => shelf_map.get_mut(key),
            _ => None,
        }
    }

    pub fn set(&mut self, key: String, value: ShelfContent<T>, client_id: usize) -> Option<Self> {
        let mut new_shelf = Shelf {
            content: value,
            clock: self.clock
        };
        match &mut self.content {
            ShelfContent::ShelfMap(shelf_map) => {
                if let Some(old_shelf) = shelf_map.get(&key) {
                    new_shelf.clock = old_shelf.clock.increment(client_id)
                }
                shelf_map.insert(key,new_shelf)
            },
            _ => None,
        }
    }

    pub fn set_content(&mut self, val: ShelfContent<T>, client_id: usize) {
        self.content = val;
        self.clock = self.clock.increment(client_id); 

    }
    /// Deletes any ShelfMap children with a lower clock value than the parent.
    pub fn prune(&mut self) {
        let shelf_map = match &mut self.content {
            ShelfContent::ShelfMap(shelf_map) => shelf_map,
            _ => return
        };
        shelf_map.retain(|_,shelf| shelf.clock.clock >= self.clock.clock);
    }

    /// Recursively prunes the shelf tree
    pub fn garbage_collect(&mut self) {
        self.prune();
        let shelf_map = match &mut self.content {
            ShelfContent::ShelfMap(shelf_map) => shelf_map,
            _ => return
        };
        shelf_map.iter_mut().for_each(|(_, shelf)| {shelf.garbage_collect();});

    }

    

    

}

// SERDE Stuff
impl<T: PartialOrd + Clone + Serialize + DeserializeOwned> Shelf<T> {
    pub fn encode_state_vector(&self) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(&self.get_state_vector())
    }

    pub fn encode_state_delta(&self, sv_bytes: &[u8]) -> Option<Result<Vec<u8>, Box<bincode::ErrorKind>>> {
        match bincode::deserialize(sv_bytes) {
            Ok(sv) => {
                self.get_state_delta(&sv).map(|delta| bincode::serialize(&delta))
            },
            Err(err) => {
                Some(Err(err))
            }
        }
        
    }
}


impl Shelf<Value> {

    pub fn from_json_values(values: JSON, client_id: usize) -> Result<Self,String> {
        let content = match values {
            JSON::Object(obj) => {
                let mut children = HashMap::new();
                for (key, value) in obj {
                    children.insert(key, Self::from_json_values(value, client_id)?);
                }
                ShelfContent::ShelfMap(children)
                
            },
            val => ShelfContent::Value(val.try_into()?)
        };

        Ok(Shelf {
            content,
            clock: ShelfClock::new(client_id)
        })
    }

    pub fn to_json_values(self) -> JSON {
        self.content.to_json_values()
    }
}

impl<T: Display + PartialOrd + Clone> Display for Shelf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.content, self.clock)
    }
}

impl<T:Display + PartialOrd + Clone> Debug for Shelf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return std::fmt::Display::fmt(&self, f)
    }
}

impl TryFrom<JSON> for Shelf<Value> {
    type Error = String;

    fn try_from(value: JSON) -> Result<Self, Self::Error> {
        match value {
            JSON::Array(mut array) => {
                if array.len() != 2 {
                    return Err("Array did not have 2 dimensions".to_string());
                }

                let (val, clock) = (array.remove(0), array.remove(0));
                let clock = ShelfClock::try_from(clock)?;
                let mut shelf = Shelf {
                    content: val.try_into()?,
                    clock,
                };
                shelf.prune();

                Ok(shelf)
            }
            val => Err(format!("Could not covert JSON into a shelf: {:?}", val)),
        }
    }
}

impl From<Shelf<Value>> for JSON {
    fn from(shelf: Shelf<Value>) -> Self {
            let val = ShelfContent::from(shelf.content);
            json!([val, shelf.clock])

    }
}

/*
Currently (x, 1) cmp (x,1) -> x must be in delta
    Because we don't know if clock was set by current client or another
We want (x, (1,1)) cmp (x, (1,1)) 
    Because we can prune this

*/

impl<T: PartialOrd + Clone> Mergeable<Self> for Shelf<T> {
    /// Merges another shelf into the current one, returning the resulting union.
    fn merge(&mut self, other: Self) {
        let other_content = other.content;
        let clock_order = self.clock.partial_cmp(&other.clock);
        // Merging two shelf maps

        match (&mut self.content, other_content, clock_order) {
            (_, other_content, Some(Ordering::Less)) => {
                self.content = other_content;
                self.clock = other.clock;
            } // Update is greater so take on that value
            (_, _, Some(Ordering::Greater)) => (), // Self is greater so no need to update
            (&mut ShelfContent::ShelfMap(ref mut these_shelves), ShelfContent::ShelfMap(other_shelves), _) => {
                for (key, val) in other_shelves.into_iter() {
                    if let Some(sub_shelf) = these_shelves.get_mut(&key) {
                        sub_shelf.merge(val);
                    } else {
                        these_shelves.insert(key, val);
                    }
                }
                if self.clock.client_id < other.clock.client_id {
                    // Not necessary but makes clock state match between replicas
                    self.clock = other.clock;
                }
            } // If there is no priority between maps, they should be merged recursively.
            (_,_, Some(Ordering::Equal)) => (), // Ruling out recursive map merges ^, if clocks are the same, then the value is unchanged.
            (this_val, other_val, None) => 
            {
                // Try partial comparison of content and default to client_ids if this fails. Type compare will fail for things like floats that equal NaN.
                let order = (*this_val).partial_cmp(&other_val);
                let replacement_shelf_values = match order {
                    Some(Ordering::Less) => Some((other_val, other.clock)),
                    Some(Ordering::Equal) | None if self.clock.client_id < other.clock.client_id => Some((other_val, other.clock)),
                    _ => None
                };
                if let Some((value, clock)) = replacement_shelf_values {
                    self.content = value;
                    self.clock = clock;
                }
                
            } // In the case that both are different shelf content types, just take the type max.
        };
    }
}

impl<T: PartialOrd + Clone> DeltaCRDT for Shelf<T> {
    type Delta = Shelf<T>;
    type StateVector = StateVector;
    fn get_state_vector(&self) -> StateVector {
        let children =  match &self.content {
            ShelfContent::Value(_) => None,
            ShelfContent::ShelfMap(shelf_map) => Some(
                shelf_map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.get_state_vector()))
                    .collect(),
            ),
        };

        if let Some(children) = children {
            StateVector::Node(children, self.clock)
        } else {
            StateVector::Leaf(self.clock)
        }
    }

    /*

    (1,1) higher client id wins? Yeah that makes sense
    So there is an absolute order in this case
    Dealing with this is
    (1,2)
    client falls back on type partial order?
    send if client has higher value id?

    highest clock wins
    but client breaks the tie?
    what if when client fails we say these cannot be partially ordered?
    None can be an escape hatch
    Otherwise we cant be sure if we should merge children. We would have to check each
    This is pretty much what we do now but the extra int allows us to fix the delta issue. 
    Downside? we lose client ordering. But the Shelf CRDT can work without.
     */

    fn get_state_delta(&self, state_vector: &StateVector) -> Option<Self::Delta> {
        let clock_ordering = self.clock.partial_cmp(&state_vector.get_clock());
        let content = &self.content;
        match (content, state_vector, clock_ordering) {
            (_,_, Some(Ordering::Less)) => None, // No new information to share due to clock Some(ordering or lack) of data
            (_,_, Some(Ordering::Greater)) => Some(self.clone()), // This content more prevalent than peer.
            (ShelfContent::ShelfMap(shelf_map), StateVector::Node(sv_children, sv_clock), _) => {
                    let updated_shelf_map: HashMap<String, _> = shelf_map.iter()
                        .filter_map(|(k, v)| {
                            let delta = if let Some(sv_child) = sv_children
                                .get(k) {
                                    v.get_state_delta(sv_child)
                                } else if &v.clock < sv_clock {
                                    // Values less than parent clock have been overwritten
                                    None
                                } else {
                                    // Send the shelf if the values cannot be compared (ie different clients) or the parent clock is >= to value
                                    Some(v.clone())
                                };
                            Some((k.to_owned(), delta?))
                        })
                        .collect();
                    let has_elements = !updated_shelf_map.is_empty(); // Even if empty, it is an update if clocks don't match.
                    has_elements.then(|| Shelf {
                        content: ShelfContent::ShelfMap(updated_shelf_map),
                        clock: self.clock,
                    })
            }, // if maps, merge recursively
            (_, _, Some(Ordering::Equal)) => None, // If the clocks equal, no need to send anything over.
            (ShelfContent::Value(_), StateVector::Node(..), None) => None, // Type order wins: Map > anything else
            (_,_, None) => Some(self.clone()) // No partial ordering? Values must be compared directly
        }
    }
}


struct Awareness<T: PartialOrd + Clone> {
    shelf: Option<Shelf<T>>,
    client_id: usize
}

impl <T: PartialOrd + Clone> Awareness<T> {
    fn new() -> Self {
        let client_id = rand::thread_rng().gen();
        Awareness { shelf: None, client_id }
    }

    fn get(&self, key: &str) -> Option<&Shelf<T>> {
        self.shelf.as_ref().and_then(|shelf| shelf.get(key))
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Shelf<T>> {
        self.shelf.as_mut().and_then(|shelf| shelf.get_mut(key))
    }

    fn set(&mut self, key: String, value: ShelfContent<T>) -> Option<Shelf<T>> {
        self.shelf.as_mut().and_then(|shelf| shelf.set(key, value, self.client_id))
    }

    fn set_content(&mut self, value: ShelfContent<T>) {
        if let Some(shelf) = self.shelf.as_mut() {
            shelf.set_content( value, self.client_id)
        }
    }
}

impl Awareness<Value> {
    fn from_values(values: JSON) -> Result<Self, String> {
        let client_id = rand::thread_rng().gen();
        let shelf = Shelf::from_json_values(values, client_id)?;
        Ok(Awareness { shelf:Some(shelf), client_id })
    }

    



}




#[cfg(test)]
mod tests {


    use rand::{prelude::StdRng, SeedableRng};

    use crate::{shelf_fuzzer::ShelfFuzzer, wrap_crdt::*};

    fn val(a: Value) -> ShelfContent<Value> {
        ShelfContent::Value(a)
    }
    fn merge(branch: Shelf<Value>, mut main: Shelf<Value>) -> Shelf<Value> {
        let sv = main.get_state_vector();
        if let Some(delta) = branch.get_state_delta(&sv) {
            main.merge(delta);
        }
        main
    }

    fn array(elements: Vec<isize>) -> ShelfContent<Value> {
        val(Value::Array(
            elements.into_iter().map(|el| Value::Int(el)).collect(),
        ))
    }

    fn clock(c: usize) -> ShelfClock {
        ShelfClock { client_id: 0, clock: c }
    }

    fn validate_crdt_properties<T: PartialEq + PartialOrd + Clone + Display>(shelf: Shelf<T>, shelf2: Shelf<T>) -> Shelf<T> {
        // Forwards
        let mut receiver = shelf.clone();
        let sender = shelf2.clone();
        let sv = receiver.get_state_vector();

        
        let delta = sender.get_state_delta(&sv);
        let cached_delta = delta.clone();
        if let Some(delta) = delta {
            receiver.merge(delta);
        }

        // Backwards
        let mut receiver_back = shelf2.clone();
        let sender_back = shelf.clone();
        let sv_back = receiver_back.get_state_vector();
        let delta_back = sender_back.get_state_delta(&sv_back);
        let cached_delta_back = delta_back.clone();
        if let Some(delta) = delta_back {
            receiver_back.merge(delta);
        }
        let id_delta = cached_delta.clone();
        let report = || {
            format!("\nReceiver: {shelf}, \nSender {shelf2}, \nStateVector {sv:?}, \nDelta: {id_delta:?} \nStateVectorBACK: {sv_back:?}, \nDeltaBACK {cached_delta_back:?}")
        };
        

        // Ensure both forwards and backwards match
        assert_eq!(receiver, receiver_back, "Not commutative {}", report());
        if let Some(delta) = cached_delta {
            receiver.merge(delta);
            assert_eq!(receiver, receiver_back, "Not idempotent {}", report());
        }
        return receiver
    }

    #[test]
    fn test_init() {
        Shelf {
            content: val(Value::Bool(true)),
            clock: clock(1),
        };
    }

    #[test]
    fn test_clock() {
        let mut shelf = Shelf {
            content: val(Value::Int(1)),
            clock: clock(1),
        };
        let y = val(Value::Int(2));
        let shelf2 = Shelf {
            content: y,
            clock: clock(2),
        };
        shelf.merge(shelf2);

        assert!(shelf.clock == clock(2));
    }
    #[test]
    fn test_object_override() {
        let x = ShelfContent::ShelfMap(HashMap::new());
        let shelf: Shelf<_> = Shelf::new(x, 0);
        let y = val(Value::Int(2));
        let shelf2 = Shelf::new(y, 1);
        let shelf = merge(shelf, shelf2);

        if let ShelfContent::ShelfMap(_) = shelf.content {
        } else {
            panic!("Expected map to override the integer value")
        }
    }
    #[test]
    fn test_vec_diff() {
        let shelf: Shelf<_> = Shelf {
            content: array(vec![1]),
            clock: clock(1),
        };
        let y = array(vec![2]);
        let shelf2 = Shelf {
            content: y,
            clock: clock(2),
        };
        let shelf = merge(shelf2, shelf);

        if let ShelfContent::Value(Value::Array(list)) = shelf.content {
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
        let sub_shelf = Shelf::new(val(Value::Int(1)), 0);

        let mut dict = HashMap::new();
        dict.insert("a".to_string(), sub_shelf);
        let shelf: Shelf<_> = Shelf::new(ShelfContent::ShelfMap(dict), 0);

        let sub_shelf2 = Shelf::new(val(Value::Int(2)), 1);

        let mut dict2 = HashMap::new();
        dict2.insert("b".to_string(), sub_shelf2);
        let shelf2 = Shelf::new(ShelfContent::ShelfMap(dict2), 1);

        let shelf = merge(shelf2, shelf);

        if let ShelfContent::ShelfMap(m) = shelf.content {
            assert!(m.len() == 2)
        }
    }

    #[test]
    fn test_state_vector() {
        let shelf: Shelf<_> = json!([{ 
            "user": [{
            "mouse_position": [[0, 1], [0,0]],
            "cursor": [{"left": ["a", [0,0]], "right": ["b", [0,0]]}, [0,0]]
        },[0,0]]  
    }, [0,0]])
        .try_into()
        .unwrap();

        let state_vector = shelf.get_state_vector();
        match state_vector {
            StateVector::Node(map, c) => {
                assert_eq!(c, clock(0));
                match &map["user"] {
                    StateVector::Node(map, c) => {
                        assert_eq!(*c, clock(0));
                        match &map["mouse_position"] {
                            StateVector::Node(_, _) => panic!("Array should not be a node"),
                            StateVector::Leaf(i) => assert_eq!(*i, clock(0)),
                        };

                        match &map["cursor"] {
                            StateVector::Node(map, c) => {
                                assert_eq!(*c, clock(0));
                                if let StateVector::Leaf(i) = &map["left"] {
                                    assert_eq!(i, &clock(0));
                                } else {
                                    panic!("left isn't a leaf");
                                }

                                if let StateVector::Leaf(i) = &map["right"] {
                                    assert_eq!(i, &clock(0));
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
        let shelf1: Shelf<Value> = json!([{
            "user1": [{
                "username": ["waidhoferj", [0,0]]
            }, [0,0]]
        }, [0,0]])
        .try_into()
        .unwrap();
        // number ,string, List*, Map<string, Shelf>
        let mut shelf2: Shelf<Value> = json!([{ "user2": [{"username": ["jwaidhof", [1,0]]}, [1,0]] }, [1,0]])
            .try_into()
            .unwrap();
        let state_vec = shelf2.get_state_vector();
        let diff = shelf1.get_state_delta(&state_vec).unwrap();
        

        shelf2.merge(diff);

        let expected: Shelf<Value> = json!([{
            "user1": [{
                "username": ["waidhoferj",[0,0]]
            }, [0,0]],
            "user2": [{"username": ["jwaidhof",[1,0]]}, [1,0]]
        }, [1,0]])
        .try_into()
        .unwrap();
        assert_eq!(shelf2, expected);

    }

    #[test]
    fn test_get() {
        let mut shelf: Shelf<Value> = json!([{ "user": [{
            "mouse_position": [[0, 1], [0,0]],
            "cursor": [{"left": ["a",[0,0]], "right": ["b",[0,0]]},[0,0]]
        }, [0,0]]  }, [0,0]])
        .try_into()
        .unwrap();
        let res: Option<ShelfContent<Value>> = (|| {
            shelf
                .get_mut("user")?
                .get_mut("cursor")?
                .get_mut("left")
                .map(|s| s.content.clone())
        })();
        if let Some(ShelfContent::Value(Value::String(s))) = res {
            assert_eq!(s, "a");
        } else {
            panic!("Unexpected value {:?}", res)
        }

        assert!(shelf.get("BOOM/goes/the/path").is_none())
    }

    #[test]

    fn test_adding_user() {
        let mut shelf1: Shelf<Value> = json!([{"1": [{"mouse_position": [[1, 2], [0,2]]}, [0,2]]}, [0,1]])
            .try_into()
            .unwrap();
        let shelf2: Shelf<Value> = json!([{"2": [{"mouse_position": [[3, 4], [1,1]]}, [1,1]]}, [1,1]])
            .try_into()
            .unwrap();

        let expected = json!([{
            "1": [{"mouse_position": [[1, 2], [0,2]]}, [0,2]],
            "2": [{"mouse_position": [[3, 4], [1,1]]}, [1,1]]
        }, [1,1]])
        .try_into()
        .unwrap();
        let sv = shelf1.get_state_vector();
        let diff = shelf2.get_state_delta(&sv).unwrap();
        shelf1.merge(diff); // Mutate in place
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
            let shelves: Vec<Shelf<Value>> = match test.remove("shelves") {
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
                    receiver.merge(delta);

                    // Backwards
                    let mut receiver_back = shelves[j].clone();
                    let sender_back = shelves[i].clone();
                    let sv = receiver_back.get_state_vector();
                    
                    if let Some(delta) = sender_back.get_state_delta(&sv) {
                        receiver_back.merge(delta)
                    }
                    

                    // Ensure both forwards and backwards match
                    assert_eq!(receiver, receiver_back, "Not commutative\n {description}");

                    // Ensure duplicate application of deltas has no effect
                    receiver.merge(cached_delta);
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
        let shelf_map_with_value = Shelf {
            content: ShelfContent::Value(1),
            clock: ShelfClock::new(0)
        };

        let empty_shelf_map: Shelf<isize> = Shelf {
            content: ShelfContent::ShelfMap(HashMap::new()),
            clock: ShelfClock::new(1)
        };

        let shelf_with_value: Shelf<isize> = Shelf {
            content: ShelfContent::ShelfMap(HashMap::new()),
            clock: ShelfClock::new(2)
        };

        let shelves = vec![shelf_with_value, empty_shelf_map, shelf_map_with_value];
        // test all combinations
        for i in 0..shelves.len() {
            for j in (i+1)..shelves.len() {
                let shelf1 = shelves[i].clone();
                let mut shelf2 = shelves[j].clone();
                shelf2.clock = shelf2.clock.increment(1);

                let res = validate_crdt_properties(shelf2, shelf1);
                println!("{res}");
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
            let mut shelf = Shelf::try_from(fuzzer.generate_json_shelf(1)).unwrap();
            let mut shelf2 = Shelf::try_from(fuzzer.generate_json_shelf(2)).unwrap();
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
                receiver.merge(delta);
            }

            // Backwards
            let mut receiver_back = shelf2.clone();
            let sender_back = shelf.clone();
            let sv_back = receiver_back.get_state_vector();
            let delta_back = sender_back.get_state_delta(&sv_back);
            let cached_delta_back = delta_back.clone();
            if let Some(delta) = delta_back {
                receiver_back.merge(delta);
            }
            let id_delta = cached_delta.clone();
            let report = || {
                format!("\nTEST {i}\nReceiver: {shelf}, \nSender {shelf2}, \nStateVector {sv:?}, \nDelta: {id_delta:?} \nStateVectorBACK: {sv_back:?}, \nDeltaBACK {cached_delta_back:?}")
            };
            

            // Ensure both forwards and backwards match
            assert_eq!(receiver, receiver_back, "Not commutative {}", report());
            if let Some(delta) = cached_delta {
                receiver.merge(delta);
                assert_eq!(receiver, receiver_back, "Not idempotent {}", report());
            }
            
        }
    }

}
