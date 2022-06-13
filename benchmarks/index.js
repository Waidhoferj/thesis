
/* benchmark.js */
import * as b from "benny"
import * as awareness from 'y-protocols/awareness.js'
import * as Y from 'yjs'
import * as t from 'lib0/testing'
import * as shelf from "shelf"
let {Fuzzer, Awareness:ShelfAwareness} = shelf
let {Awareness} = awareness


function generateContent(config) {
  let fuzzer = new Fuzzer(config)
  return fuzzer.generateShelfContent()
}




function benchmark() {

// N additions
// size after deletions (pruning)
// batch addition
// Time to encode
// Size of encoding
// Size of crdt
// Size of crdt metadata
//
let benchSettings = {

  B1: {
    n : 1000,
    fuzzerConfig: {seed:1, value_range: [1000,1000+1], depthRange: [1,2], branchRange: [0,1]}
  }
}
b.suite(
  'B1: Test N additions',

  // b.add('ShelfCRDT', () => {
  //   let {n, fuzzerConfig} = benchSettings.B1
  //   let fuzzer = new Fuzzer({seed: fuzzerConfig.seed})

  //   let elementFuzzer = new Fuzzer(fuzzerConfig)
  //   let insertElements = elementFuzzer.generateShelfContent()
  //   let crdt = new ShelfAwareness("1")
  //   crdt.state = insertElements
  //   return () => {
  //     for (let [key, val] of Object.entries(insertElements)) {
  //       crdt.state.set(key,val)
  //     }
  //   }
  // }),

  b.add('Yjs Awareness CRDT', () => {
    let {n, fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer({seed: fuzzerConfig.seed})
    let content = fuzzer.generateShelfContent()
    let doc = new Y.Doc(1)
    let crdt = new Awareness(doc)
    let elementFuzzer = new Fuzzer(fuzzerConfig)
    let insertElements = elementFuzzer.generateShelfContent()
    crdt.setLocalState({})
    return () => {
      for (let [key, val] of Object.entries(insertElements)) {
        crdt.setLocalStateField(key,val)
      }
    }

  }),

  b.cycle(),
  b.complete(),
  b.save({ file: 'reduce', version: '1.0.0' }),
  b.save({ file: 'reduce', format: 'chart.html' }),
)


// b.suite(
//   'B2: Size of encoding',

//   b.add('ShelfCRDT', () => {
    
//   }),
//   b.add('Yjs Awareness CRDT', () => {
//     let fuzzer = new Fuzzer({seed: 1})
//     let content = fuzzer.generateShelfContent()
//     let doc = new Y.Doc(1)
//     let crdt = new Awareness(doc)
//     crdt.setLocalState(content)
//   }),

//   b.cycle(),
//   b.complete(),
// )
}

function test() {
  const doc1 = new Y.Doc()
  doc1.clientID = 0
  const doc2 = new Y.Doc()
  doc2.clientID = 1
  const aw1 = new awareness.Awareness(doc1)
  const aw2 = new awareness.Awareness(doc2)
  aw1.on('update', /** @param {any} p */ ({ added, updated, removed }) => {
    const enc = awareness.encodeAwarenessUpdate(aw1, added.concat(updated).concat(removed))
    awareness.applyAwarenessUpdate(aw2, enc, 'custom')
  })
  let lastChangeLocal = /** @type {any} */ (null)
  aw1.on('change', /** @param {any} change */ change => {
    lastChangeLocal = change
  })
  let lastChange = /** @type {any} */ (null)
  aw2.on('change', /** @param {any} change */ change => {
    lastChange = change
  })
  aw1.setLocalState({ x: 3 })
  t.compare(aw2.getStates().get(0), { x: 3 })
  t.assert(/** @type {any} */ (aw2.meta.get(0)).clock === 1)
  t.compare(lastChange.added, [0])
  // When creating an Awareness instance, the the local client is already marked as available, so it is not updated.
  t.compare(lastChangeLocal, { added: [], updated: [0], removed: [] })

  // update state
  lastChange = null
  lastChangeLocal = null
  aw1.setLocalState({ x: 4 })
  t.compare(aw2.getStates().get(0), { x: 4 })
  t.compare(lastChangeLocal, { added: [], updated: [0], removed: [] })
  t.compare(lastChangeLocal, lastChange)

  lastChange = null
  lastChangeLocal = null
  aw1.setLocalState({ x: 4 })
  t.assert(lastChange === null)
  t.assert(/** @type {any} */ (aw2.meta.get(0)).clock === 3)
  t.compare(lastChangeLocal, lastChange)
  aw1.setLocalState(null)
  t.assert(lastChange.removed.length === 1)
  t.compare(aw1.getStates().get(0), undefined)
  t.compare(lastChangeLocal, lastChange)

}

benchmark()