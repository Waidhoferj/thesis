
/* benchmark.js */
import * as b from "benny"
import * as sizeOfMod from "object-sizeof"
const sizeOf = sizeOfMod.default
import * as awareness from 'y-protocols/awareness.js'
import * as Y from 'yjs'
import * as t from 'lib0/testing'
import * as shelf from "shelf"
let {Fuzzer, Awareness:ShelfAwareness, Shelf} = shelf
let {Awareness} = awareness
import * as fs from "fs"
import * as Automerge from 'automerge'



// N additions
// size after deletions (pruning)
// batch addition
// Time to encode
// Size of encoding
// Size of crdt
// Size of crdt metadata


function testMemoryFootprint() {
  testDeltaSize()
  testSizeAfterDeletion()
  testCRDTSize()
}
/**
 * Fuzzer generates two random JSON trees. Trees are wrapped in 
 */
function testDeltaSize() {
  const shelfCRDT= []
  const yjs = []
  const automerge = []
  const N = 1000
  const fuzzer = new Fuzzer({seed:1, valueRange: [100,200], depthRange: [3,5], branchRange: [1,5]})

    const doc1 = new Y.Doc()
    doc1.clientID = 0
    const doc2 = new Y.Doc()
    doc2.clientID = 1
    const aw1 = new awareness.Awareness(doc1)
    const aw2 = new awareness.Awareness(doc2)

    for(let i = 0; i < N; i++){
    const firstValues = fuzzer.generateContent()
    const secondValues = fuzzer.generateContent()
    if (deepEq(firstValues,secondValues)) {
      continue // Not a fair comparison if all the values are the same. We only count deltas that actually exist.
    }

    // Test Yjs
    aw1.setLocalState(firstValues)
    aw2.setLocalState(secondValues)
    const enc = awareness.encodeAwarenessUpdate(aw2, Array.from(aw2.getStates().keys()))
    yjs.push(enc.byteLength)

    // Test Shelf
    const shelf1 = new Shelf(firstValues)
    const shelf2 = new Shelf(secondValues)
    const sv = shelf1.encodeStateVector()
    const delta = shelf2.encodeStateDelta(sv)

    shelfCRDT.push(delta.byteLength)

    // Test Automerge
    let autoDoc = Automerge.init()
    autoDoc = Automerge.change(autoDoc, 'Set state', doc => {
      doc.contents = secondValues
    })
    const encodedState = Automerge.save(autoDoc)
    automerge.push(encodedState.byteLength)
    }

    const report = {
      shelfCRDT,
      yjs,
      automerge
    }

    fs.writeFileSync("benchmark/results/delta-size.json", JSON.stringify(report))

}

function testSizeAfterDeletion() {
  // Yjs


  // Shelf

  //Automerge

  console.log("TODO: testSizeAfterDeletion")
}

/**
 * How do the sizes of the CRDTs compare? Maybe stringify each?
 * This should be done manually. TODO: sizeOf is probably incorrect. Currently a placeholder
 */
function testCRDTSize() {
  const N = 1000
  const fuzzer = new Fuzzer({seed:1, valueRange: [100,200], depthRange: [3,6], branchRange: [1,5]})
    fuzzer.setSeed(1)
    const yDoc = new Y.Doc()
    yDoc.clientID = 0
    const yjsAwareness = new awareness.Awareness(yDoc)
    let values = fuzzer.generateContent()
    const valueSize = sizeOf(values)
    yjsAwareness.setLocalState(values)
   const yjsSize = sizeOf(yjsAwareness)

   const shelfCRDT = new Shelf(values)
   const shelfSize = sizeOf(shelfCRDT)

   let autoDoc = Automerge.init()
    autoDoc = Automerge.change(autoDoc, 'Set state', doc => {
      doc.contents = values
    })
   const automergeSize = sizeOf(autoDoc)


   const report = {
    shelfCRDT: [shelfSize],
    yjs: [yjsSize],
    automerge: [automergeSize],
    values: [valueSize]
  }

  fs.writeFileSync("benchmark/results/crdt-size.json", JSON.stringify(report))

   console.log("TODO: testCRDTSize")
}

