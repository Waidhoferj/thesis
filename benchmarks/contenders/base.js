export default class BenchmarkEnvironment {
  constructor() {
    this.name = "BASE";
    this.fuzzerConfig = {
      seed: 42,
      valueRange: [300, 500],
      depthRange: [3, 5],
      branchRange: [1, 4],
    };

    this.config = {
      nAdditions: {
        fuzzerConfig: {
          seed: 42,
          valueRange: [9000, 9000 + 1],
          depthRange: [0, 1],
          branchRange: [0, 5],
        },
      },
      merges: {
        smallFuzzer: {
          seed: 37,
          valueRange: [300, 500],
          depthRange: [2, 4],
          branchRange: [0, 3],
        },
        largeFuzzer: {
          seed: 42,
          valueRange: [300, 500],
          depthRange: [3, 5],
          branchRange: [1, 5],
        },
      },
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
