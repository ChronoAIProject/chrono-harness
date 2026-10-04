use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Step {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Exit(i32),
    SleepMillis(u64),
    Append {
        path: String,
        bytes: Vec<u8>,
    },
    Create {
        path: String,
        bytes: Vec<u8>,
    },
    Require {
        path: String,
        bytes: Vec<u8>,
    },
    Rendezvous {
        address: String,
        id: String,
    },
    RequireWithWitness {
        path: String,
        bytes: Vec<u8>,
        witness: String,
    },
}
