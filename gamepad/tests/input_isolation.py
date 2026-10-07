#!/usr/bin/env python3
"""Linux process-level proof of explicit fixture initialization, without OS input setup."""
import argparse
import pathlib
import re
import subprocess
import sys

FOCUSED_TEST = (
    "runtime::fixture::initialization_tests::"
    "fixture_initialization_and_reset_use_only_the_explicit_input_factory"
)


def require_focused_pass(output):
    """Reject libtest's successful zero-test result and wrong selected tests."""
    passed = re.search(r"^test " + re.escape(FOCUSED_TEST) + r" \.\.\. ok$", output, re.MULTILINE)
    if not passed or not re.search(
        r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", output, re.MULTILINE
    ):
        raise SystemExit("the intended initialization/reset test did not execute exactly once and pass")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("test_binary", type=pathlib.Path)
    parser.add_argument("trace", type=pathlib.Path)
    args = parser.parse_args()
    result = subprocess.run([
        "strace", "-f", "-e", "trace=socket", "-o", str(args.trace),
        str(args.test_binary.resolve()), "--exact", FOCUSED_TEST, "--nocapture",
    ], check=True, capture_output=True, text=True)
    print(result.stdout, end="")
    print(result.stderr, end="", file=sys.stderr)
    require_focused_pass(result.stdout)
    if "AF_NETLINK" in args.trace.read_text():
        raise SystemExit("fixture initialized the native udev gamepad backend")
    print("one focused initialization/reset test passed with zero native netlink socket attempts")


if __name__ == "__main__":
    main()
