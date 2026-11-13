#!/usr/bin/env node
export interface Shape { area(): number }
export class Square implements Shape { constructor(public side: number) {} area(): number { return this.side ** 2; } }
const square = new Square(2);
enum Color { Red = "red", Blue }
let value = "before"; value = "after";
const Local = class { accessor count = 0; field: string = "value"; method(x: number): void; method(x: number) {} };
const {left: renamed = "default", ...rest} = {left: "x"};
const f = (x: string = "hi") => `hello ${x}`;
const asserted = <string>value;

async function asynchronous() { await g(); }
a.if; ({ default: 1 }); x.null; x.true; class C { static() {} }
