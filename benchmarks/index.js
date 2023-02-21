/* benchmark.js */
// import * as b from "benny";
import * as fs from "fs";
import * as shelf from "shelf";
import * as sizeOfMod from "object-sizeof";
const sizeOf = sizeOfMod.default;

let { Fuzzer } = shelf;
import YjsAwarenessBench from "./contenders/yjs.js";
import {
  DotShelfBench,
  ShelfAwarenessBench,
  SecureShelfBench,
} from "./contenders/shelf-aware.js";
import AutomergeBench from "./contenders/automerge.js";

const contenders = [
  new DotShelfBench(),
  new YjsAwarenessBench(),
  new AutomergeBench(),
  new ShelfAwarenessBench(),
  new SecureShelfBench(),
];

// N additions
// size after deletions (pruning)
// batch addition
// Time to encode
// Size of encoding
// Size of crdt
// Size of crdt metadata

function testMemoryFootprint() {
  console.log("Testing delta size...");
  testDeltaSize();
  console.log("Testing deletion size...");
  testSizeAfterDeletion();
  console.log("Testing crdt size...");
  testCRDTSize();
  console.log("Finished Memory Footprint evaluation");
}

/**
 * Fuzzer generates two random JSON trees.
 * Random merge is the update size between two completely desynchronized sources
 * Single Change takes a large tree, makes a single change and encodes the delta update
 * Key metric: Update size in bytes
 */
function testDeltaSize() {
  const N = 100;
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
  const N = 100;
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
  benchmarkMerges();
  benchmarkUpdates();
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

class Benchmark {
  constructor(contenders = [], config = {}) {
    config = { name: "Benchmark", warmupRounds: 3, rounds: 7, ...config };
    this.contenders = contenders;
    this.name = config.name;
    this.warmupRounds = config.warmupRounds;
    this.rounds = config.rounds;
  }

  run() {
    const times = Object.fromEntries(
      this.contenders.map(({ name }) => [name, []])
    );
    for (let { name, code } of this.contenders) {
      for (let i = 0; i < this.warmupRounds; i++) {
        let func = code();
        func();
      }

      for (let round = 0; round < this.rounds; round++) {
        let func = code();
        let start = performance.now();
        func();
        let end = performance.now();
        times[name].push(end - start);
      }
    }
    const averageTimes = Object.fromEntries(
      Object.entries(times).map(([name, times]) => [
        name,
        times.reduce((sum, cur) => sum + cur, 0) / times.length,
      ])
    );
    const opsPerSec = Object.fromEntries(
      Object.entries(averageTimes).map(([name, time]) => [
        name,
        Math.pow(10, 6) / time,
      ])
    );
    return opsPerSec;
  }

  summarize(results) {
    let [fastest] = Object.entries(results).reduce(
      ([bestName, fastestOps], [name, ops]) =>
        ops > fastestOps ? [name, ops] : [bestName, fastestOps],
      ["", -Infinity]
    );
    let [slowest] = Object.entries(results).reduce(
      ([bestName, slowestOps], [name, ops]) =>
        ops < slowestOps ? [name, ops] : [bestName, slowestOps],
      ["", Infinity]
    );

    return { fastest, slowest };
  }
}

function benchmarkMerges() {
  let competitors = contenders.map((c) => ({
    name: c.name,
    code: c.testMerge.bind(c),
  }));

  let bench = new Benchmark(competitors, { name: "Merges" });
  let results = bench.run();
  console.log("Merges", bench.summarize(results), results);
  fs.writeFileSync("benchmark/results/merges.json", JSON.stringify(results));
}

function benchmarkUpdates() {
  let competitors = contenders.map((c) => ({
    name: c.name,
    code: c.testNAdditions.bind(c),
  }));

  let bench = new Benchmark(competitors, { name: "Updates" });
  let results = bench.run();
  console.log("Updates", bench.summarize(results), results);

  fs.writeFileSync("benchmark/results/additions.json", JSON.stringify(results));
}

function generateReports() {
  benchmark();
  testMemoryFootprint();
}

function deepEq(ob1, ob2) {
  return JSON.stringify(ob1) == JSON.stringify(ob2);
}

generateReports();
process.exit(0);
