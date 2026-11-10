"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { Worker } = require("node:worker_threads");
const lib = require("..");

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function assertV7(id) {
  assert.equal(typeof id, "bigint");
  assert.equal((id >> 76n) & 0xfn, 7n, "version");
  assert.equal((id >> 62n) & 0x3n, 2n, "variant");
}

function timestampMs(id) {
  return Number(id >> 80n);
}

test("bigint generators produce UUID v7 layout", () => {
  for (const gen of [
    lib.genId,
    lib.genIdWithCount,
    lib.genIdWithSubMs4,
    lib.genIdWithSubMs8,
    lib.genIdWithSubMs12,
  ]) {
    assertV7(gen());
  }
});

test("timestamp is close to Date.now()", () => {
  const before = Date.now();
  const ms = timestampMs(lib.genId());
  const after = Date.now();
  assert.ok(ms >= before - 2 && ms <= after + 2, `${ms} not in [${before}, ${after}]`);
});

test("string generators use canonical lowercase format", () => {
  assert.match(lib.genIdStr(), UUID_RE);
  assert.match(lib.genIdWithCountStr(), UUID_RE);
  assert.match(lib.uuid7(), UUID_RE);
  assert.match(lib.genIdHex(), /^[0-9a-f]{12}7[0-9a-f]{3}[89ab][0-9a-f]{15}$/);
});

test("buffer and string forms agree", () => {
  const buf = lib.genIdBuffer();
  assert.ok(Buffer.isBuffer(buf));
  assert.equal(buf.length, 16);
  const hex = buf.toString("hex");
  assert.equal(lib.formatUuid(buf).replaceAll("-", ""), hex);
  assert.equal(lib.formatUuid(BigInt(`0x${hex}`)), lib.formatUuid(buf));
});

test("formatUuid matches a known value", () => {
  const id = 0x0123456789abcdef0123456789abcdefn;
  assert.equal(lib.formatUuid(id), "01234567-89ab-cdef-0123-456789abcdef");
});

test("formatUuid rejects invalid input", () => {
  assert.throws(() => lib.formatUuid(-1n));
  assert.throws(() => lib.formatUuid(1n << 128n));
  assert.throws(() => lib.formatUuid(Buffer.alloc(15)));
});

test("ids are unique", () => {
  const ids = new Set();
  for (let i = 0; i < 100_000; i++) ids.add(lib.genIdStr());
  assert.equal(ids.size, 100_000);
});

test("genIdWithCount is monotonic on one thread", () => {
  let prev = lib.genIdWithCount();
  for (let i = 0; i < 100_000; i++) {
    const next = lib.genIdWithCount();
    assert.ok(next > prev);
    prev = next;
  }
});

test("fillIds fills whole UUIDs and leaves the remainder untouched", () => {
  const buf = Buffer.alloc(16 * 3 + 5, 0xff);
  assert.equal(lib.fillIds(buf), 3);
  for (let i = 0; i < 3; i++) {
    assertV7(BigInt(`0x${buf.subarray(i * 16, i * 16 + 16).toString("hex")}`));
  }
  assert.deepEqual([...buf.subarray(48)], [0xff, 0xff, 0xff, 0xff, 0xff]);
  assert.equal(lib.fillIds(Buffer.alloc(8)), 0);
});

test("SequentialGenerator is strictly increasing in every form", () => {
  const gen = new lib.SequentialGenerator();
  let prevInt = gen.nextId();
  let prevStr = gen.nextIdStr();
  let prevBuf = gen.nextIdBuffer();
  for (let i = 0; i < 100_000; i++) {
    const int = gen.nextId();
    const str = gen.nextIdStr();
    const buf = gen.nextIdBuffer();
    assert.ok(int > prevInt);
    assert.ok(str > prevStr);
    assert.ok(Buffer.compare(buf, prevBuf) > 0);
    prevInt = int;
    prevStr = str;
    prevBuf = buf;
  }
});

test("genIdBuffer stays unique across pool refills", () => {
  const ids = new Set();
  for (let i = 0; i < 2_000; i++) {
    const buf = lib.genIdBuffer();
    assert.equal(buf.length, 16);
    ids.add(buf.toString("hex"));
  }
  assert.equal(ids.size, 2_000);
});

test("SequentialGenerator fillIds and iterator continue the sequence", () => {
  const gen = new lib.SequentialGenerator();
  const first = gen.nextId();
  const buf = Buffer.alloc(16 * 4);
  assert.equal(gen.fillIds(buf), 4);
  const filled = [0, 1, 2, 3].map((i) => BigInt(`0x${buf.subarray(i * 16, i * 16 + 16).toString("hex")}`));
  const iterated = [];
  for (const id of gen) {
    iterated.push(id);
    if (iterated.length === 3) break;
  }
  const all = [first, ...filled, ...iterated];
  for (let i = 1; i < all.length; i++) assert.ok(all[i] > all[i - 1]);
});

test("worker threads generate distinct ids", async () => {
  const run = () =>
    new Promise((resolve, reject) => {
      const worker = new Worker(
        `const { parentPort } = require("node:worker_threads");
         const lib = require(${JSON.stringify(require.resolve(".."))});
         parentPort.postMessage(Array.from({ length: 10000 }, () => lib.genIdStr()));`,
        { eval: true },
      );
      worker.once("message", resolve);
      worker.once("error", reject);
    });
  const results = await Promise.all([run(), run(), run(), run()]);
  const ids = new Set(results.flat());
  assert.equal(ids.size, 40_000);
});
