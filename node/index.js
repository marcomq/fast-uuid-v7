"use strict";

const native = require("./native.js");

const UUID_LEN = 16;

/** UUID v7 as 16-byte big-endian Buffer. */
function genIdBuffer() {
  const id = Buffer.allocUnsafe(UUID_LEN);
  native.fillIds(id);
  return id;
}

/** Strictly increasing UUID v7 ids, guaranteed per instance. */
class SequentialGenerator {
  #handle = native.sequentialCreate();

  /** Next id as BigInt. */
  nextId() {
    return native.sequentialNextId(this.#handle);
  }

  /** Next id as canonical string. */
  nextIdStr() {
    return native.sequentialNextIdStr(this.#handle);
  }

  /** Next id as 16-byte big-endian Buffer. */
  nextIdBuffer() {
    const id = Buffer.allocUnsafe(UUID_LEN);
    native.sequentialFillIds(this.#handle, id);
    return id;
  }

  /** Fills `target` with consecutive 16-byte ids and returns their count. */
  fillIds(target) {
    return native.sequentialFillIds(this.#handle, target);
  }

  /** Infinite iterator of BigInt ids. */
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
