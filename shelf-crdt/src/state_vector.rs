use serde::{Deserialize, Serialize};

use crate::clock::{ShelfClock};
use crate::wrap_crdt::Shelf;
use std::clone::Clone;
use std::cmp::Ordering;
use std::{collections::HashMap, fmt::Debug};

use crate::traits::{DeltaCRDT};

#[derive(PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum StateVector<NodeClock: PartialEq + PartialOrd, LeafClock: PartialEq + PartialOrd> {
    Node(
        HashMap<String, StateVector<NodeClock, LeafClock>>,
        NodeClock,
    ),
    Leaf(LeafClock),
}

impl<NodeClock, LeafClock> StateVector<NodeClock, LeafClock>
where
    NodeClock: PartialEq + PartialOrd,
    LeafClock: PartialEq + PartialOrd,
{
    fn get_clock(&self) -> ShelfClock<NodeClock, LeafClock> {
        match self {
            StateVector::Node(_, clock) => ShelfClock::MapClock(clock),
            StateVector::Leaf(clock) => ShelfClock::ValueClock(clock),
        }
    }
}

impl<N: PartialEq + PartialOrd, L: Default + PartialEq + PartialOrd> Default for StateVector<N, L> {
    fn default() -> Self {
        StateVector::Leaf(L::default())
    }
}

impl<N,L> Debug for StateVector<N, L>
    where
    N: PartialEq + PartialOrd + Debug,
    L: PartialEq + PartialOrd + Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Self::Node(node, clock) => {
                let strs: Vec<String> = node
                    .into_iter()
                    .map(|(k, sv)| format!("{k}: {sv:?}"))
                    .collect();
                format!("[{{{}}}, {clock:?}]", strs.join(", "))
            }
            Self::Leaf(clock) => format!("{clock:?}"),
        };

        write!(f, "{repr}")
    }
}

impl<T, MapClock, ValueClock> DeltaCRDT for Shelf<T, MapClock, ValueClock>
where
    T: PartialOrd + Clone,
    MapClock: PartialEq + PartialOrd + PartialOrd<ValueClock> + PartialEq<ValueClock> + Clone,
    ValueClock: PartialEq + PartialOrd + PartialOrd<MapClock> + PartialEq<MapClock> + Clone,
{
    type Delta = Shelf<T, MapClock, ValueClock>;
    type StateVector = StateVector<MapClock, ValueClock>;
    fn get_state_vector(&self) -> Self::StateVector {
        match &self {
            Shelf::Value { clock, .. } => StateVector::Leaf(clock.clone()),
            Shelf::Map { shelves, clock } => StateVector::Node(
                shelves
                    .iter()
                    .map(|(k, v)| (k.clone(), v.get_state_vector()))
                    .collect(),
                clock.clone(),
            ),
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

    fn get_state_delta(&self, state_vector: &Self::StateVector) -> Option<Self::Delta> {
        let clock_ordering = self.get_clock().partial_cmp(&state_vector.get_clock());
        match (self, state_vector, clock_ordering) {
            (_, _, Some(Ordering::Less)) => None, // No new information to share due to clock Some(ordering or lack) of data
            (_, _, Some(Ordering::Greater)) => Some(self.clone()), // This content more prevalent than peer.
            (Shelf::Map { shelves, clock: map_clock }, StateVector::Node(sv_children, sv_clock), _) => {
                let sv_clock = ShelfClock::MapClock(sv_clock);
                let updated_shelf_map: HashMap<String, _> = shelves
                    .iter()
                    .filter_map(|(k, v)| {
                        let delta = if let Some(sv_child) = sv_children.get(k) {
                            v.get_state_delta(sv_child)
                        } else if v.get_clock() < sv_clock {
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
                has_elements.then(|| Shelf::Map {
                    shelves: updated_shelf_map,
                    clock: map_clock.clone(),
                })
            } // if maps, merge recursively
            (_, _, Some(Ordering::Equal)) => None, // If the clocks equal, no need to send anything over.
            (Shelf::Value { .. }, StateVector::Node(..), None) => None, // Type order wins: Map > anything else
            (_, _, None) => Some(self.clone()), // No partial ordering? Values must be compared directly
        }
    }
}

pub struct StateVectorContext;

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::{clock::{DotClock, LamportTimestamp}, traits::Mergeable};

    use super::*;
    type TestShelf = Shelf<crate::json::Value, LamportTimestamp, DotClock>;

    fn clock(c: usize) -> DotClock {
        DotClock {
            client_id: 0,
            clock: c,
        }
    }

    #[test]
    fn test_state_vector() {
        let shelf: TestShelf = json!([{ 
            "user": [{
            "mouse_position": [[0, 1], [0,0]],
            "cursor": [{"left": ["a", [0,0]], "right": ["b", [0,0]]}, 0]
        },0]  
    }, 0])
        .try_into()
        .unwrap();

        let state_vector = shelf.get_state_vector();
        match state_vector {
            StateVector::Node(map, c) => {
                assert_eq!(c, LamportTimestamp(0));
                match &map["user"] {
                    StateVector::Node(map, c) => {
                        assert_eq!(*c, LamportTimestamp(0));
                        match &map["mouse_position"] {
                            StateVector::Node(_, _) => panic!("Array should not be a node"),
                            StateVector::Leaf(i) => assert_eq!(*i, clock(0)),
                        };

                        match &map["cursor"] {
                            StateVector::Node(map, c) => {
                                assert_eq!(*c, LamportTimestamp(0));
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
        let shelf1: TestShelf = json!([{
            "user1": [{
                "username": ["waidhoferj", [0,0]]
            }, 0]
        }, 0])
        .try_into()
        .unwrap();
        // number ,string, List*, Map<string, Shelf>
        let mut shelf2: TestShelf =
            json!([{ "user2": [{"username": ["jwaidhof", [1,0]]}, 0] }, 0])
                .try_into()
                .unwrap();
        let state_vec = shelf2.get_state_vector();
        let diff = shelf1.get_state_delta(&state_vec).unwrap();

        shelf2 = shelf2.merge(diff);

        let expected: TestShelf = json!([{
            "user1": [{
                "username": ["waidhoferj",[0,0]]
            }, 0],
            "user2": [{"username": ["jwaidhof",[1,0]]}, 0]
        }, 0])
        .try_into()
        .unwrap();
        assert_eq!(shelf2, expected);
    }
}
