#!/usr/bin/env python3
"""Local tests for the registered Verus archive consumer."""
import hashlib
import json
from pathlib import Path
import platform
import tempfile
import unittest
import zipfile

import verus_bootstrap


class VerusBootstrapTests(unittest.TestCase):
    def setUp(self):
        self.host = Path(tempfile.mkdtemp(prefix="verus-bootstrap-test-"))
        (self.host / ".chrono-harness/ci").mkdir(parents=True)
        self.cache = self.host / ".chrono-harness/cache/bootstrap-verus"
        self.install = self.host / ".chrono-harness/bin/verus"
        archive = self.cache / "verus-test.zip"
        self.cache.mkdir(parents=True)
        with zipfile.ZipFile(archive, "w") as output:
            output.writestr("verus-test/verus", b"verus executable")
            output.writestr("verus-test/z3", b"z3")
        declaration = {
            "schema": "chrono-verus/v1",
            "version": "test",
            "commit": "test",
            "toolchain": "test",
            "macro_version": "test",
            "cache_directory": ".chrono-harness/cache/bootstrap-verus",
            "install_directory": ".chrono-harness/bin/verus",
            "platforms": {
                platform.system() + "/" + platform.machine(): {
                    "url": "https://github.com/verus-lang/verus/releases/download/test/test.zip",
                    "archive": "verus-test.zip",
                    "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                }
            },
        }
        (self.host / ".chrono-harness/ci/bootstrap-verus.json").write_text(
            json.dumps({"verus_policy": ".chrono-harness/verus.json"}))
        (self.host / ".chrono-harness/verus.json").write_text(json.dumps(declaration))

    def test_verify_and_extract_are_digest_bound(self):
        verus_bootstrap.verify(self.host, Path(".chrono-harness/ci/bootstrap-verus.json"))
        verus_bootstrap.extract(self.host, Path(".chrono-harness/ci/bootstrap-verus.json"))
        self.assertEqual((self.install / "verus").read_bytes(), b"verus executable")
        self.assertEqual((self.install / "z3").read_bytes(), b"z3")

    def test_extract_rejects_changed_archive(self):
        archive = self.cache / "verus-test.zip"
        archive.write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "wrong digest"):
            verus_bootstrap.extract(self.host, Path(".chrono-harness/ci/bootstrap-verus.json"))


if __name__ == "__main__":
    unittest.main()
