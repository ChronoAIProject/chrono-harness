#!/bin/sh
exec .chrono-harness/bin/chrono-ci verify --host-root . --config .chrono-harness/ci/units.json
