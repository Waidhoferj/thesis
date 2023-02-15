import BenchmarkEnvironment from "./base.js";
import * as shelf from "shelf";
import * as sizeOfMod from "object-sizeof";
const sizeOf = sizeOfMod.default;

let { Fuzzer, Awareness: ShelfAwareness, DotShelf: Shelf } = shelf;

export class ShelfBench extends BenchmarkEnvironment {
  constructor() {
    super();
    this.name = "Shelf (ours)";
  }

  testDeltaSize(firstValues, secondValues) {
    const sizes = {};
    const client_id = 1;

    const shelf1 = new Shelf(firstValues, 1);
    const shelf2 = new Shelf(secondValues, 2);
    let sv = shelf1.getStateVector();
    let delta = shelf2.getStateDelta(sv);
    sizes["Random Merge"] = delta.byteLength;

    // With only a single element changed
    const shelf1Updated = new Shelf(firstValues, 1);
    shelf1Updated.set(["test"], "delta");
    sv = shelf1.getStateVector();
    delta = shelf1Updated.getStateDelta(sv);
    sizes["Single Change"] = delta.byteLength;

    // Complete deletion
    const deletionShelf = new Shelf({ contents: firstValues }, 1); // TODO does the shelf actually take client_id here?
    const deletionShelfCopy = new Shelf({ contents: firstValues }, 1);
    deletionShelf.set(["contents"], {}, 1);
    sv = deletionShelfCopy.getStateVector();
    delta = deletionShelf.getStateDelta(sv);
    sizes["Complete Deletion"] = delta.byteLength;
    return sizes;
  }

  testSizeAfterDeletion(values) {
    const deletionShelf = new Shelf({ contents: values }, 1);
    deletionShelf.set(["contents"], {}, 1);
    return { "Complete Deletion": sizeOf(deletionShelf) };
  }

  testCRDTSize(values) {
    const shelfCRDT = new Shelf(values);
    return sizeOf(shelfCRDT);
  }

  testNAdditions() {
    let crdt = new Shelf({ base: 1 }, 1);

    let fuzzer = new Fuzzer(this.fuzzerConfig);
    let content = fuzzer.generateContent();

    return () => {
      for (let [key, val] of Object.entries(content)) {
        crdt.set([key], val, 1);
      }
    };
  }

  testMerge() {
    let fuzzer = new Fuzzer(this.fuzzerConfig);
    let first = new Shelf(fuzzer.generateContent(), 1);
    let second = new Shelf(fuzzer.generateContent(), 2);

    return () => {
      let sv = first.getStateVector();
      let delta = second.getStateDelta(sv);
      if (delta) {
        first = first.merge(delta);
      } else {
        throw Error("There should be a delta for the shelfCRDT");
      }
    };
  }
}

export class ShelfAwarenessBench extends BenchmarkEnvironment {
  constructor() {
    super();
    this.name = "Awareness (ours)";
  }

  testDeltaSize(firstValues, secondValues) {
    const sizes = {};

    const shelf1 = new shelf.Awareness(firstValues, 1);
    const shelf2 = new shelf.Awareness(secondValues, 2);
    let sv = shelf1.getStateVector();
    let delta = shelf2.getStateDelta(sv);
    sizes["Random Merge"] = delta.byteLength;

    // With only a single element changed
    const shelf1Updated = new shelf.Awareness(firstValues, 1);
    shelf1Updated.set(["test"], "delta");
    sv = shelf1.getStateVector();
    delta = shelf1Updated.getStateDelta(sv);
    sizes["Single Change"] = delta.byteLength;

    // Complete deletion
    const deletionShelf = new shelf.Awareness({ contents: firstValues }, 1);
    const deletionShelfCopy = new shelf.Awareness({ contents: firstValues }, 1);
    deletionShelf.set(["contents"], {});
    sv = deletionShelfCopy.getStateVector();
    delta = deletionShelf.getStateDelta(sv);
    sizes["Complete Deletion"] = delta.byteLength;
    return sizes;
  }

  testSizeAfterDeletion(values) {
    const deletionShelf = new Shelf({ contents: values }, 1);
    deletionShelf.set(["contents"], {});
    return { "Complete Deletion": sizeOf(deletionShelf) };
  }

  testCRDTSize(values) {
    const shelfCRDT = new Shelf(values);
    return sizeOf(shelfCRDT);
  }

  testNAdditions() {
    let crdt = new Shelf({ base: 1 }, 1);

    let fuzzer = new Fuzzer(this.fuzzerConfig);
    let content = fuzzer.generateContent();

    return () => {
      for (let [key, val] of Object.entries(content)) {
        crdt.set([key], val, 1);
      }
    };
  }

  testMerge() {
    let fuzzer = new Fuzzer(this.fuzzerConfig);
    let first = new Shelf(fuzzer.generateContent(), 1);
    let second = new Shelf(fuzzer.generateContent(), 2);

    return () => {
      let sv = first.getStateVector();
      let delta = second.getStateDelta(sv);
      if (delta) {
        first = first.merge(delta);
      } else {
        throw Error("There should be a delta for the shelfCRDT");
      }
    };
  }
}
