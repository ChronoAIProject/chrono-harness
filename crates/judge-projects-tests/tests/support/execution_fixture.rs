use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Product {
    DoubleMultiply,
    DoubleAdd,
    Answer,
}

impl Product {
    pub fn evaluate(&self, input: i64) -> i64 {
        match self {
            Self::DoubleMultiply => input * 2,
            Self::DoubleAdd => input + input,
            Self::Answer => 42,
        }
    }
}

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
    Write {
        path: String,
        bytes: Vec<u8>,
    },
    Require {
        path: String,
        bytes: Vec<u8>,
    },
    RequireSuffix {
        path: String,
        bytes: Vec<u8>,
    },
    RequireCount {
        path: String,
        byte: u8,
        count: usize,
    },
    CheckProduct {
        path: String,
        input: i64,
        expected: i64,
    },
    ObserveProcess,
    ObserveEnvironment {
        names: Vec<String>,
    },
    CheckNativeInputs {
        path: String,
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
