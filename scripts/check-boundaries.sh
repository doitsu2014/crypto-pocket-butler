#!/usr/bin/env bash
# Clean Architecture boundary gate.
#
# Reads `cargo metadata` and fails when a crate depends outward:
#   *-domain          no local deps; no web, storage, HTTP, or serialization crate
#   *-application     local deps are *-domain only
#   *-infrastructure  local deps are *-domain / *-application only; no Axum
#   *-http            local deps are *-domain / *-application only
#   service package   anything (it is the composition root)
#
# Only normal dependencies are checked; test-only crates (Axum in an HTTP test,
# sqlx in an adapter test) are expected and allowed.
set -euo pipefail

cd "$(dirname "$0")/.."

python3 - <<'PY'
import json
import subprocess
import sys

metadata = json.loads(
    subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"], text=True
    )
)
packages = {package["name"]: package for package in metadata["packages"]}
local = set(packages)

FORBIDDEN_IN_DOMAIN = {
    "axum",
    "sqlx",
    "reqwest",
    "tower",
    "tower-http",
    "http-body-util",
    "serde",
    "serde_json",
}
FORBIDDEN_IN_APPLICATION = {"axum", "sqlx", "reqwest", "tower", "tower-http"}
FORBIDDEN_IN_INFRASTRUCTURE = {"axum", "tower-http"}

problems = []


def normal_dependencies(package):
    return [
        dependency["name"]
        for dependency in package["dependencies"]
        if dependency.get("kind") is None
    ]


def check(name, package):
    dependencies = normal_dependencies(package)
    local_deps = [dependency for dependency in dependencies if dependency in local]

    if name.endswith("-domain"):
        if local_deps:
            problems.append(f"{name} must not depend on a sibling crate: {local_deps}")
        forbidden = sorted(set(dependencies) & FORBIDDEN_IN_DOMAIN)
        if forbidden:
            problems.append(f"{name} must not import framework crates: {forbidden}")
    elif name.endswith("-application"):
        unexpected = [
            dependency
            for dependency in local_deps
            if not dependency.endswith("-domain")
        ]
        if unexpected:
            problems.append(
                f"{name} may depend only on a domain crate, not: {unexpected}"
            )
        forbidden = sorted(set(dependencies) & FORBIDDEN_IN_APPLICATION)
        if forbidden:
            problems.append(f"{name} must not import adapter crates: {forbidden}")
    elif name.endswith("-infrastructure"):
        unexpected = [
            dependency
            for dependency in local_deps
            if not (
                dependency.endswith("-domain")
                or dependency.endswith("-application")
            )
        ]
        if unexpected:
            problems.append(
                f"{name} may depend only on domain/application crates, not: {unexpected}"
            )
        forbidden = sorted(set(dependencies) & FORBIDDEN_IN_INFRASTRUCTURE)
        if forbidden:
            problems.append(f"{name} must not import the HTTP boundary: {forbidden}")
    elif name.endswith("-http"):
        unexpected = [
            dependency
            for dependency in local_deps
            if not (
                dependency.endswith("-domain")
                or dependency.endswith("-application")
            )
        ]
        if unexpected:
            problems.append(
                f"{name} may depend only on domain/application crates, not: {unexpected}"
            )


for name, package in packages.items():
    check(name, package)

if problems:
    print("boundary check FAILED:", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    sys.exit(1)

print(f"boundaries OK ({len(packages)} packages checked)")
PY
