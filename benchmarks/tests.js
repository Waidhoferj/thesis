import * as shelf from "shelf"

let {Awareness} = shelf;
let a = new Awareness("test")
a.state = {foo: "bar"}
console.assert(true)