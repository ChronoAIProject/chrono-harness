#!/usr/bin/env python3
"""Candidate decoder for explicitly registered chrono-ci-check/v1 / FILEMAP v1 data.
No discovery: ordered bindings and exact legacy definitions are conversion inputs.
"""
import copy
import json
import sys


def convert(request):
    if request["schema"] != "chrono-historical-decode/v1":
        raise ValueError("unsupported decoder protocol")
    profile = request["profile"]
    old = request["original"]
    config = old[request["config_path"]]
    fm_path = config["registries"]["filemap"]
    pr_path = config["registries"]["projects"]
    scoped = request["profile_value"]
    if (old[fm_path]["schema_version"] != 1
            or scoped["schema"] != profile["id"]
            or profile["id"] != "chrono-ci-check/v1"
            or scoped["policy"]["filemap"] != fm_path
            or scoped["policy"]["projects"] != pr_path):
        raise ValueError("unsupported version/profile pair")
    if json.loads(bytes(request["profile_bytes"])) != scoped:
        raise ValueError("profile bytes mismatch")
    for path, value in old.items():
        if json.loads(bytes(request["original_bytes"][path])) != value:
            raise ValueError("original bytes mismatch")
    values = copy.deepcopy(old)
    historical = {}
    for record in profile["legacy_records"]:
        collection = values[pr_path][record["collection"]]
        matches = [r for r in collection if r["id"] == record["id"]]
        if matches != [record["definition"]]:
            raise ValueError("historical definition mismatch")
        collection.remove(matches[0])
        for node in record["nodes"]:
            historical.setdefault(node, []).append(
                ["script:" + record["id"], matches[0]])
    policy = scoped["policy"]
    plans = {}
    for test, operations in policy["bindings"].items():
        if not operations or len(set(operations)) != len(operations):
            raise ValueError("invalid historical ordered binding")
        plans[test] = {"operations": operations,
                       "timeout_seconds": policy["operation_timeout_seconds"],
                       "output_limit_bytes": policy["operation_output_limit_bytes"]}
    values[fm_path]["schema_version"] = 2
    values[fm_path]["execution_plans"] = plans
    return {"values": values, "historical": historical,
            "mappings": profile["mappings"]}


if __name__ == "__main__":
    try:
        print(json.dumps(convert(json.load(sys.stdin)), sort_keys=True))
    except (ValueError, KeyError, TypeError) as error:
        print("E_MIGRATION:", error, file=sys.stderr)
        sys.exit(1)
