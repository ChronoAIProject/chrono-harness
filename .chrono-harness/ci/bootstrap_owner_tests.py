"""Behavioral checks for the bootstrap byte owner; lifecycle exclusion is Rust-owned."""
import json
import os
import shutil
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
        self.root = Path(self.directory.name).resolve()
        self.adoption = {"coordinator_root": "git-main-worktree",
                         "program": ".chrono-harness/bin/chrono-worktree",
                         "git": "git", "config": ".chrono-harness/worktree.json"}
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

    def owner_result(self, directory, name, code=0):
        consumer = "bootstrap-owner:" + directory.name + ":" + name
        path = ".chrono-harness/state/worktrees/" + name + ".json"
        declaration = {"schema": "chrono-retention-consumer/v1", "consumer": consumer,
                       "purpose": "actual owner phase original consumption",
                       "originals": [{"id": "output-1", "path": path, "identity": "1:2:file"}]}
        report = {"status": "used" if code == 0 else "failed", "report_path": path,
                  "report_consumer": declaration,
                  "managed_process": {"stdout_bytes": [], "stderr_bytes": [],
                                      "failure": None, "exit_code": code}}
        return report

    def test_owner_phase_requests_native_consumer_and_retains_exact_handoff(self):
        directory = self.root / "attempt-current"
        directory.mkdir()
        for name in ("migration", "construction", "deployment"):
            with self.subTest(name=name):
                report = self.owner_result(directory, name)
                result = subprocess.CompletedProcess([], 0, stdout=json.dumps(report).encode(), stderr=b"")
                with mock.patch.object(owner.subprocess, "run", return_value=result) as native:
                    self.assertEqual(owner.run_owner(self.root, self.selected, ["bootstrap"], directory, name), report)
                argv = native.call_args.args[0]
                self.assertIn("--report-consumer", argv)
                self.assertEqual(argv[argv.index("--report-consumer") + 1], report["report_consumer"]["consumer"])
                self.assertEqual(json.loads((directory / (name + "-consumer.json")).read_bytes()),
                                 report["report_consumer"])

    def test_owner_phase_refuses_missing_or_misbound_consumer_even_on_exit_zero(self):
        for defect in ("missing", "consumer", "original"):
            with self.subTest(defect=defect):
                directory = self.root / ("attempt-" + defect)
                directory.mkdir()
                report = self.owner_result(directory, "construction")
                if defect == "missing":
                    report.pop("report_consumer")
                elif defect == "consumer":
                    report["report_consumer"]["consumer"] = "different caller"
                else:
                    report["report_consumer"]["originals"][0]["path"] = ".chrono-harness/state/worktrees/unrelated.json"
                raw = json.dumps(report).encode()
                with mock.patch.object(owner.subprocess, "run", return_value=subprocess.CompletedProcess(
                        [], 0, stdout=raw, stderr=b"")):
                    with self.assertRaisesRegex(ValueError, "report consumer"):
                        owner.run_owner(self.root, self.selected, ["bootstrap"], directory, "construction")
                self.assertEqual((directory / "construction.stdout").read_bytes(), raw)
                self.assertFalse((directory / "construction-consumer.json").exists())

    def test_failed_owner_phase_preserves_native_exit_and_consumer_for_diagnosis(self):
        directory = self.root / "attempt-failed"
        directory.mkdir()
        report = self.owner_result(directory, "construction", 7)
        with mock.patch.object(owner.subprocess, "run", return_value=subprocess.CompletedProcess(
                [], 2, stdout=json.dumps(report).encode(), stderr=b"")):
            with self.assertRaises(subprocess.CalledProcessError) as failed:
                owner.run_owner(self.root, self.selected, ["bootstrap"], directory, "construction")
        self.assertEqual(failed.exception.returncode, 7)
        self.assertEqual((directory / "construction.exit").read_bytes(), b"2\n")
        self.assertEqual(json.loads((directory / "construction-consumer.json").read_bytes()),
                         report["report_consumer"])

    def test_ordinary_shared_install_may_only_reuse_identical_owner_bytes(self):
        subprocess.run(["git", "init", "-q", str(self.root)], check=True, capture_output=True)
        with mock.patch.dict(os.environ, {"CHRONO_WORKTREE_BOOTSTRAP": str(self.root)}):
            with self.assertRaisesRegex(ValueError, "exclusive owner deployment"):
                owner.check_shared_install(self.root, self.staged, self.selected, self.adoption)
            self.staged.write_bytes(self.selected.read_bytes())
            owner.check_shared_install(self.root, self.staged, self.selected, self.adoption)

    def linked_checkout(self):
        def git(*args):
            subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Fixture",
                            "-c", "user.email=fixture@example.invalid", *args],
                           check=True, capture_output=True)
        git("init", "-q")
        (self.root / "source").write_bytes(b"fixed source")
        git("add", "source")
        git("commit", "-qm", "fixture source")
        linked = self.root / "linked"
        git("worktree", "add", "--detach", str(linked), "HEAD")
        return linked

    def test_linked_shared_install_publishes_local_tool_without_changing_coordinator(self):
        linked = self.linked_checkout()
        target = linked / ".chrono-harness/bin/chrono-worktree"
        target.parent.mkdir(parents=True)
        before = owner.file_binding(self.selected)
        with mock.patch.dict(os.environ, {"CHRONO_WORKTREE_BOOTSTRAP": str(linked)}):
            owner.check_shared_install(linked, self.staged, target, self.adoption)
            shutil.copy2(self.staged, target)
            self.assertEqual(target.read_bytes(), b"source-built correction")
            target.write_bytes(b"previous local tool")
            owner.check_shared_install(linked, self.staged, target, self.adoption)
            shutil.copy2(self.staged, target)
        self.assertEqual(owner.file_binding(self.selected), before)
        self.assertEqual(target.read_bytes(), b"source-built correction")

    def test_linked_shared_install_refuses_coordinator_inode_and_symlink_aliases(self):
        linked = self.linked_checkout()
        target = linked / ".chrono-harness/bin/chrono-worktree"
        target.parent.mkdir(parents=True)
        before = owner.file_binding(self.selected)
        with mock.patch.dict(os.environ, {"CHRONO_WORKTREE_BOOTSTRAP": str(linked)}):
            os.link(self.selected, target)
            with self.assertRaisesRegex(ValueError, "exclusive owner deployment"):
                owner.check_shared_install(linked, self.staged, target, self.adoption)
            self.staged.write_bytes(self.selected.read_bytes())
            owner.check_shared_install(linked, self.staged, target, self.adoption)
            self.staged.write_bytes(b"source-built correction")
            target.unlink()
            target.symlink_to(self.selected)
            with self.assertRaisesRegex(ValueError, "symlink"):
                owner.check_shared_install(linked, self.staged, target, self.adoption)
            target.unlink()
            target.parent.rmdir()
            target.parent.symlink_to(self.selected.parent)
            with self.assertRaisesRegex(ValueError, "symlink"):
                owner.check_shared_install(linked, self.staged, target, self.adoption)
        self.assertEqual(owner.file_binding(self.selected), before)


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

    def upgrade_fixture(self, published=False):
        if published:
            fixture, selected, _ = self.published_fixture()
        else:
            fixture, selected = self.selected_original_fixture()
        declaration = {"schema": "chrono-bootstrap-owner-upgrade/v1",
                       "selected_original": ".chrono-harness/state/bootstrap.json",
                       "stage_config": ".chrono-harness/ci/stage.json"}
        config = dict(fixture.host.config,
                      owner_upgrade=".chrono-harness/ci/owner-upgrade.json",
                      participation={"program": selected.relative_to(fixture.root).as_posix(),
                                     "config": ".chrono-harness/worktree.json"})
        config_path = Path(".chrono-harness/ci/upgrade-core.json")
        fixture.host.write_json(fixture.root / config["owner_upgrade"], declaration)
        fixture.host.write_json(fixture.root / config_path, config)
        fixture.host.git("add", ".")
        fixture.host.git("commit", "-qm", "current upgrade consumer")
        # Installed-owner qualification uses its actual retained producer, even
        # when the unrelated shared-startup transfer has already been reclaimed.
        shutil.rmtree(fixture.transfer)
        return fixture, selected, config_path, config

    def test_upgrade_constructs_with_actual_qualified_owner_without_old_supply(self):
        for published in (False, True):
            with self.subTest(published=published):
                fixture, selected, config_path, config = self.upgrade_fixture(published)
                original = selected.read_bytes()
                current = bootstrap.observe_source(fixture.root)
                staged = fixture.root / owner.STAGES / "current/chrono-worktree"
                staged.parent.mkdir(parents=True)
                staged.write_bytes(b"current built owner")
                calls = []
                def native(root, program, arguments, directory, name):
                    calls.append(name)
                    if name in ("migration", "construction"):
                        self.assertEqual(program.read_bytes(), original)
                        self.assertEqual(selected.read_bytes(), original)
                        qualification = json.loads((directory / "selected-qualification.json").read_bytes())
                        self.assertEqual(qualification["selected"], owner.file_binding(selected))
                        self.assertEqual(owner.shared.digest((directory / "selected-producer.json").read_bytes()),
                                         qualification["sha256"])
                    if name == "construction":
                        return {"managed_process": {"stdout_bytes": list(
                            b'chrono-owner-stage-result {"path":".chrono-harness/state/bootstrap-stage.json",'
                            b'"sha256":"stage-fixture"}\n')}}
                    if name == "deployment":
                        self.assertEqual(program, staged)
                        selected.write_bytes(staged.read_bytes())
                        (root / owner.CURRENT).write_text(json.dumps({
                            "source_commit": current["commit"], "installed": owner.file_binding(selected)}))
                    return {}
                actual_stage_report = owner.stage_report
                def stage_result(root, reference, expected_sha=None, producer_commit=None):
                    if reference == ".chrono-harness/state/bootstrap-stage.json":
                        self.assertEqual(expected_sha, "stage-fixture")
                        return {"owner_stage": {"staged": {
                            "path": staged.relative_to(fixture.root).as_posix()}}}, {}
                    return actual_stage_report(root, reference, expected_sha, producer_commit)
                with mock.patch.object(owner, "run_owner", side_effect=native), mock.patch.object(
                        owner, "stage_report", side_effect=stage_result):
                    owner.upgrade(fixture.root, config_path, config)
                self.assertEqual(calls, ["migration", "construction", "deployment"])
                self.assertEqual(selected.read_bytes(), b"current built owner")

    def test_upgrade_rejects_missing_or_changed_producer_before_native_effects(self):
        for change in ("missing-report", "stream", "selected"):
            with self.subTest(change=change):
                fixture, selected, config_path, config = self.upgrade_fixture()
                report_path = fixture.root / ".chrono-harness/state/bootstrap.json"
                report = json.loads(report_path.read_bytes())
                if change == "missing-report":
                    report_path.unlink()
                elif change == "stream":
                    (fixture.root / report["operations"][0]["stdout"]["path"]).write_bytes(b"changed stream")
                else:
                    selected.write_bytes(b"unproduced owner")
                before = owner.file_binding(selected)
                with mock.patch.object(owner, "run_owner") as native:
                    with self.assertRaises((ValueError, OSError)):
                        owner.upgrade(fixture.root, config_path, config)
                native.assert_not_called()
                self.assertEqual(owner.file_binding(selected), before)
                self.assertFalse((fixture.root / ".chrono-harness/state/bootstrap-owner-upgrade").exists())

    def test_upgrade_rechecks_qualified_owner_and_original_before_construction(self):
        qualify = owner.qualified_selected_original
        for change in ("selected", "report"):
            with self.subTest(change=change):
                fixture, selected, config_path, config = self.upgrade_fixture()
                def drift(*args):
                    result = qualify(*args)
                    path = selected if change == "selected" else fixture.root / result["path"]
                    path.write_bytes(path.read_bytes() + b"drift after qualification")
                    return result
                with mock.patch.object(owner, "qualified_selected_original", side_effect=drift), mock.patch.object(
                        owner, "run_owner") as native:
                    with self.assertRaisesRegex(ValueError, "qualified selected owner changed"):
                        owner.upgrade(fixture.root, config_path, config)
                native.assert_not_called()

    def test_upgrade_construction_failure_never_publishes(self):
        fixture, selected, config_path, config = self.upgrade_fixture()
        before = owner.file_binding(selected)
        calls = []
        def native(root, program, arguments, directory, name):
            calls.append(name)
            self.assertEqual(program.read_bytes(), selected.read_bytes())
            if name == "construction":
                raise subprocess.CalledProcessError(7, [str(program), *arguments])
            return {}
        with mock.patch.object(owner, "run_owner", side_effect=native), mock.patch.object(
                owner, "stage_report") as stage:
            with self.assertRaises(subprocess.CalledProcessError) as failed:
                owner.upgrade(fixture.root, config_path, config)
        self.assertEqual(failed.exception.returncode, 7)
        self.assertEqual(calls, ["migration", "construction"])
        stage.assert_not_called()
        self.assertFalse((fixture.root / owner.CURRENT).exists())
        self.assertEqual(owner.file_binding(selected), before)

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
            selection = {"host_environment": {"TMPDIR": ".chrono-harness/cache/owner-tmp"}}
            env = bootstrap.provision_environment(root, selection)
            self.assertEqual(env["TMPDIR"], str(root / ".chrono-harness/cache/owner-tmp"))
            self.assertFalse((root / ".chrono-harness").exists())
            for path in ("/tmp", "../outside", ".chrono-harness/bin/temporary", ".chrono-harness/state/owner-tmp"):
                with self.assertRaises(ValueError):
                    bootstrap.provision_environment(root, {"host_environment": {"TMPDIR": path}})

    def test_bad_immutable_producer_reference_refuses_before_reading(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "immutable producer"):
                owner.committed_reader(root, {"commit": "latest", "tree": "0" * 40})

