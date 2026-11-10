/** UUID v7 as BigInt, 74 random bits. */
export declare function genId(): bigint;
/** UUID v7 as canonical string `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`. */
export declare function genIdStr(): string;
/** UUID v7 as 32 lowercase hex chars without dashes. */
export declare function genIdHex(): string;
/** UUID v7 as 16-byte big-endian Buffer. */
export declare function genIdBuffer(): Buffer;
/**
 * Fills `target` with as many 16-byte big-endian UUIDs as fit and returns
 * that count. Trailing bytes that do not fit a whole UUID are left unchanged.
 */
export declare function fillIds(target: Uint8Array): number;

/** Per-thread monotonic UUID v7 as BigInt (56 random bits + 18-bit counter). */
export declare function genIdWithCount(): bigint;
/** Per-thread monotonic UUID v7 as canonical string. */
export declare function genIdWithCountStr(): string;

/** UUID v7 as BigInt with a 4-bit sub-millisecond fraction in `rand_a`. */
export declare function genIdWithSubMs4(): bigint;
/** UUID v7 as BigInt with an 8-bit sub-millisecond fraction in `rand_a`. */
export declare function genIdWithSubMs8(): bigint;
/** UUID v7 as BigInt with a 12-bit sub-millisecond fraction in `rand_a`. */
export declare function genIdWithSubMs12(): bigint;

/** Formats a BigInt or 16-byte buffer as canonical UUID string. */
export declare function formatUuid(id: bigint | Uint8Array): string;

/**
 * Every id is strictly greater than the previous one from the same instance,
 * numerically, lexicographically and byte-wise. The guarantee is per instance.
 */
export declare class SequentialGenerator implements Iterable<bigint> {
  constructor();
  nextId(): bigint;
  nextIdStr(): string;
  nextIdBuffer(): Buffer;
  /** Fills `target` with consecutive ids, see {@link fillIds}. */
  fillIds(target: Uint8Array): number;
  [Symbol.iterator](): Iterator<bigint>;
}

/** Alias for `genIdStr`. */
export declare function uuid7(): string;
/** Alias for `genIdHex`. */
export declare function uuid7Hex(): string;
/** Alias for `genIdBuffer`. */
export declare function uuid7Buffer(): Buffer;
