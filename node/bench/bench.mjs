import { randomUUID } from "node:crypto";
import { createRequire } from "node:module";
import { v7 } from "uuid";
import { uuidv7 } from "uuidv7";

const lib = createRequire(import.meta.url)("../index.js");

const N = Number(process.argv[2] ?? 5_000_000);
const BATCH = 1024;
const batchBuf = Buffer.alloc(16 * BATCH);

const cases = [
  ["fast-uuid-v7 genIdStr", lib.genIdStr],
  ["fast-uuid-v7 genIdHex", lib.genIdHex],
  ["fast-uuid-v7 genIdWithCountStr", lib.genIdWithCountStr],
  ["fast-uuid-v7 genIdBuffer", lib.genIdBuffer],
  ["fast-uuid-v7 genId (bigint)", lib.genId],
  [`fast-uuid-v7 fillIds (per id, batch ${BATCH})`, () => lib.fillIds(batchBuf), BATCH],
  ["SequentialGenerator.nextIdStr", ((g) => () => g.nextIdStr())(new lib.SequentialGenerator())],
  ["uuid v7()", v7],
  ["uuidv7 uuidv7()", uuidv7],
  ["crypto.randomUUID() (v4)", randomUUID],
];

console.log(`node ${process.version}, ${process.platform}-${process.arch}, ${N} ids per case`);
for (const [name, fn, perCall = 1] of cases) {
  const calls = Math.ceil(N / perCall);
  for (let i = 0; i < Math.min(calls, 200_000); i++) fn();
  let sink;
  const start = process.hrtime.bigint();
  for (let i = 0; i < calls; i++) sink = fn();
  const ns = Number(process.hrtime.bigint() - start) / (calls * perCall);
  if (sink === undefined) throw new Error(name);
  console.log(`${name.padEnd(40)} ${ns.toFixed(1).padStart(8)} ns/id`);
}
