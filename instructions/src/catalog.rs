use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) const DEFAULT_CATALOG: &str = include_str!("../../assets/instructions/catalog.json");
pub(super) const DEFAULT_MANIFEST: &str =
    include_str!("../../assets/instructions/default-manifest.json");
pub(super) const RENDER: &str = "atomic-rules/relative-alias/v3";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub schema_version: u32,
    pub producer: String,
    pub render: String,
    pub catalog: String,
    pub host_context: String,
    pub outputs: Vec<Output>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Output {
    pub id: String,
    pub path: String,
    pub format: Format,
    pub locale: String,
    pub roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<Skill>,
}

#[derive(Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Format {
    RootGuide,
    Markdown,
    Skill,
}
impl Format {
    fn label(self) -> &'static str {
        match self {
            Self::RootGuide => "root-guide",
            Self::Markdown => "markdown",
            Self::Skill => "skill",
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Skill {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Catalog {
    pub schema_version: u32,
    pub locales: Vec<Locale>,
    pub atoms: Vec<Atom>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Locale {
    pub id: String,
    pub root_frame: String,
    pub projection_notice: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Atom {
    pub id: String,
    pub requires: Vec<String>,
    pub variants: Vec<Variant>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Variant {
    pub locale: String,
    pub source: Content,
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum Content {
    Inline { text: String },
    File { path: String },
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}
pub(super) fn bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut result = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    result.push(b'\n');
    Ok(result)
}
fn identity(s: &str) -> Result<(), String> {
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(format!(
            "invalid identity {s:?}; use ASCII letters, digits, '.', '_' or '-'"
        ));
    }
    Ok(())
}
fn single_line(s: &str) -> bool {
    !s.trim().is_empty()
        && !s
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}
pub(super) fn content_valid(s: &str) -> Result<(), String> {
    if s.contains("<!-- chrono-instructions") {
        return Err("content contains reserved marker prefix <!-- chrono-instructions".into());
    }
    Ok(())
}
impl Skill {
    fn validate(&self) -> Result<(), String> {
        let name = &self.name;
        if name.is_empty()
            || name.len() > 63
            || name.starts_with('-')
            || name.ends_with('-')
            || name.contains("--")
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(format!("invalid skill name {name:?}"));
        }
        if !single_line(&self.description)
            || self.description.chars().count() > 1024
            || self.description.contains(['<', '>'])
        {
            return Err("skill description must be nonempty, single-line, <=1024 characters, without controls or angle brackets".into());
        }
        Ok(())
    }
    fn header(&self) -> String {
        // JSON quoted strings are YAML double-quoted scalars. Controls are rejected.
        format!(
            "---\nname: {}\ndescription: {}\n---\n",
            serde_json::to_string(&self.name).unwrap(),
            serde_json::to_string(&self.description).unwrap()
        )
    }
}
impl Manifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2
            || self.producer != "chrono-instructions"
            || self.render != RENDER
        {
            return Err("unsupported registration: expected schema 2 / chrono-instructions / atomic-rules/relative-alias/v3".into());
        }
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut roots = 0;
        for output in &self.outputs {
            identity(&output.id)?;
            if !ids.insert(&output.id) {
                return Err(format!("duplicate output ID {}", output.id));
            }
            if !paths.insert(&output.path) {
                return Err(format!("duplicate output path {}", output.path));
            }
            if output.roots.is_empty() {
                return Err(format!("output {}: roots must not be empty", output.id));
            }
            if let Some(title) = &output.title {
                if !single_line(title) {
                    return Err(format!(
                        "output {}: title must be nonempty single-line text",
                        output.id
                    ));
                }
                content_valid(title)?;
            }
            match (&output.format, &output.skill) {
                (Format::Skill, Some(skill)) => {
                    skill
                        .validate()
                        .map_err(|e| format!("output {}: {e}", output.id))?;
                    let path = Path::new(&output.path);
                    if path.file_name().and_then(|s| s.to_str()) != Some("SKILL.md")
                        || path
                            .parent()
                            .and_then(|p| p.file_name())
                            .and_then(|s| s.to_str())
                            != Some(&skill.name)
                    {
                        return Err(format!(
                            "output {}: skill path must end in {}/SKILL.md",
                            output.id, skill.name
                        ));
                    }
                }
                (Format::Skill, None) => {
                    return Err(format!("output {}: missing skill metadata", output.id));
                }
                (_, Some(_)) => {
                    return Err(format!(
                        "output {}: only skill format accepts skill metadata",
                        output.id
                    ));
                }
                _ => {}
            }
            if output.format == Format::RootGuide {
                roots += 1;
                if output.path != "CLAUDE.md" {
                    return Err("root-guide path must be CLAUDE.md".into());
                }
            } else if output.path == "CLAUDE.md" {
                return Err("CLAUDE.md is reserved for root-guide".into());
            }
        }
        if roots != 1 {
            return Err("registration must contain exactly one root-guide".into());
        }
        Ok(())
    }
    pub fn root(&self) -> &Output {
        self.outputs
            .iter()
            .find(|o| o.format == Format::RootGuide)
            .unwrap()
    }
}
impl Catalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("unsupported catalog schema_version (expected 1)".into());
        }
        let mut locales = BTreeSet::new();
        for locale in &self.locales {
            identity(&locale.id)?;
            if !locales.insert(&locale.id) {
                return Err(format!("duplicate locale {}", locale.id));
            }
            if locale.root_frame.trim().is_empty() || locale.projection_notice.trim().is_empty() {
                return Err(format!("locale {}: empty frame/notice", locale.id));
            }
            content_valid(&locale.root_frame)?;
            content_valid(&locale.projection_notice)?;
        }
        let mut atoms = BTreeMap::new();
        for atom in &self.atoms {
            identity(&atom.id)?;
            if atoms.insert(atom.id.as_str(), atom).is_some() {
                return Err(format!("duplicate atom {}", atom.id));
            }
            let mut variants = BTreeSet::new();
            for variant in &atom.variants {
                if !locales.contains(&variant.locale) {
                    return Err(format!(
                        "atom {}: unregistered locale {}",
                        atom.id, variant.locale
                    ));
                }
                if !variants.insert(&variant.locale) {
                    return Err(format!(
                        "atom {}: duplicate variant {}",
                        atom.id, variant.locale
                    ));
                }
            }
        }
        let mut visited = BTreeSet::new();
        for atom in &self.atoms {
            visit(
                &atom.id,
                &atoms,
                &mut visited,
                &mut Vec::new(),
                &mut Vec::new(),
            )?;
        }
        Ok(())
    }
    pub fn file_inputs(&self) -> Vec<&str> {
        self.atoms
            .iter()
            .flat_map(|a| &a.variants)
            .filter_map(|v| match &v.source {
                Content::File { path } => Some(path.as_str()),
                _ => None,
            })
            .collect()
    }
    pub fn compose(
        &self,
        output: &Output,
        files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(String, &Locale), String> {
        let locale = self
            .locales
            .iter()
            .find(|l| l.id == output.locale)
            .ok_or_else(|| {
                format!(
                    "output {}: unregistered locale {}",
                    output.id, output.locale
                )
            })?;
        let atoms: BTreeMap<_, _> = self.atoms.iter().map(|a| (a.id.as_str(), a)).collect();
        let mut order = Vec::new();
        let mut visited = BTreeSet::new();
        for root in &output.roots {
            visit(root, &atoms, &mut visited, &mut Vec::new(), &mut order)
                .map_err(|e| format!("output {}: {e}", output.id))?;
        }
        let mut parts = Vec::new();
        for atom in order {
            let variant = atom
                .variants
                .iter()
                .find(|v| v.locale == output.locale)
                .ok_or_else(|| {
                    format!(
                        "output {}: atom {} missing selected locale {}",
                        output.id, atom.id, output.locale
                    )
                })?;
            let text = match &variant.source {
                Content::Inline { text } => text.as_str(),
                Content::File { path } => std::str::from_utf8(
                    files
                        .get(path)
                        .ok_or_else(|| format!("missing registered file input {path}"))?,
                )
                .map_err(|e| e.to_string())?,
            };
            content_valid(text).map_err(|e| {
                format!(
                    "output {} / atom {} / locale {}: {e}",
                    output.id, atom.id, output.locale
                )
            })?;
            if !text.is_empty() {
                parts.push(text);
            }
        }
        Ok((parts.join("\n\n"), locale))
    }
}
fn visit<'a>(
    id: &str,
    atoms: &BTreeMap<&str, &'a Atom>,
    visited: &mut BTreeSet<String>,
    stack: &mut Vec<String>,
    order: &mut Vec<&'a Atom>,
) -> Result<(), String> {
    if stack.iter().any(|s| s == id) {
        let mut chain = stack.clone();
        chain.push(id.into());
        return Err(format!("dependency cycle: {}", chain.join(" -> ")));
    }
    if visited.contains(id) {
        return Ok(());
    }
    let atom = atoms.get(id).ok_or_else(|| {
        format!(
            "unresolved atom {id}; dependency chain: {}",
            stack.join(" -> ")
        )
    })?;
    stack.push(id.into());
    for required in &atom.requires {
        visit(required, atoms, visited, stack, order)?;
    }
    stack.pop();
    visited.insert(id.into());
    order.push(atom);
    Ok(())
}
impl Output {
    fn marker(&self, edge: &str) -> String {
        format!(
            "<!-- chrono-instructions:output producer=chrono-instructions id={} format={} {edge} -->",
            self.id,
            self.format.label()
        )
    }
    pub fn title_body(&self, body: &str) -> String {
        match &self.title {
            Some(title) => format!("# {title}\n\n{body}"),
            None => body.into(),
        }
    }
    pub fn projection(&self, notice: &str, body: &str) -> Vec<u8> {
        format!(
            "{}{}\n{}\n\n{}\n{}\n",
            self.skill.as_ref().map_or(String::new(), Skill::header),
            self.marker("begin"),
            notice,
            self.title_body(body),
            self.marker("end")
        )
        .into_bytes()
    }
    pub fn check_owned(&self, bytes: &[u8]) -> Result<(), String> {
        let failure = || {
            format!(
                "output {} ({}): unowned or malformed projection envelope; preserve the existing file and resolve ownership explicitly",
                self.id, self.path
            )
        };
        let text = std::str::from_utf8(bytes).map_err(|_| failure())?;
        let body = if self.format == Format::Skill {
            // Accept only the producer's scalar frontmatter shape, retaining YAML at byte zero.
            let mut lines = text.splitn(5, '\n');
            if lines.next() != Some("---") {
                return Err(failure());
            }
            let name = lines
                .next()
                .and_then(|s| s.strip_prefix("name: "))
                .ok_or_else(failure)?;
            let description = lines
                .next()
                .and_then(|s| s.strip_prefix("description: "))
                .ok_or_else(failure)?;
            let _: String = serde_json::from_str(name).map_err(|_| failure())?;
            let _: String = serde_json::from_str(description).map_err(|_| failure())?;
            if lines.next() != Some("---") {
                return Err(failure());
            }
            lines.next().ok_or_else(failure)?
        } else {
            text
        };
        if !body.starts_with(&format!("{}\n", self.marker("begin")))
            || !body.ends_with(&format!("\n{}\n", self.marker("end")))
            || text.matches("<!-- chrono-instructions").count() != 2
        {
            return Err(failure());
        }
        Ok(())
    }
}
