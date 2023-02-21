import BenchmarkEnvironment from "./base.js";
import * as Y from "yjs";
import * as awareness from "y-protocols/awareness.js";
let { Awareness } = awareness;
import * as sizeOfMod from "object-sizeof";
const sizeOf = sizeOfMod.default;

import * as shelf from "shelf";

let { Fuzzer } = shelf;

export default class YjsAwarenessBench extends BenchmarkEnvironment {
  constructor() {
    super();
    this.name = "Yjs Awareness";
  }
  testDeltaSize(firstValues, secondValues) {
    const sizes = {};
    // Test Yjs
    const doc1 = new Y.Doc();
    doc1.clientID = 0;
    const aw1 = new awareness.Awareness(doc1);
    aw1.setLocalState(firstValues);
    let enc = awareness.encodeAwarenessUpdate(
      aw1,
      Array.from(aw1.getStates().keys())
    );
    sizes["Random Merge Update"] = enc.byteLength;
    aw1.setLocalStateField("test", "delta");
    enc = awareness.encodeAwarenessUpdate(
      aw1,
      [0] // Only encode this client, closest to delta
    );
    sizes["Single Change Update"] = enc.byteLength;

    // complete deletion
    aw1.setLocalState({});
    enc = awareness.encodeAwarenessUpdate(aw1, [0]);
    sizes["Complete Deletion Update"] = enc.byteLength;

    return sizes;
  }

  yjsSizeOf(awareness) {
    // sizeOf does not account for the data stored by yjs awareness, so it must be dereferenced and logged explicitly.
    return (
      sizeOf(awareness) + // metadata associated with the CRDT
      sizeOf(Object.fromEntries(awareness.getStates().entries())) // representation of the contents
    );
  }

  testSizeAfterDeletion(values) {
    const doc1 = new Y.Doc();
    doc1.clientID = 0;
    const yjsAwareness = new awareness.Awareness(doc1);
    yjsAwareness.setLocalState({ contents: values });
    let size = sizeOf(yjsAwareness);
    yjsAwareness.setLocalState({ contents: {} });

    return {
      "Complete Deletion": this.yjsSizeOf(yjsAwareness),
    };
  }

  testCRDTSize(values) {
    const yDoc = new Y.Doc();
    yDoc.clientID = 0;
    const yjsAwareness = new awareness.Awareness(yDoc);
    yjsAwareness.setLocalState(values);
    return this.yjsSizeOf(yjsAwareness);
  }

  testNAdditions() {
    let fuzzer = new Fuzzer(this.config.nAdditions.fuzzerConfig);
    let doc = new Y.Doc(1);
    let crdt = new Awareness(doc);
    let insertElements = fuzzer.generateContent();
    crdt.setLocalState({ base: 1 });
    return () => {
      for (let [key, val] of Object.entries(insertElements)) {
        crdt.setLocalStateField(key, val);
      }
    };
  }

  testMerge() {
    let smallFuzzer = new Fuzzer(this.config.merges.smallFuzzer);
    let largeFuzzer = new Fuzzer(this.config.merges.largeFuzzer);
    const doc1 = new Y.Doc();
    doc1.clientID = 0;
    const doc2 = new Y.Doc();
    doc2.clientID = 1;

    const aw1 = new awareness.Awareness(doc1);
    aw1.setLocalState(largeFuzzer.generateContent());
    const aw2 = new awareness.Awareness(doc2);
    aw2.setLocalState(smallFuzzer.generateContent());

    return () => {
      const enc = awareness.encodeAwarenessUpdate(
        aw1,
        Array.from(aw1.getStates().keys())
      );
      awareness.applyAwarenessUpdate(aw2, enc, "custom");
    };
  }
}