function benchmark() {
let benchSettings = {

  B1: {
    n : 1000,
    fuzzerConfig: {seed:1, valueRange: [1000,1000+1], depthRange: [0,1], branchRange: [0,5]}
  }
}
b.suite(
  'B1: Test N additions',

  b.add('ShelfCRDT', () => {
    let {n, fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer(fuzzerConfig)
    let content = fuzzer.generateContent()
    let insertElements = fuzzer.generateContent()
    let crdt = new Shelf(content)

    return () => {
      for (let [key, val] of Object.entries(insertElements)) {
        crdt.set([key],val)
      }
    }
  }),

  b.add('Yjs Awareness CRDT', () => {
    let {n, fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer(fuzzerConfig)
    let content = fuzzer.generateContent()
    let doc = new Y.Doc(1)
    let crdt = new Awareness(doc)
    let insertElements = fuzzer.generateContent()
    crdt.setLocalState(content)
    return () => {
      for (let [key, val] of Object.entries(insertElements)) {
        crdt.setLocalStateField(key,val)
      }
    }

  }),


  b.add('Automerge', () => {
    let {n, fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer(fuzzerConfig)
    let content = fuzzer.generateContent()
    let insertElements = fuzzer.generateContent()
    let autoDoc = Automerge.init()
    autoDoc = Automerge.change(autoDoc, 'Set initial state', doc => {
      doc.contents = content
    })

    return () => {
      for (let [key, val] of Object.entries(insertElements)) {
        autoDoc = Automerge.change(autoDoc, key, doc => {
          doc[key] = val
        })
      }
    }

  }),

  b.cycle(),
  b.complete(),
  b.save({ file: 'additions', version: '1.0.0' }),
  b.save({ file: 'additions', format: 'chart.html' }),
)

b.suite(
  'B2: Test Merge',

  b.add('ShelfCRDT', () => {
    let {fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer(fuzzerConfig)
    let first = new Shelf(fuzzer.generateContent())
    let second = new Shelf(fuzzer.generateContent())

    return () => {
      let sv = first.encodeStateVector()
      let delta = second.encodeStateDelta(sv)
      if (delta) {
        first.merge(delta)
      } else {
        throw Error("There should be a delta for the shelfCRDT")
      }
      
    }
  }),

  b.add('Yjs Awareness CRDT', () => {
    let {n, fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer(fuzzerConfig)
    const doc1 = new Y.Doc()
    doc1.clientID = 0
    const doc2 = new Y.Doc()
    doc2.clientID = 1
    const aw1 = new awareness.Awareness(doc1)
    aw1.setLocalState(fuzzer.generateContent())
    const aw2 = new awareness.Awareness(doc2)
    aw2.setLocalState(fuzzer.generateContent())

    return () => {
      const enc = awareness.encodeAwarenessUpdate(aw1, Array.from(aw1.getStates().keys()))
      awareness.applyAwarenessUpdate(aw2, enc, 'custom')
    }

  }),

  b.add('Automerge', () => {
    let {n, fuzzerConfig} = benchSettings.B1
    let fuzzer = new Fuzzer(fuzzerConfig)
    let first = new Shelf(fuzzer.generateContent())
    let second = new Shelf(fuzzer.generateContent())
    let firstDoc = Automerge.init()
    let secondDoc = Automerge.init()
    firstDoc = Automerge.change(firstDoc, "", doc => {
      doc.contents = first
    })

    secondDoc = Automerge.change(secondDoc, "", doc => {
      doc.contents = second
    })

    return () => {
      const encodedState = Automerge.save(firstDoc)
      const update = Automerge.load(encodedState)
      Automerge.merge(update, secondDoc)
    }

  }),

  b.cycle(),
  b.complete(),
  b.save({ file: 'merges', version: '1.0.0' }),
  b.save({ file: 'merges', format: 'chart.html' }),
)
}

function test() {
  const doc1 = new Y.Doc()
  doc1.clientID = 0
  const doc2 = new Y.Doc()
  doc2.clientID = 1
  const aw1 = new awareness.Awareness(doc1)
  const aw2 = new awareness.Awareness(doc2)
  aw1.on('update', /** @param {any} p */ ({ added, updated, removed }) => {
    const enc = awareness.encodeAwarenessUpdate(aw1, Array.from(awareness.getStates().keys()))
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

function generateReports() {
  benchmark()
  testMemoryFootprint()
}

function deepEq(ob1, ob2) {
  return JSON.stringify(ob1) == JSON.stringify(ob2)
}

generateReports()