<img src="./assets//ShelfAware.png"
     alt="ShelfAware Logo"
     width="200" />

# CRDT Experiments

Implementations of the Shelf CRDT

## CRDTs

### Delta Wrap CRDT

The basic implementation of a Shelf CRDT where all Maps and Values are wrapped recursively in Shelf data structures.

**Pros ✅**

- Updates are delta encoded, which make them small and efficient to calculate.

**Cons ❌**

- Walking the tree requires many HashMap lookups.

### Macro Adjacent CRDT

An optimized implementation where specialized State Vectors and Deltas are derived for each struct labeled with `CRDT`.

**Pros ✅**

- State Vectors, Deltas and Merges are very efficient with a minimum number of conditional checks. This could also be vectorized for even more performance gains.

**Cons ❌**

- Doesn't work for dynamic runtime structures or through FFI bindings.
