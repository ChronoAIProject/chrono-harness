"""Behavioral checks for the bootstrap byte owner; lifecycle exclusion is Rust-owned."""
import json
import os
import subprocess
from pathlib import Path
import tempfile
import unittest
import sys
from unittest import mock

import bootstrap
import bootstrap_owner as owner


class OwnerPublication(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="bootstrap-owner-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.selected = self.root / ".chrono-harness/bin/chrono-worktree"
        self.selected.parent.mkdir(parents=True)
        self.selected.write_bytes(b"original installed owner")
        self.staged = self.selected.parent / "owner-stages/one/chrono-worktree"
        self.staged.parent.mkdir(parents=True)
        self.staged.write_bytes(b"source-built correction")
        self.record = {"selected": ".chrono-harness/bin/chrono-worktree",
                       "previous": owner.file_binding(self.selected),
                       "staged": dict(owner.file_binding(self.staged),
                                      path=self.staged.relative_to(self.root).as_posix())}

    def test_publication_preserves_original_inode_and_reentry_is_idempotent(self):
        previous = self.record["previous"]
        result = owner.publish_bytes(self.root, self.record)
        self.assertEqual(self.selected.read_bytes(), b"source-built correction")
        retained = self.staged.parent / "previous-owner"
        self.assertEqual(owner.file_binding(retained), previous)
        self.assertEqual(owner.publish_bytes(self.root, self.record), result)

    def test_interrupted_after_atomic_replace_recovers_from_original_intent(self):
        replace = os.replace
        def interrupt(source, target):
            replace(source, target)
            if Path(target) == self.selected:
                raise OSError("original publication interruption")
        with mock.patch.object(owner.os, "replace", side_effect=interrupt):
            with self.assertRaisesRegex(OSError, "original publication interruption"):
                owner.publish_bytes(self.root, self.record)
        self.assertFalse((self.staged.parent / "publication-result.json").exists())
        result = owner.publish_bytes(self.root, self.record)
        self.assertEqual(result["installed"], owner.file_binding(self.selected))
        self.assertEqual((self.staged.parent / "previous-owner").read_bytes(),
                         b"original installed owner")

    def test_changed_selected_or_stage_and_foreign_retention_refuse_before_replacement(self):
        for changed in (self.selected, self.staged, self.staged.parent / "previous-owner"):
            with self.subTest(changed=changed):
                original = changed.read_bytes() if changed.exists() else None
                changed.write_bytes(b"foreign bytes")
                before = self.selected.read_bytes()
                with self.assertRaises(ValueError):
                    owner.publish_bytes(self.root, self.record)
                self.assertEqual(self.selected.read_bytes(), before)
                if original is None:
                    changed.unlink()
                else:
                    changed.write_bytes(original)

    def test_reentry_refuses_same_bytes_from_a_different_published_inode(self):
        owner.publish_bytes(self.root, self.record)
        foreign = self.selected.parent / "foreign-same-bytes"
        foreign.write_bytes(self.selected.read_bytes())
        os.replace(foreign, self.selected)
        with self.assertRaisesRegex(ValueError, "drifted after publication intent"):
            owner.publish_bytes(self.root, self.record)

    def test_symlink_and_marker_without_capabilities_refuse(self):
        self.staged.unlink()
        self.staged.symlink_to(self.selected)
        with self.assertRaises(ValueError):
            owner.publish_bytes(self.root, self.record)
        with mock.patch.object(bootstrap, "PASS_FDS", ()):
            with self.assertRaisesRegex(ValueError, "ownership descriptors"):
                owner.require_capabilities(self.root)

    def test_ordinary_shared_install_may_only_reuse_identical_owner_bytes(self):
        with mock.patch.dict(os.environ, {"CHRONO_WORKTREE_BOOTSTRAP": str(self.root)}):
            with self.assertRaisesRegex(ValueError, "exclusive owner deployment"):
                owner.check_shared_install(self.root, self.staged, self.selected)
            self.staged.write_bytes(self.selected.read_bytes())
            owner.check_shared_install(self.root, self.staged, self.selected)


class ProducerSource(unittest.TestCase):
    def published_fixture(self):
        fixture, selected = self.selected_original_fixture()
        host = fixture.host
        profile = dict(host.config, install=[], report_path=".chrono-harness/state/bootstrap-fixture-stage.json",
                       owner_stage={"source": "built-tool", "selected": selected.relative_to(fixture.root).as_posix()})
        config_path = ".chrono-harness/ci/stage.json"
        host.write_json(fixture.root / config_path, profile)
        host.declare("write", "built-tool", "staged owner")
        host.git("add", ".")
        host.git("commit", "-qm", "new owner candidate")
        with tempfile.TemporaryFile() as lease, mock.patch.object(bootstrap, "PASS_FDS", (lease.fileno(),)), mock.patch.dict(
                os.environ, dict(host.environment, CHRONO_WORKTREE_BOOTSTRAP=str(fixture.root))), mock.patch.object(
                sys, "argv", ["bootstrap.py", str(fixture.root), config_path]):
            bootstrap.main()
        original = fixture.root / profile["report_path"]
        stage = json.loads(original.read_bytes())
        publication = owner.publish_bytes(fixture.root, stage["owner_stage"])
        adoption = {"schema": "chrono-worktree-owner-adoption/v1", "source_commit": stage["source"]["commit"],
                    "stage_result": profile["report_path"], "stage_sha256": owner.shared.digest(original.read_bytes()),
                    "installed": publication["installed"], "original_owner": stage["owner_stage"]["previous"],
                    "original_protection": {"fixture": "unit-owned capability"}, "publication": publication,
                    "terminal_handoff": "not-claimed", "task_completion": "not-claimed"}
        receipt = ".chrono-harness/state/automatic-cleanup-v2/owner-adoption-fixture.json"
        owner.shared.write_new(fixture.root / receipt, owner.shared.encode(adoption))
        adoption["receipt"] = {"path": receipt, "sha256": owner.shared.digest((fixture.root / receipt).read_bytes())}
        owner.shared.write_new(fixture.root / owner.CURRENT, owner.shared.encode(adoption))
        return fixture, selected, adoption

    def test_changed_candidate_qualifies_published_owner_after_interrupted_core_install(self):
        fixture, selected, adoption = self.published_fixture()
        (fixture.root / "next-candidate").write_text("ordinary later candidate")
        fixture.host.git("add", ".")
        fixture.host.git("commit", "-qm", "next candidate")
        result = owner.qualified_selected_original(fixture.root, ".chrono-harness/state/bootstrap.json", selected)
        self.assertEqual(result["source"]["commit"], adoption["source_commit"])
        self.assertEqual(result["selected"], owner.file_binding(selected))

    def test_published_owner_rejects_missing_changed_and_unqualified_originals(self):
        fixture, selected, adoption = self.published_fixture()
        report = json.loads((fixture.root / adoption["stage_result"]).read_bytes())
        staged = fixture.root / report["owner_stage"]["staged"]["path"]
        paths = [fixture.root / adoption["stage_result"], fixture.root / adoption["receipt"]["path"],
                 fixture.root / report["operations"][0]["stdout"]["path"], staged,
                 staged.parent / "previous-owner", staged.parent / "publication-intent.json",
                 staged.parent / "publication-result.json"]
        for path in paths:
            raw = path.read_bytes()
            for missing in (False, True):
                with self.subTest(path=path, missing=missing):
                    # Keep the original inode; an ordinary lost/changed original
                    # must fail, even when selected executable bytes still match.
                    saved = path.with_name(path.name + ".fixture-original")
                    os.rename(path, saved)
                    if not missing:
                        path.write_bytes(b"changed original")
                    try:
                        with self.assertRaises((ValueError, KeyError, OSError, subprocess.CalledProcessError)):
                            owner.qualified_selected_original(fixture.root, ".chrono-harness/state/bootstrap.json", selected)
                    finally:
                        if path.exists():
                            path.unlink()
                        os.rename(saved, path)
                    self.assertEqual(path.read_bytes(), raw)
        # Publication with a failed producer is not qualified by self-consistent
        # new digests. Execution validation must still reject its original facts.
        path = fixture.root / adoption["stage_result"]
        report["status"] = "failed"
        path.write_bytes(owner.shared.encode(report))
        adoption["stage_sha256"] = owner.shared.digest(path.read_bytes())
        receipt = fixture.root / adoption["receipt"]["path"]
        receipt.write_bytes(owner.shared.encode({k:v for k,v in adoption.items() if k != "receipt"}))
        adoption["receipt"]["sha256"] = owner.shared.digest(receipt.read_bytes())
        (fixture.root / owner.CURRENT).write_bytes(owner.shared.encode(adoption))
        with self.assertRaisesRegex(ValueError, "successful original execution"):
            owner.qualified_selected_original(fixture.root, ".chrono-harness/state/bootstrap.json", selected)

    def selected_original_fixture(self):
        from bootstrap_shared_tests import SharedStartup
        fixture = SharedStartup()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        fixture.produce()
        selected = fixture.root / fixture.host.config["install"][0]["destination"]
        return fixture, selected

    def test_selected_installed_owner_qualifies_from_its_original_fixed_producer(self):
        fixture, selected = self.selected_original_fixture()
        original_source = bootstrap.observe_source(fixture.root)
        (fixture.root / "later-source").write_text("later candidate\n")
        fixture.host.git("add", "later-source")
        fixture.host.git("commit", "-qm", "later candidate")
        self.assertNotEqual(bootstrap.observe_source(fixture.root)["commit"], original_source["commit"])
        result = owner.qualified_selected_original(
            fixture.root, ".chrono-harness/state/bootstrap.json", selected)
        self.assertEqual(result["source"]["commit"], original_source["commit"])
        self.assertEqual(result["selected"], owner.file_binding(selected))

    def test_selected_original_rejects_changed_bytes_streams_and_report_before_construction(self):
        for change in ("selected", "stream", "source", "unproduced"):
            with self.subTest(change=change):
                fixture, selected = self.selected_original_fixture()
                path = fixture.root / ".chrono-harness/state/bootstrap.json"
                report = json.loads(path.read_bytes())
                if change == "selected":
                    selected.write_bytes(b"different installed owner")
                elif change == "stream":
                    stream = report["operations"][0]["stdout"]["path"]
                    (fixture.root / stream).write_bytes(b"different original stream")
                elif change == "source":
                    report["source"]["commit"] = "0" * 40
                    path.write_text(json.dumps(report))
                else:
                    selected = fixture.root / ".chrono-harness/bin/unproduced-owner"
                    selected.write_bytes(b"not in the original installation")
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    owner.qualified_selected_original(
                        fixture.root, ".chrono-harness/state/bootstrap.json", selected)

    def test_writable_homes_resolve_inside_this_host_without_pregrant_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            selection = {"host_environment": {"TMPDIR": ".chrono-harness/state/owner-tmp"}}
            env = bootstrap.provision_environment(root, selection)
            self.assertEqual(env["TMPDIR"], str(root / ".chrono-harness/state/owner-tmp"))
            self.assertFalse((root / ".chrono-harness").exists())
            for path in ("/tmp", "../outside", ".chrono-harness/bin/temporary"):
                with self.assertRaises(ValueError):
                    bootstrap.provision_environment(root, {"host_environment": {"TMPDIR": path}})

    def test_producer_validation_uses_immutable_source_and_retains_all_byte_checks(self):
        from bootstrap_shared_tests import SharedStartup
        fixture = SharedStartup()
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        fixture.produce()
        source = {key: bootstrap.observe_source(fixture.root)[key] for key in ("commit", "tree")}
        manifest = json.loads((fixture.transfer / "manifest.json").read_bytes())
        tool = manifest["profiles"]["core"]["files"][0]
        declaration = {"schema": "chrono-bootstrap-owner-supply/v1", "source": source,
                       "platform": owner.shared.native_platform(), "startup_config": ".chrono-harness/ci/bootstrap-shared.json",
                       "bundle": str(fixture.transfer), "binding": fixture.environment["CHRONO_STARTUP_BINDING"],
                       "issuer": {key: tool[key] for key in ("destination", "sha256", "size")}}
        # The live candidate diverges; the producer bytes stay producer bytes.
        (fixture.root / ".chrono-harness/ci/bootstrap.json").write_text('{"candidate": "changed"}')
        data, retained, originals = owner.qualified_supply(fixture.root, declaration)
        self.assertEqual(owner.shared.digest(data), tool["sha256"])
        self.assertEqual(retained["source"]["commit"], source["commit"])
        self.assertEqual(set(originals), {"core"})
        stream = manifest["profiles"]["core"]["report"]
        original = fixture.transfer / stream["path"]
        original.write_bytes(original.read_bytes() + b"corrupt")
        with self.assertRaisesRegex(ValueError, "startup original report differs"):
            owner.qualified_supply(fixture.root, declaration)

    def test_bad_or_relabelled_immutable_source_refuses_before_supply_preparation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "immutable producer"):
                owner.committed_reader(root, {"commit": "latest", "tree": "0" * 40})

