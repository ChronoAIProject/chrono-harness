#!/usr/bin/env python3
"""Transport real bootstrap fixture outputs, then exercise consumer rejection."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("bootstrap_fixture", SOURCE / "bootstrap-tests.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
CONFIG = ".chrono-harness/ci/bootstrap-shared.json"


class SharedStartup(unittest.TestCase):
    def setUp(self):
        self.host = fixture.Bootstrap()
        self.host.setUp()
        self.addCleanup(self.host.doCleanups)
        self.root = self.host.root
        shutil.copyfile(SOURCE / "bootstrap_shared.py", self.host.ci / "bootstrap_shared.py")
        self.host.config["install"].append({"source": "built-second", "destination": ".chrono-harness/bin/second"})
        self.host.write_json(self.host.ci / "bootstrap.json", self.host.config)
        for name in ["built-tool", "built-second"]:
            path = self.root / name
            path.write_bytes(b"second")
            path.chmod(0o755)
        self.shared = {"schema": "chrono-host-startup/v1", "directory": ".chrono-harness/startup/",
                       "profiles": {"core": {"config": ".chrono-harness/ci/bootstrap.json", "build": True}},
                       "consumers": {"unit_fixture": ["core"]}}
        self.host.write_json(self.root / CONFIG, self.shared)
        (self.root / ".gitignore").write_text("/built-*\n/.chrono-harness/bin/\n/.chrono-harness/state/\n/.chrono-harness/startup/\n/mock-tools/\n")
        (self.root / "product-source").write_text("original")
        self.host.git("init", "-q")
        self.host.git("config", "user.email", "fixture@example.invalid")
        self.host.git("config", "user.name", "Fixture")
        self.host.git("add", ".")
        self.host.git("commit", "-qm", "source")
        self.environment = self.host.environment
        self.output = self.host.tools / "github-output"
        self.output.touch()
        self.environment["GITHUB_OUTPUT"] = str(self.output)
        self.transfer = self.root / self.shared["directory"]

    def invoke(self, command, *args, root=None):
        root = root or self.root
        return subprocess.run([sys.executable, "-B", str(root / ".chrono-harness/ci/bootstrap_shared.py"),
                               str(root), CONFIG, command, *args], env=self.environment,
                              pass_fds=fixture.PASS_FDS, capture_output=True)

    def produce(self):
        self.host.succeeded(self.invoke("produce"))
        self.environment["CHRONO_STARTUP_BINDING"] = self.output.read_text().strip().split("=", 1)[1]
        self.assertEqual(self.environment["CHRONO_STARTUP_BINDING"],
                         hashlib.sha256((self.transfer / "manifest.json").read_bytes()).hexdigest())

    def clone_consumer(self):
        directory = tempfile.TemporaryDirectory(prefix="startup consumer ")
        self.addCleanup(directory.cleanup)
        root = Path(directory.name).resolve() / "host"
        subprocess.run(["git", "clone", "-q", str(self.root), str(root)], check=True,
                       capture_output=True, pass_fds=fixture.PASS_FDS)
        shutil.copytree(self.transfer, root / self.shared["directory"])
        return root

    def test_import_validates_originals_without_building_and_verify_detects_installed_drift(self):
        self.produce()
        original = self.host.state_path.read_bytes()
        root = self.clone_consumer()
        for path in (root / self.shared["directory"] / "tools").iterdir():
            path.chmod(0o644)  # Native artifact transfer does not preserve execute bits.
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        for name, expected in [("tool", b"binary"), ("second", b"second")]:
            path = root / ".chrono-harness/bin" / name
            self.assertEqual(path.read_bytes(), expected)
            self.assertEqual(path.stat().st_mode & 0o777, 0o755)
        self.assertFalse((root / "built-tool").exists(), "consumer ran a producer operation")
        result = json.loads((root / ".chrono-harness/state/bootstrap.json").read_bytes())
        self.assertEqual(result["provenance"]["kind"], "shared-startup")
        self.assertEqual(result["provenance"]["report_sha256"], hashlib.sha256(original).hexdigest())
        self.host.succeeded(self.invoke("verify", "core", root=root))
        (root / ".chrono-harness/bin/tool").write_bytes(b"changed")
        failed = self.invoke("verify", "core", root=root)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn(b"installed startup tool differs", failed.stderr)

    def test_bad_binding_source_platform_profile_or_late_asset_never_partially_installs(self):
        self.produce()
        for defect in ["binding", "missing-binding", "dirty", "source", "platform", "profile", "late-asset", "symlink"]:
            with self.subTest(defect=defect):
                root = self.clone_consumer()
                transfer = root / self.shared["directory"]
                manifest_path = transfer / "manifest.json"
                manifest = json.loads(manifest_path.read_bytes())
                binding = self.environment["CHRONO_STARTUP_BINDING"]
                if defect == "binding": self.environment["CHRONO_STARTUP_BINDING"] = "0" * 64
                elif defect == "missing-binding": self.environment.pop("CHRONO_STARTUP_BINDING")
                elif defect == "dirty": (root / "product-source").write_text("different")
                elif defect == "source":
                    subprocess.run(["git", "-C", str(root), "-c", "user.name=Fixture", "-c", "user.email=f@example.invalid", "commit", "--allow-empty", "-qm", "other"], check=True, pass_fds=fixture.PASS_FDS)
                elif defect == "profile": (root / ".chrono-harness/ci/bootstrap.json").write_text("{}")
                elif defect == "platform":
                    manifest["platform"] = {"system": "different", "machine": "different"}
                    manifest_path.write_text(json.dumps(manifest))
                    self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
                elif defect == "late-asset": (transfer / "tools/1").write_bytes(b"damaged")
                elif defect == "symlink":
                    (transfer / "tools/1").unlink()
                    (transfer / "tools/1").symlink_to(root / "product-source")
                target = root / ".chrono-harness/bin/tool"
                target.parent.mkdir()
                target.write_bytes(b"keep")
                result = self.invoke("install", "unit_fixture", root=root)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn(b"E_STARTUP:", result.stderr)
                self.assertEqual(target.read_bytes(), b"keep")
                self.assertFalse((root / ".chrono-harness/state/startup-install.json").exists())
                self.environment["CHRONO_STARTUP_BINDING"] = binding

    def test_publication_failure_and_unknown_consumer_are_not_success(self):
        self.environment["GITHUB_OUTPUT"] = str(self.host.tools)
        result = self.invoke("produce")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"E_STARTUP:", result.stderr)
        self.assertEqual(self.output.read_bytes(), b"")
        self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256((self.transfer / "manifest.json").read_bytes()).hexdigest()
        root = self.clone_consumer()
        result = self.invoke("install", "unknown", root=root)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((root / ".chrono-harness/bin").exists())
        state = root / ".chrono-harness/state"
        state.mkdir()
        (state / "startup-install.json").mkdir()
        self.assertNotEqual(self.invoke("install", "unit_fixture", root=root).returncode, 0)

    def test_dirty_producer_and_existing_transfer_are_rejected(self):
        (self.root / "product-source").write_text("dirty")
        failed = self.invoke("produce")
        self.assertNotEqual(failed.returncode, 0)
        self.assertFalse(self.transfer.exists())
        self.host.git("restore", "product-source")
        self.produce()
        original = (self.transfer / "manifest.json").read_bytes()
        self.assertNotEqual(self.invoke("produce").returncode, 0)
        self.assertEqual((self.transfer / "manifest.json").read_bytes(), original)


if __name__ == "__main__":
    unittest.main()


class DistributionStartup(unittest.TestCase):
    """Use the existing Rust installer, with distinct release and host fixture commits."""
    invoke = SharedStartup.invoke
    produce = SharedStartup.produce
    clone_consumer = SharedStartup.clone_consumer

    def setUp(self):
        SharedStartup.setUp(self)
        self.release_commit = self.host.git("rev-parse", "HEAD")
        self.release_tree = self.host.git("rev-parse", "HEAD^{tree}")
        import bootstrap
        system = {"Darwin": "macos", "Linux": "linux"}[bootstrap.platform.system()]
        arch = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64"}[bootstrap.platform.machine()]
        self.platform = system + "-" + arch
        package = self.host.tools / "release"
        package.mkdir()
        installer = SOURCE.parents[1] / "crates/distribution/target/debug/chrono-distribution"
        self.pinned = (self.host.tools / "cargo").read_bytes()
        members = {}
        for name, data in [("chrono-distribution", installer.read_bytes()), ("tool", self.pinned)]:
            filename = name + "-" + self.platform
            (package / filename).write_bytes(data)
            members[name] = {"file": filename, "size": len(data),
                             "sha256": hashlib.sha256(data).hexdigest()}
        self.release = {"schema": "chrono-release/v1", "version": "fixture-release",
                        "source_commit": self.release_commit, "source_tree": self.release_tree,
                        "platforms": {self.platform: members}}
        manifest = self.root / ".chrono-harness/distribution-release.json"
        self.host.write_json(manifest, self.release)
        shutil.copyfile(manifest, package / "release.json")
        subprocess.run([str(installer), "adopt", "--host-root", str(self.root),
                        "--manifest", str(manifest), "--source-dir", str(package),
                        "--tool", "tool"], check=True, capture_output=True, pass_fds=fixture.PASS_FDS)
        for item in self.host.config["install"]:
            item["destination"] = item["destination"].replace("/bin/", "/bin/source/")
        self.host.config["distribution"] = {"manifest": ".chrono-harness/distribution-release.json"}
        self.host.write_json(self.host.ci / "bootstrap.json", self.host.config)
        self.host.git("add", ".")
        self.host.git("commit", "-qm", "host adoption after release")

    def test_installer_receipt_and_source_operations_keep_distinct_identities(self):
        self.produce()
        original = self.host.state_path.read_bytes()
        receipt = (self.root / ".chrono-harness/state/distribution.json").read_bytes()
        release = json.loads(receipt)
        source_report = json.loads(original)
        self.assertEqual(release["source_commit"], self.release_commit)
        self.assertEqual(release["source_tree"], self.release_tree)
        self.assertEqual(source_report["source"]["commit"], self.host.git("rev-parse", "HEAD"))
        self.assertNotEqual(release["source_commit"], source_report["source"]["commit"])
        self.assertEqual(source_report["installed"][0]["path"], ".chrono-harness/bin/source/tool")
        self.assertEqual((self.root / ".chrono-harness/bin/source/tool").read_bytes(), b"binary")
        self.assertEqual((self.root / ".chrono-harness/bin/tool").read_bytes(), self.pinned)
        root = self.clone_consumer()
        self.host.succeeded(self.invoke("install", "unit_fixture", root=root))
        self.assertEqual((root / ".chrono-harness/state/distribution.json").read_bytes(), receipt)
        self.assertEqual((root / ".chrono-harness/state/startup-originals/core-distribution.json").read_bytes(), receipt)
        self.assertEqual((root / ".chrono-harness/state/startup-originals/core.json").read_bytes(), original)
        self.assertFalse((root / "built-tool").exists())
        self.assertFalse((root / "mock-tools").exists(), "consumer reran the installer transport")
        self.host.succeeded(self.invoke("verify", "core", root=root))
        (root / ".chrono-harness/state/distribution.json").write_bytes(receipt + b" ")
        failed = self.invoke("verify", "core", root=root)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn(b"installed startup distribution report differs", failed.stderr)

    def test_source_output_cannot_replace_a_pinned_enforcement_destination(self):
        self.host.config["install"][0]["destination"] = ".chrono-harness/bin/tool"
        self.host.write_json(self.host.ci / "bootstrap.json", self.host.config)
        (self.root / "built-tool").unlink()
        result = self.host.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"source install overlaps immutable enforcement executable", result.stderr)
        self.assertFalse((self.root / "built-tool").exists())
        self.assertFalse(self.host.state_path.exists())

    def test_release_source_receipt_and_payload_drift_fail_before_install(self):
        self.produce()
        for defect in ["release-source", "receipt-missing", "payload", "manifest", "installer-binding"]:
            with self.subTest(defect=defect):
                root = self.clone_consumer()
                transfer = root / self.shared["directory"]
                manifest_path = transfer / "manifest.json"
                manifest = json.loads(manifest_path.read_bytes())
                entry = manifest["profiles"]["core"]
                if defect == "release-source":
                    receipt_path = transfer / entry["distribution_report"]["path"]
                    receipt = json.loads(receipt_path.read_bytes())
                    receipt["source_commit"] = manifest["source"]["commit"]
                    receipt_path.write_text(json.dumps(receipt))
                    entry["distribution_report"]["sha256"] = hashlib.sha256(receipt_path.read_bytes()).hexdigest()
                    report_path = transfer / entry["report"]["path"]
                    report = json.loads(report_path.read_bytes())
                    report["distribution"]["receipt_sha256"] = entry["distribution_report"]["sha256"]
                    report_path.write_text(json.dumps(report))
                    entry["report"]["sha256"] = hashlib.sha256(report_path.read_bytes()).hexdigest()
                    manifest_path.write_text(json.dumps(manifest))
                elif defect == "receipt-missing":
                    (transfer / entry["distribution_report"]["path"]).unlink()
                elif defect == "payload":
                    tool = entry["files"][-1]
                    (transfer / tool["path"]).write_bytes(b"unrelated source build")
                    tool["size"] = len(b"unrelated source build")
                    tool["sha256"] = hashlib.sha256(b"unrelated source build").hexdigest()
                    manifest_path.write_text(json.dumps(manifest))
                elif defect == "manifest":
                    (root / ".chrono-harness/distribution-release.json").write_text("{}")
                else:
                    lock = json.loads((root / ".chrono-harness/distribution.json").read_bytes())
                    lock["installers"][self.platform]["sha256"] = "0" * 64
                    self.host.write_json(root / ".chrono-harness/distribution.json", lock)
                self.environment["CHRONO_STARTUP_BINDING"] = hashlib.sha256(manifest_path.read_bytes()).hexdigest()
                result = self.invoke("install", "unit_fixture", root=root)
                self.assertNotEqual(result.returncode, 0, defect)
                self.assertFalse((root / ".chrono-harness/bin").exists(), "partial installation: " + defect)
