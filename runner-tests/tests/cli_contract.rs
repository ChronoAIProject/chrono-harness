use chrono_harness::dispatch;

#[test]
fn check_never_claims_success_even_with_help_or_snapshot_arguments() {
    let cases: &[&[&str]] = &[
        &["check"],
        &["check", "--help"],
        &[
            "check",
            "--config",
            ".chrono-harness/config.json",
            "--base",
            "0123456789abcdef0123456789abcdef01234567",
            "--candidate",
            "abcdef0123456789abcdef0123456789abcdef01",
            "--context",
            ".chrono-harness/state/context.json",
        ],
    ];
    for arguments in cases {
        let result = dispatch(arguments);
        assert_eq!(result.exit_code, 3, "{arguments:?}");
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("E_NOT_IMPLEMENTED"));
        assert!(result.stderr.contains("no validation was performed"));
    }
}

#[test]
fn unknown_and_malformed_commands_are_errors() {
    for arguments in [
        vec!["checks"],
        vec!["spec"],
        vec!["spec", "status", "--pretend-pass"],
        vec!["--version", "check"],
        vec!["help", "check"],
        vec![""],
    ] {
        let result = dispatch(&arguments);
        assert_eq!(result.exit_code, 2, "{arguments:?}");
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("E_USAGE"));
    }
}

#[test]
fn status_explicitly_reports_unimplemented_enforcement() {
    let result = dispatch(&["spec", "status"]);
    assert_eq!(result.exit_code, 0);
    assert!(result.stderr.is_empty());
    for fact in [
        "SPEC_STATUS=draft",
        "ENFORCEMENT=not-implemented",
        "HOST_REGISTRIES=proposed",
        "CONTRACT=SPEC.md",
    ] {
        assert!(result.stdout.lines().any(|line| line == fact));
    }
}

#[test]
fn help_and_version_are_informational_successes() {
    for arguments in [vec![], vec!["help"], vec!["--help"], vec!["-h"]] {
        let result = dispatch(&arguments);
        assert_eq!(result.exit_code, 0);
        assert!(result.stderr.is_empty());
        assert!(result.stdout.contains("NOT IMPLEMENTED"));
    }
    for arguments in [vec!["--version"], vec!["-V"]] {
        let result = dispatch(&arguments);
        assert_eq!(result.exit_code, 0);
        assert!(result.stderr.is_empty());
        assert!(result.stdout.starts_with("chrono-harness "));
        assert_eq!(result.stdout.lines().count(), 1);
    }
}
