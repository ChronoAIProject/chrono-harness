"""Behavioral checks for bootstrap qualification and owner phase consumers."""
import json
import hashlib
import importlib.util
import os
import shutil
import subprocess
from pathlib import Path
import tempfile
import unittest
import sys
import zlib
from unittest import mock
from types import SimpleNamespace

import bootstrap
import bootstrap_owner as owner

_fixture_spec = importlib.util.spec_from_file_location(
    "bootstrap_owner_fixture", Path(__file__).with_name("bootstrap-tests.py"))
_fixture_module = importlib.util.module_from_spec(_fixture_spec)
_fixture_spec.loader.exec_module(_fixture_module)


def retained_process(stdout=b"", stderr=b"", **metadata):
    process = dict(metadata, encoding="chrono-retained-process/v2")
    for stream, raw in (("stdout", stdout), ("stderr", stderr)):
        process[stream + "_zlib_hex"] = zlib.compress(raw, 1).hex()
        process[stream + "_length"] = len(raw)
        process[stream + "_sha256"] = hashlib.sha256(raw).hexdigest()
    return process


class ProducerSource(unittest.TestCase):
    def test_construction_requires_original_capabilities(self):
        with tempfile.TemporaryDirectory(prefix="bootstrap-owner-") as directory:
            with mock.patch.object(bootstrap, "PASS_FDS", ()):
                with self.assertRaisesRegex(ValueError, "ownership descriptors"):
                    owner.require_capabilities(Path(directory).resolve())

    def published_fixture(self, reuse=False):
        fixture, selected = self.selected_original_fixture()
        host = fixture.host
        profile = dict(host.config, install=[], report_path=".chrono-harness/state/bootstrap-fixture-stage.json",
                       owner_stage={"source": "built-tool", "selected": selected.relative_to(fixture.root).as_posix()})
        config_path = ".chrono-harness/ci/stage.json"
        host.write_json(fixture.root / config_path, profile)
        if reuse:
            core = dict(host.config, owner_upgrade=".chrono-harness/ci/owner-upgrade.json",
                        reuse_owner_operations=profile["operations"],
                        participation={"coordinator_root": "git-main-worktree", "git": "git",
                                       "program": selected.relative_to(fixture.root).as_posix(),
                                       "config": ".chrono-harness/worktree.json"})
            host.write_json(fixture.root / ".chrono-harness/ci/reuse-core.json", core)
            host.write_json(fixture.root / core["owner_upgrade"], {
                "schema": "chrono-bootstrap-owner-upgrade/v1", "stage_config": config_path,
                "selected_original": ".chrono-harness/state/bootstrap.json"})
        host.declare("write", "built-tool", "staged owner")
        host.git("add", ".")
        host.git("commit", "-qm", "new owner candidate")
        with tempfile.TemporaryFile() as lease, mock.patch.object(bootstrap, "PASS_FDS", (lease.fileno(),)), mock.patch.dict(
                os.environ, dict(host.environment, CHRONO_WORKTREE_BOOTSTRAP=str(fixture.root))), mock.patch.object(
                sys, "argv", ["bootstrap.py", str(fixture.root), config_path]):
            bootstrap.main()
        original = fixture.root / profile["report_path"]
        stage = json.loads(original.read_bytes())
        # Synthetic external publication inputs test this Python reader only.
        # Real byte production, inode retention and recovery are Rust CLI tests.
        staged = fixture.root / stage["owner_stage"]["staged"]["path"]
        selected.rename(staged.parent / "previous-owner")
        shutil.copyfile(staged, selected)
        layout = json.loads((Path(__file__).parent / "fixtures/owner-publication.json").read_bytes())
        publication = dict(layout["result"], stage=stage["owner_stage"], installed=owner.file_binding(selected))
        intent = dict(layout["intent"], stage=stage["owner_stage"], replacement=publication["installed"])
        owner.shared.write_new(staged.parent / "publication-intent.json", owner.shared.encode(intent))
        owner.shared.write_new(staged.parent / "publication-result.json", owner.shared.encode(publication))
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

    def test_core_consumes_the_actual_current_stage_without_rebuilding_or_replacing_it(self):
        fixture, selected, adoption = self.published_fixture(reuse=True)
        original = owner.file_binding(selected)
        stage = json.loads((fixture.root / adoption["stage_result"]).read_bytes())
        # A separately rebuilt scratch output is neither the deployed bytes nor
        # an input to this already successful construction's handoff.
        (fixture.root / "built-tool").write_bytes(b"different scratch rebuild")
        with tempfile.TemporaryFile() as lease, mock.patch.object(bootstrap, "PASS_FDS", (lease.fileno(),)), mock.patch.dict(
                os.environ, dict(fixture.host.environment, CHRONO_WORKTREE_BOOTSTRAP=str(fixture.root))), mock.patch.object(
                sys, "argv", ["bootstrap.py", str(fixture.root), ".chrono-harness/ci/reuse-core.json"]), mock.patch.object(
                bootstrap, "run_operation", side_effect=AssertionError("duplicate owner construction")):
            bootstrap.main()
        result = json.loads((fixture.root / ".chrono-harness/state/bootstrap.json").read_bytes())
        self.assertEqual(result["status"], "passed")
        self.assertEqual(owner.file_binding(selected), original)
        for row, source in zip(result["operations"], stage["operations"]):
            self.assertEqual({k: v for k, v in row.items() if k != "reused_from"}, source)
            self.assertEqual(row["reused_from"], {"path": adoption["stage_result"],
                                                  "sha256": adoption["stage_sha256"]})
        config = json.loads((fixture.root / ".chrono-harness/ci/reuse-core.json").read_bytes())
        owner.shared.original_report(fixture.root, {"config": ".chrono-harness/ci/reuse-core.json"},
                                     config, bootstrap.observe_source(fixture.root))

    def test_construction_reuse_binds_only_the_reused_operations_tool_selection(self):
        fixture, selected, adoption = self.published_fixture(reuse=True)
        config = json.loads((fixture.root / ".chrono-harness/ci/reuse-core.json").read_bytes())
        report = json.loads((fixture.root / adoption["stage_result"]).read_bytes())
        observed = bootstrap.observe_source(fixture.root)
        # Core's later Verus bootstrap needs Python; owner construction does not.
        config["tools"]["python3"] = sys.executable
        reused, source = owner.reuse_construction(fixture.root, config, report, observed)
        self.assertEqual(set(reused), set(config["reuse_owner_operations"]))
        self.assertEqual(source, fixture.root / report["owner_stage"]["staged"]["path"])
        tool = report["operations"][0]["tool"]
        config["tools"][tool] = "different-reused-tool"
        with self.assertRaisesRegex(ValueError, "owner construction selection differs: tool " + tool):
            owner.reuse_construction(fixture.root, config, report, observed)

    def test_construction_reuse_refuses_different_selection_observation_and_missing_operations(self):
        fixture, selected, adoption = self.published_fixture(reuse=True)
        config = json.loads((fixture.root / ".chrono-harness/ci/reuse-core.json").read_bytes())
        report = json.loads((fixture.root / adoption["stage_result"]).read_bytes())
        observed = bootstrap.observe_source(fixture.root)
        for defect in ("selection", "observation", "operation", "candidate"):
            with self.subTest(defect=defect):
                selected_config = json.loads(json.dumps(config))
                result = json.loads(json.dumps(report))
                source = dict(observed)
                if defect == "selection":
                    selected_config["environment"] = {"CARGO_NET_OFFLINE": "true"}
                elif defect == "observation":
                    result["environment"]["CARGO_INCREMENTAL"] = "different"
                elif defect == "operation":
                    selected_config["operations"].append("unproduced")
                    selected_config["reuse_owner_operations"] = ["unproduced"]
                else:
                    source["commit"] = "0" * 40
                with self.assertRaises(ValueError):
                    owner.reuse_construction(fixture.root, selected_config, result, source)

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
        host = _fixture_module.Bootstrap()
        host.setUp()
        self.addCleanup(host.doCleanups)
        (host.root / ".gitignore").write_text(
            "/built-*\n/.chrono-harness/bin/\n/.chrono-harness/state/\n/mock-tools/\n")
        host.git("init", "-q")
        host.git("config", "user.email", "fixture@example.invalid")
        host.git("config", "user.name", "Fixture")
        host.git("add", ".")
        host.git("commit", "-qm", "source producer")
        host.succeeded(host.invoke())
        fixture = SimpleNamespace(root=host.root, host=host)
        selected = fixture.root / host.config["install"][0]["destination"]
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
                        return {"managed_process": retained_process(
                            b'chrono-owner-stage-result {"path":".chrono-harness/state/bootstrap-stage.json",'
                            b'"sha256":"stage-fixture"}\n')}
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

