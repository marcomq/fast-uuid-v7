"use strict";

const native = require("./native.js");

const UUID_LEN = 16;
const POOL_SIZE = UUID_LEN * 256;

// Returned buffers are views into a pre-filled pool that is never written
// again, like the small buffers of Node's own `Buffer.allocUnsafe`.
let pool = Buffer.alloc(0);
let poolOffset = 0;

function genIdBuffer() {
  if (poolOffset === pool.length) {
    pool = Buffer.allocUnsafeSlow(POOL_SIZE);
    native.fillIds(pool);
    poolOffset = 0;
  }
  const id = pool.subarray(poolOffset, poolOffset + UUID_LEN);
  poolOffset += UUID_LEN;
  return id;
}

class SequentialGenerator {
  #handle = native.sequentialCreate();

  nextId() {
    return native.sequentialNextId(this.#handle);
  }

  nextIdStr() {
    return native.sequentialNextIdStr(this.#handle);
  }

  nextIdBuffer() {
    const id = Buffer.allocUnsafe(UUID_LEN);
    native.sequentialFillIds(this.#handle, id);
    return id;
  }

  fillIds(target) {
    return native.sequentialFillIds(this.#handle, target);
  }

  *[Symbol.iterator]() {
    for (;;) yield this.nextId();
  }
}

// Plain `exports.x =` assignments so ESM `import { x }` can detect them.
exports.genId = native.genId;
exports.genIdStr = native.genIdStr;
exports.genIdHex = native.genIdHex;
exports.genIdBuffer = genIdBuffer;
exports.fillIds = native.fillIds;
exports.genIdWithCount = native.genIdWithCount;
exports.genIdWithCountStr = native.genIdWithCountStr;
exports.genIdWithSubMs4 = native.genIdWithSubMs4;
exports.genIdWithSubMs8 = native.genIdWithSubMs8;
exports.genIdWithSubMs12 = native.genIdWithSubMs12;
exports.formatUuid = native.formatUuid;
exports.SequentialGenerator = SequentialGenerator;
exports.uuid7 = native.genIdStr;
exports.uuid7Hex = native.genIdHex;
exports.uuid7Buffer = genIdBuffer;
