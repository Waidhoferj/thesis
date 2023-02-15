/* benchmark.js */
import * as b from "benny";
import * as fs from "fs";
import * as shelf from "shelf";
import * as sizeOfMod from "object-sizeof";
const sizeOf = sizeOfMod.default;

let { Fuzzer } = shelf;
import YjsAwarenessBench from "./contenders/yjs.js";
import { ShelfBench, ShelfAwarenessBench } from "./contenders/shelf-aware.js";
import AutomergeBench from "./contenders/automerge.js";

const contenders = [
  new YjsAwarenessBench(),
  new ShelfBench(),
  new ShelfAwarenessBench(),
  new AutomergeBench(),
];

// N additions
// size after deletions (pruning)
// batch addition
// Time to encode
// Size of encoding
// Size of crdt
// Size of crdt metadata

function testMemoryFootprint() {
  testDeltaSize();
  testSizeAfterDeletion();
  testCRDTSize();
}

/**
 * Fuzzer generates two random JSON trees.
 * Random merge is the update size between two completely desynchronized sources
 * Single Change takes a large tree, makes a single change and encodes the delta update
 * Key metric: Update size in bytes
 */
function testDeltaSize() {
  const N = 1000;
  const fuzzer = new Fuzzer({
    seed: 1,
    valueRange: [100, 200],
    depthRange: [3, 5],
    branchRange: [1, 5],
  });
  const results = Object.fromEntries(contenders.map((c) => [c.name, {}]));
  for (let i = 0; i < N; i++) {
    const firstValues = fuzzer.generateContent();
    const secondValues = fuzzer.generateContent();
    if (deepEq(firstValues, secondValues)) {
      continue; // Not a fair comparison if all the values are the same. We only count deltas that actually exist.
    }

    for (let contender of contenders) {
      let first = structuredClone(firstValues);
      let second = structuredClone(secondValues);
      let result = contender.testDeltaSize(first, second);
      results[contender.name] = keyWiseFold(results[contender.name], result);
    }
  }

  fs.writeFileSync(
    "benchmark/results/delta-size.json",
    JSON.stringify(results)
  );
}

function testSizeAfterDeletion() {
  const N = 1000;
  const fuzzer = new Fuzzer({
    seed: 1,
    valueRange: [100, 200],
    depthRange: [3, 5],
    branchRange: [1, 5],
  });
  const results = Object.fromEntries(contenders.map((c) => [c.name, {}]));
  for (let i = 0; i < N; i++) {
    const values = fuzzer.generateContent();
    for (let contender of contenders) {
      let vals = structuredClone(values);
      let result = contender.testSizeAfterDeletion(vals);
      results[contender.name] = keyWiseFold(results[contender.name], result);
    }
  }

  fs.writeFileSync(
    "benchmark/results/deletion-size.json",
    JSON.stringify(results)
  );
}

/**
 * How do the sizes of the CRDTs compare? Maybe stringify each?
 * This should be done manually. TODO: sizeOf is probably incorrect. Currently a placeholder
 */
function testCRDTSize() {
  const fuzzer = new Fuzzer({
    seed: 1,
    valueRange: [100, 200],
    depthRange: [3, 6],
    branchRange: [1, 5],
  });
  let values = fuzzer.generateContent();
  const valueSize = sizeOf(values);
  const results = Object.fromEntries(
    contenders.map((c) => [c.name, [c.testCRDTSize(structuredClone(values))]])
  );

  const report = {
    values: [valueSize],
    ...results,
  };

  fs.writeFileSync("benchmark/results/crdt-size.json", JSON.stringify(report));
}

function benchmark() {
  b.suite(
    "B1: Test N additions",
    ...contenders.map((c) => b.add(c.name, c.testNAdditions.bind(c))),
    b.cycle(),
    b.complete(),
    b.save({ file: "additions", version: "1.0.0" }),
    b.save({ file: "additions", format: "chart.html" })
  );

  b.suite(
    "B2: Test Merge",
    ...contenders.map((c) => b.add(c.name, c.testMerge.bind(c))),
    b.cycle(),
    b.complete(),
    b.save({ file: "merges", version: "1.0.0" }),
    b.save({ file: "merges", format: "chart.html" })
  );
}
/**
 * Folds values from dictionary into a list by key
 */
function keyWiseFold(acc, addition) {
  for (let [k, v] of Object.entries(addition)) {
    if (k in acc) {
      acc[k].push(v);
    } else {
      acc[k] = [v];
    }
  }
  return acc;
}

function generateReports() {
  benchmark();
  testMemoryFootprint();
  console.log("Finished Memory Footprint evaluation");
}

function deepEq(ob1, ob2) {
  return JSON.stringify(ob1) == JSON.stringify(ob2);
}

generateReports();
