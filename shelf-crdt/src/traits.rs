// pub mod temporal;
// pub mod wrap_crdt;
pub trait Incrementable {
    /// Increments the counter of the source type. It is expected that the type monotonically increases.
    fn increment(&mut self);
}

// Controls how types recursively merge together: TODO: should this be generic?
pub trait Mergeable {
    type Other;
    fn merge(&mut self, other: Self::Other);
}

/// A type with an associated CRDT wrapper that a Doc can use to track/update the object
pub trait CRDTBackend: Clone {
    type Backend: DeltaCRDT;

    fn new_crdt(&self) -> Self::Backend;
}

pub trait TypeOrd {
    fn type_cmp(&self, other: &Self) -> std::cmp::Ordering;
}

// Default behavior is just to choose the max IF THEY ARE NOT COLLECTIONS (Don't have a reliable way to check for this)
// impl Mergeable for usize {
//     fn merge(self, other: Self) -> Self {
//         self.max(other)
//     }
// }

// impl<K, V: Mergeable> Mergeable for HashMap<K, V> {
//     fn merge(&mut self, other: Self) -> Self {
//         todo!()
//     }
// }

pub trait DeltaCRDT: Mergeable {
    type Delta;
    type StateVector;
    fn get_state_vector(&self) -> Self::StateVector;
    fn get_state_delta(&self, sv: &Self::StateVector) -> Option<Self::Delta>;
}
