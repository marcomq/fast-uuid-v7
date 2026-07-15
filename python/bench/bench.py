#!/usr/bin/env python3

import importlib
import importlib.metadata
import timeit
import uuid

CALLS = 500_000


def optional_module(name):
    try:
        return importlib.import_module(name)
    except ImportError:
        return None


def optional_callable(module_name, attribute_names):
    module = optional_module(module_name)
    if module is None:
        return None

    for attribute_name in attribute_names:
        candidate = getattr(module, attribute_name, None)
        if callable(candidate):
            return f"{module_name}.{attribute_name}", candidate

    return None


def has_distribution(name):
    try:
        importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return False
    return True


def resolve_optional_benchmark(label, module_candidates, required_distribution=None):
    if required_distribution is not None and not has_distribution(required_distribution):
        return None

    for module_name, attribute_names in module_candidates:
        candidate = optional_callable(module_name, attribute_names)
        if candidate is None:
            continue

        candidate_name, fn = candidate
        if candidate_name == label or candidate_name.startswith(f"{label}."):
            return candidate
        return f"{label} via {candidate_name}", fn

    return None


def resolve_benchmarks():
    skipped = []
    benchmarks = []
    seen = set()

    for label, required_distribution, module_candidates in [
        ("uuid6", "uuid6", [("uuid6", ["uuid7"])]),
        ("uuid-v7", "uuid-v7", [("uuid_v7", ["uuid7"]), ("uuid_v7.base", ["uuid7"])]),
        ("uuid7", "uuid7", [("uuid7", ["uuid7"]), ("uuid_extensions", ["uuid7", "uuid7str"])]),
        ("uuid-utils", "uuid-utils", [("uuid_utils", ["uuid7"])]),
        ("uuidv7", "uuidv7", [("uuidv7", ["uuid7", "uuidv7", "generate"])]),
        ("c-uuid-v7", "c-uuid-v7", [("c_uuid_v7", ["uuid7"])]),
        (
            "fastuuid7",
            "fastuuid7",
            [
                ("fastuuid7", ["uuid7", "generate", "uuid"]),
                ("uuidv7", ["uuid7", "uuidv7", "generate"]),
            ],
        ),
    ]:
        candidate = resolve_optional_benchmark(
            label,
            module_candidates,
            required_distribution=required_distribution,
        )
        if candidate is None:
            skipped.append(label)
            continue

        name, fn = candidate
        if name in seen:
            continue

        seen.add(name)
        benchmarks.append((name, fn))

    local = optional_module("fastuuidv7")
    if local is None:
        skipped.append("fastuuidv7")
    else:
        for attribute_name in ("uuid7", "uuid7_str", "uuid7_hex", "gen_id_str"):
            fn = getattr(local, attribute_name, None)
            name = f"fastuuidv7.{attribute_name}"
            if callable(fn) and name not in seen:
                seen.add(name)
                benchmarks.append((name, fn))

    if hasattr(uuid, "uuid7"):
        if "uuid.uuid7" not in seen:
            benchmarks.append(("uuid.uuid7", uuid.uuid7))
    else:
        skipped.append("uuid.uuid7")

    return benchmarks, skipped


def benchmark(name, fn):
    elapsed = timeit.timeit(fn, number=CALLS)
    ops_per_sec = CALLS / elapsed
    ns_per_call = elapsed * 1e9 / CALLS
    print(f"{name:24} {ops_per_sec:12.0f} ops/s  {ns_per_call:10.1f} ns/call")


def main():
    print(f"Single-call throughput over {CALLS:,} repeated calls")
    print("Includes Python/native boundary overhead when calling extension modules.")
    print()

    benchmarks, skipped = resolve_benchmarks()
    for name in skipped:
        print(f"Skipping {name}: not installed or no known UUIDv7 callable found")

    if skipped:
        print()

    for name, fn in benchmarks:
        benchmark(name, fn)


if __name__ == "__main__":
    main()
