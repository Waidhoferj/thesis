import BenchmarkEnvironment from "./base.js";
import * as shelf from "shelf";
import * as sizeOfMod from "object-sizeof";
const sizeOf = sizeOfMod.default;

let { Fuzzer, Awareness: ShelfAwareness, DotShelf: Shelf, SecureShelf } = shelf;

function shelfSizeOf(shelf) {
  // We cannot measure the shelf with sizeOf directly because the data is stored in wasm. So we must use a convenience method.
  return (
    shelf.getTotalBytes() + // Size of parts stored in WASM
    sizeOf(shelf) // Size of everything stored in JS
  );
}
export class DotShelfBench extends BenchmarkEnvironment {
  constructor() {
    super();
    this.name = "DotShelf (ours)";
  }

  testDeltaSize(firstValues, secondValues) {
    const sizes = {};

    const shelf1 = new Shelf(firstValues, 1);
    const shelf2 = new Shelf(secondValues, 2);
    let sv = shelf1.getStateVector();
    let delta = shelf2.getStateDelta(sv);
    sizes["Random Merge Update"] = delta.byteLength;

    // With only a single element changed
    const shelf1Updated = new Shelf(firstValues, 1);
    shelf1Updated.set(["test"], "delta");
    sv = shelf1.getStateVector();
    delta = shelf1Updated.getStateDelta(sv);
    sizes["Single Change Update"] = delta.byteLength;

    // Complete deletion
    const deletionShelf = new Shelf({ contents: firstValues }, 1);
    const deletionShelfCopy = new Shelf({ contents: firstValues }, 1);
    deletionShelf.set(["contents"], {}, 1);
    sv = deletionShelfCopy.getStateVector();
    delta = deletionShelf.getStateDelta(sv);
    sizes["Complete Deletion Update"] = delta.byteLength;
    return sizes;
  }

  testSizeAfterDeletion(values) {
    const deletionShelf = new Shelf({ contents: values }, 1);
    deletionShelf.set(["contents"], {}, 1);
    return {
      "Complete Deletion": shelfSizeOf(deletionShelf),
    };
  }

  testCRDTSize(values) {
    const shelfCRDT = new Shelf(values);

    return shelfSizeOf(shelfCRDT);
  }

  testNAdditions() {
    let crdt = new Shelf({ base: 1 }, 1);

    let fuzzer = new Fuzzer(this.config.nAdditions.fuzzerConfig);
    let content = fuzzer.generateContent();

    return () => {
      for (let [key, val] of Object.entries(content)) {
        crdt.set([key], val, 1);
      }
    };
  }

  testMerge() {
    let smallFuzzer = new Fuzzer(this.config.merges.smallFuzzer);
    let largeFuzzer = new Fuzzer(this.config.merges.largeFuzzer);
    let first = new SecureShelf(smallFuzzer.generateContent());
    let second = new SecureShelf(largeFuzzer.generateContent());

    return () => {
      let sv = first.getStateVector();
      let delta = second.getStateDelta(sv);
      if (delta) {
        first.merge(delta);
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
    sizes["Random Merge Update"] = delta.byteLength;

    // With only a single element changed
    const shelf1Updated = new shelf.Awareness(firstValues, 1);
    shelf1Updated.set(["test"], "delta");
    sv = shelf1.getStateVector();
    delta = shelf1Updated.getStateDelta(sv);
    sizes["Single Change Update"] = delta.byteLength;

    // Complete deletion
    const deletionShelf = new shelf.Awareness({ contents: firstValues }, 1);
    const deletionShelfCopy = new shelf.Awareness({ contents: firstValues }, 1);
    deletionShelf.set(["contents"], {});
    sv = deletionShelfCopy.getStateVector();
    delta = deletionShelf.getStateDelta(sv);
    sizes["Complete Deletion Update"] = delta.byteLength;
    return sizes;
  }

  testSizeAfterDeletion(values) {
    const awareness = new shelf.Awareness({ contents: values }, 1);
    awareness.set(["contents"], {});
    return { "Complete Deletion": shelfSizeOf(awareness) };
  }

  testCRDTSize(values) {
    const awareness = new shelf.Awareness(values);
    return shelfSizeOf(awareness); // TODO update
  }

  testNAdditions() {
    let crdt = new shelf.Awareness({ base: 1 }, 1);

    let fuzzer = new Fuzzer(this.config.nAdditions.fuzzerConfig);
    let content = fuzzer.generateContent();

    return () => {
      for (let [key, val] of Object.entries(content)) {
        crdt.set([key], val, 1);
      }
    };
  }

  testMerge() {
    let smallFuzzer = new Fuzzer(this.config.merges.smallFuzzer);
    let largeFuzzer = new Fuzzer(this.config.merges.largeFuzzer);
    let first = new shelf.Awareness(smallFuzzer.generateContent());
    let second = new shelf.Awareness(largeFuzzer.generateContent());

    return () => {
      let sv = first.getStateVector();
      let delta = second.getStateDelta(sv);
      if (delta) {
        first = first.merge(delta);
      }
    };
  }
}

export class SecureShelfBench extends BenchmarkEnvironment {
  constructor() {
    super();
    this.name = "Secure Shelf (ours)";
  }

  testDeltaSize(firstValues, secondValues) {
    const sizes = {};

    const shelf1 = new SecureShelf(firstValues);
    const shelf2 = new SecureShelf(secondValues);
    let sv = shelf1.getStateVector();
    let delta = shelf2.getStateDelta(sv);
    sizes["Random Merge Update"] = delta.byteLength;

    // With only a single element changed
    const shelf1Updated = new SecureShelf(firstValues);
    shelf1Updated.set(["test"], "delta");
    sv = shelf1.getStateVector();
    delta = shelf1Updated.getStateDelta(sv);
    sizes["Single Change Update"] = delta.byteLength;

    // Complete deletion
    const deletionShelf = new SecureShelf({ contents: firstValues });
    const deletionShelfCopy = new SecureShelf({ contents: firstValues });
    deletionShelf.set(["contents"], {});
    sv = deletionShelfCopy.getStateVector();
    delta = deletionShelf.getStateDelta(sv);
    sizes["Complete Deletion Update"] = delta.byteLength;
    return sizes;
  }

  testSizeAfterDeletion(values) {
    const deletionShelf = new SecureShelf({ contents: values });
    deletionShelf.set(["contents"], {});
    return { "Complete Deletion": shelfSizeOf(deletionShelf) };
  }

  testCRDTSize(values) {
    const shelfCRDT = new SecureShelf(values);
    return shelfSizeOf(shelfCRDT);
  }

  testNAdditions() {
    let crdt = new SecureShelf({ base: 1 });

    let fuzzer = new Fuzzer(this.config.nAdditions.fuzzerConfig);
    let content = fuzzer.generateContent();

    return () => {
      for (let [key, val] of Object.entries(content)) {
        crdt.set([key], val);
      }
    };
  }

  testMerge() {
    let smallFuzzer = new Fuzzer(this.config.merges.smallFuzzer);
    let largeFuzzer = new Fuzzer(this.config.merges.largeFuzzer);
    let first = new SecureShelf(smallFuzzer.generateContent());
    let second = new SecureShelf(largeFuzzer.generateContent());

    return () => {
      let sv = first.getStateVector();
      let delta = second.getStateDelta(sv);
      if (delta) {
        first = first.merge(delta);
      }
    };
  }
}
