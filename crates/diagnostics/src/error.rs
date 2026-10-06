use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;

pub type Context = BTreeMap<String, Value>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorNode {
    #[serde(rename = "type")]
    pub error_type: String,
    pub code: String,
    pub message: String,
    pub context: Context,
    pub source: Option<Box<ErrorNode>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceback: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<RelatedError>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedError {
    pub role: String,
    pub error: ErrorNode,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ErrorSchema {
    #[serde(rename = "chrono-error/v1")]
    V1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorRecord {
    pub schema: ErrorSchema,
    pub error: ErrorNode,
}

/// An error and the record produced by its concrete owner travel together.
/// This trait deliberately does not guess a concrete type from erased Display text.
pub trait RecordedError: StdError + Send + Sync + 'static {
    fn error_node(&self) -> ErrorNode;
}

#[derive(Debug)]
pub struct Failure {
    node: ErrorNode,
    source: Option<Box<dyn RecordedError>>,
    related: Vec<(String, Box<dyn RecordedError>)>,
}

impl Failure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            node: ErrorNode {
                error_type: std::any::type_name::<Self>().into(),
                code: code.into(),
                message: message.into(),
                context: Context::new(),
                source: None,
                native_code: None,
                traceback: None,
                related: Vec::new(),
            },
            source: None,
            related: Vec::new(),
        }
    }

    pub fn at(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.node.context.insert(name.into(), value.into());
        self
    }

    pub fn wrap(
        code: impl Into<String>,
        message: impl Into<String>,
        source: impl RecordedError,
    ) -> Self {
        let mut failure = Self::new(code, message);
        failure.source = Some(Box::new(source));
        failure
    }

    pub fn with_related(mut self, role: impl Into<String>, error: impl RecordedError) -> Self {
        self.related.push((role.into(), Box::new(error)));
        self
    }

    pub fn record(&self) -> ErrorRecord {
        ErrorRecord {
            schema: ErrorSchema::V1,
            error: self.error_node(),
        }
    }

    pub fn related(&self) -> impl Iterator<Item = (&str, &dyn RecordedError)> {
        self.related
            .iter()
            .map(|(role, error)| (role.as_str(), error.as_ref()))
    }
}

impl RecordedError for Failure {
    fn error_node(&self) -> ErrorNode {
        let mut node = self.node.clone();
        node.source = self
            .source
            .as_ref()
            .map(|source| Box::new(source.error_node()));
        node.related = self
            .related
            .iter()
            .map(|(role, error)| RelatedError {
                role: role.clone(),
                error: error.error_node(),
            })
            .collect();
        node
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.node.message)
    }
}

impl StdError for Failure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &dyn StdError)
    }
}

/// A received record remains a typed cause; importing it adds no claim of local execution.
#[derive(Debug)]
pub struct ReceivedError {
    node: ErrorNode,
    source: Option<Box<ReceivedError>>,
}

impl From<ErrorRecord> for ReceivedError {
    fn from(record: ErrorRecord) -> Self {
        Self::from_node(record.error)
    }
}

impl ReceivedError {
    fn from_node(mut node: ErrorNode) -> Self {
        let source = node
            .source
            .take()
            .map(|source| Box::new(Self::from_node(*source)));
        Self { node, source }
    }
}

impl fmt::Display for ReceivedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.node.message)
    }
}

impl StdError for ReceivedError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &dyn StdError)
    }
}

impl RecordedError for ReceivedError {
    fn error_node(&self) -> ErrorNode {
        let mut node = self.node.clone();
        node.source = self
            .source
            .as_ref()
            .map(|source| Box::new(source.error_node()));
        node
    }
}
