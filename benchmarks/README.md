# CRDT Benchmarks

Memory and performance comparisons between competing CRDT implementations

## Benchmarks

| Test       | Description                                                                  |
| ---------- | ---------------------------------------------------------------------------- |
| Additions  | How fast can the CRDT add / update values?                                   |
| Deletions  | How fast can the CRDT remove values form its state?                          |
| Merges     | How quickly can the CRDT combine and validate the state between two clients? |
| CRDT Size  | How much memory is taken up by the CRDT?                                     |
| Delta Size | How much memory do encoded state updates take up?                            |

## CRDTs

- Automerge
- Yjs Awareness
- Shelf CRDT (ours)

## Getting Started

1. Set up the JavaScript Shelf CRDT bindings in `shelf-js`
2. Install dependencies:

```
npm i
```

3. Run benchmarks:

```
npm run bench
```

Benchmark results and data can be found in `benchmark/results`.
