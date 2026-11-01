"""Behavioral checks for the bootstrap byte owner; lifecycle exclusion is Rust-owned."""
import json
import os
from pathlib import Path
import tempfile
import unittest
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

