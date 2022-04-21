use std::collections::HashMap;

mod adjacent_crdt;
pub mod temporal;
pub mod wrap_crdt;
pub trait Incrementable {
    /// Increments the counter of the source type. It is expected that the type monotonically increases.
    fn increment(&mut self);
}

pub trait Mergeable<Other = Self> {
    fn merge(self, other: Other) -> Self;
}

pub trait TypeOrd {
    fn type_cmp(&self, other: &Self) -> std::cmp::Ordering;
}

// Default behavior is just to choose the max IF THEY ARE NOT COLLECTIONS (Don't have a reliable way to check for this)
impl Mergeable for usize {
    fn merge(self, other: Self) -> Self {
        self.max(other)
    }
}

impl<K, V: Mergeable> Mergeable for HashMap<K, V> {
    fn merge(self, other: Self) -> Self {
        todo!()
    }
}

pub trait DeltaCRDT<Delta, StateVector> {
    fn get_state_vector(&self) -> StateVector;
    fn get_state_delta(&self, sv: &StateVector) -> Option<Delta>;
}
