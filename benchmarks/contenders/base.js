export default class BenchmarkEnvironment {
  constructor() {
    this.name = "BASE";
    this.fuzzerConfig = {
      seed: 42,
      valueRange: [1000, 1000 + 1],
      depthRange: [0, 1],
      branchRange: [0, 5],
    };
  }

  testDeltaSize(firstValues, secondValues) {
    throw new Error("unimplemented testDeltaSize");
  }

  testSizeAfterDeletion(values) {
    throw new Error("unimplemented testSizeAfterDeletion");
  }

  testCRDTSize(values) {
    throw new Error("unimplemented testCRDTSize");
  }

  testNAdditions() {
    throw new Error("unimplemented testNAdditions");
  }

  testMerge() {
    throw new Error("unimplemented testMerge");
  }
}
