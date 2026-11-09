//! 临时发布宿主只接收样本显式提供的发布输入。
use std::{ffi::OsStr, process::Command};

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    for name in [
        "CHRONO_RELEASE_LOCAL",
        "CHRONO_RELEASE_RUN",
        "CHRONO_RELEASE_ATTEMPT",
        "CHRONO_RELEASE_JOB",
        "CHRONO_RELEASE_DEPENDENCIES",
        "CHRONO_RELEASE_CHANNEL",
        "CHRONO_RELEASE_SOURCE",
        "CHRONO_RELEASE_CI_RUN_ID",
        "CHRONO_RELEASE_CI_ATTEMPT",
        "CHRONO_RELEASE_VERSION",
    ] {
        command.env_remove(name);
    }
    command
}
