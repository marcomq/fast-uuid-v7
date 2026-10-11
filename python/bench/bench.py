#!/usr/bin/env python3

import argparse
import importlib
import importlib.metadata
import platform
import timeit
import uuid

CALLS = 500_000
REPEAT = 3
# Runner hardware differs between runs, so the Markdown report also gives each
# result relative to this callable from the same run.
REFERENCE = "fastuuidv7.uuid7"

PACKAGE_VERSIONS = [
    ("uuid6", "uuid6"),
    ("uuid-v7", "uuid-v7"),
    ("uuid7", "uuid7"),
    ("uuid-utils", "uuid-utils"),
    ("uuidv7", "uuidv7"),
    ("c-uuid-v7", "c-uuid-v7"),
    ("fastuuid7", "fastuuid7"),
    ("fastuuidv7", "fastuuidv7"),
]


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
        benchmarks.append((name, fn, None))

    uuid7_callable = getattr(uuid, "uuid7", None)
    if callable(uuid7_callable):
        if "uuid.uuid7" not in seen:
            benchmarks.append(("uuid.uuid7", uuid7_callable, None))
    else:
        skipped.append("uuid.uuid7")

    # c-uuid-v7 has no string generator, so str() of its UUID is the counterpart
    # of fastuuidv7.uuid7_str.
    fn = getattr(optional_module("c_uuid_v7"), "uuid7", None)
    if callable(fn):
        benchmarks.append(("str(c_uuid_v7.uuid7())", "str(uuid7())", {"uuid7": fn}))

    local = optional_module("fastuuidv7")
    if local is None:
        skipped.append("fastuuidv7")
    else:
        for attribute_name in ("uuid7", "uuid7_str", "uuid7_hex", "gen_id_str"):
            fn = getattr(local, attribute_name, None)
            name = f"fastuuidv7.{attribute_name}"
            if callable(fn) and name not in seen:
                seen.add(name)
                benchmarks.append((name, fn, None))

    return benchmarks, skipped


def benchmark(name, stmt, namespace=None):
    # Take the fastest of REPEAT runs to reject scheduler/VM noise.
    elapsed = min(timeit.repeat(stmt, number=CALLS, repeat=REPEAT, globals=namespace))
    ops_per_sec = CALLS / elapsed
    ns_per_call = elapsed * 1e9 / CALLS
    print(f"{name:24} {ops_per_sec:12.0f} ops/s  {ns_per_call:10.1f} ns/call")
    return name, ops_per_sec, ns_per_call


def package_versions():
    versions = []
    for label, dist_name in PACKAGE_VERSIONS:
        try:
            version = importlib.metadata.version(dist_name)
        except importlib.metadata.PackageNotFoundError:
            version = "not installed"

        versions.append((label, version))

    return versions


def print_package_versions(versions):
    print("Installed benchmark package versions:")
    for label, version in versions:
        print(f"  {label:<12} {version}")

    print()


def cpu_model():
    # platform.processor() is just the architecture on Linux.
    try:
        with open("/proc/cpuinfo", encoding="utf-8") as cpuinfo:
            for line in cpuinfo:
                if line.startswith("model name"):
                    return line.split(":", 1)[1].strip()
    except OSError:
        pass

    return platform.processor() or platform.machine()


def write_markdown(path, title, versions, results):
    reference = next((ns for name, _, ns in results if name == REFERENCE), None)
    lines = [
        f"## {title}",
        "",
        f"- Platform: `{platform.platform()}`",
        f"- CPU: `{cpu_model()}`",
        f"- Python: `{platform.python_version()}`",
        f"- Packages: {', '.join(f'`{label} {version}`' for label, version in versions)}",
        f"- Fastest of {REPEAT} runs of {CALLS:,} single calls each.",
        "",
        f"| Callable | ops/s | ns/call | vs `{REFERENCE}` |",
        "| --- | ---: | ---: | ---: |",
    ]
    for name, ops_per_sec, ns_per_call in results:
        relative = f"{ns_per_call / reference:.2f}x" if reference else "n/a"
        lines.append(f"| `{name}` | {ops_per_sec:,.0f} | {ns_per_call:.1f} | {relative} |")

    # Append, so several platforms can share one file (e.g. $GITHUB_STEP_SUMMARY).
    with open(path, "a", encoding="utf-8") as output:
        output.write("\n".join(lines) + "\n\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--markdown", metavar="FILE", help="append the results as a Markdown table")
    parser.add_argument("--title", default=f"{platform.system()} {platform.machine()}")
    args = parser.parse_args()

    print(f"Single-call throughput over {CALLS:,} repeated calls")
    print("Includes Python/native boundary overhead when calling extension modules.")
    print()

    versions = package_versions()
    print_package_versions(versions)
    benchmarks, skipped = resolve_benchmarks()
    for name in skipped:
        print(f"Skipping {name}: not installed or no known UUIDv7 callable found")

    if skipped:
        print()

    results = [benchmark(*entry) for entry in benchmarks]

    if args.markdown:
        write_markdown(args.markdown, args.title, versions, results)


if __name__ == "__main__":
    main()
