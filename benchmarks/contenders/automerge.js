import BenchmarkEnvironment from "./base.js";
import * as Automerge from "automerge";
import * as sizeOfMod from "object-sizeof";
const sizeOf = sizeOfMod.default;

import * as shelf from "shelf";

let { Fuzzer } = shelf;

export default class AutomergeBench extends BenchmarkEnvironment {
  constructor() {
    super();
    this.name = "Automerge";
  }
  testDeltaSize(firstValues, secondValues) {
    const sizes = {};
    let autoDoc = Automerge.init();
    autoDoc = Automerge.change(autoDoc, "Set state", (doc) => {
      doc.contents = secondValues;
    });

    let encodedState = getLastAutomergeChange(autoDoc);
    sizes["Random Merge"] = encodedState.byteLength;

    autoDoc = Automerge.change(autoDoc, "Single update", (doc) => {
      doc.contents.test = "delta";
    });
    encodedState = getLastAutomergeChange(autoDoc);
    sizes["Single Change"] = encodedState.byteLength;

    // Complete Deletion
    autoDoc = Automerge.init();
    autoDoc = Automerge.change(autoDoc, "add elements", (doc) => {
      doc.contents = firstValues;
    });
    autoDoc = Automerge.change(autoDoc, "add elements", (doc) => {
      doc.contents = {};
    });
    encodedState = getLastAutomergeChange(autoDoc);
    sizes["Complete Deletion"] = encodedState.byteLength;

    return sizes;
  }

  testSizeAfterDeletion(values) {
    // Test Automerge
    let autoDoc = Automerge.init();
    autoDoc = Automerge.change(autoDoc, "add elements", (doc) => {
      doc.contents = values;
    });
    autoDoc = Automerge.change(autoDoc, "delete all elements", (doc) => {
      doc.contents = {};
    });
    return {
      "Complete Deletion": sizeOf(autoDoc),
    };
  }

  testCRDTSize(values) {
    let autoDoc = Automerge.init();
    autoDoc = Automerge.change(autoDoc, "Set state", (doc) => {
      doc.contents = values;
    });
    return sizeOf(autoDoc);
  }

  testNAdditions() {
    let fuzzer = new Fuzzer(this.fuzzerConfig);
    let content = fuzzer.generateContent();
    let insertElements = fuzzer.generateContent();
    let autoDoc = Automerge.init();
    autoDoc = Automerge.change(autoDoc, "Set initial state", (doc) => {
      doc.contents = content;
    });

    return () => {
      for (let [key, val] of Object.entries(insertElements)) {
        autoDoc = Automerge.change(autoDoc, key, (doc) => {
          doc[key] = val;
        });
      }
    };
  }

  testMerge() {
    let fuzzer = new Fuzzer(this.fuzzerConfig);
    let first = fuzzer.generateContent();
    let second = fuzzer.generateContent();
    let firstDoc = Automerge.init();
    let secondDoc = Automerge.init();
    firstDoc = Automerge.change(firstDoc, "", (doc) => {
      doc.contents = first;
    });

    secondDoc = Automerge.change(secondDoc, "", (doc) => {
      doc.contents = second;
    });

    return () => {
      const encodedState = Automerge.save(firstDoc);
      const update = Automerge.load(encodedState);
      Automerge.merge(update, secondDoc);
    };
  }
}

function getLastAutomergeChange(doc) {
  let changes = Automerge.getAllChanges(doc);
  return changes[changes.length - 1];
}
