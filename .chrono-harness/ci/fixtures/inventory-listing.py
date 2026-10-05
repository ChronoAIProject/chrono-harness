"""Provide the explicit Cargo-listing protocol cases used by inventory tests."""
import sys

mode, action = sys.argv[1:3]
if mode == "failed" and action == "right":
    print("original failure", file=sys.stderr)
    sys.exit(7)
names = {"all": ["a", "b"], "left": ["a"], "right": ["b"]}[action]
if "--ignored" in sys.argv:
    names = ["a"] if mode == "ignored" else []
elif mode == "empty" and action == "right":
    names = []
elif mode == "overlap" and action == "right":
    names = ["a", "b"]
elif mode == "omit" and action == "all":
    names = ["a", "b", "c"]
print("Running tests/cases.rs (target/cases-123abc)", file=sys.stderr)
for name in names:
    print(name + ": test")
print(str(len(names)) + " tests, 0 benchmarks")
