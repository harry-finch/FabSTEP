//! Bounded, in-memory 3MF Core import. No filesystem or network access.
use fabstep_core::{Diagnostic, FabError, Mesh};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitOrigin {
    Declared,
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodySource {
    pub model_path: String,
    pub object_id: u32,
    pub build_item: usize,
    /// Component indices from the build object down to this mesh occurrence.
    pub instance_path: Vec<usize>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedBody {
    /// Coordinates already transformed into document space, in millimetres.
    pub mesh: Mesh,
    pub source: BodySource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedDocument {
    pub bodies: Vec<ImportedBody>,
    pub diagnostics: Vec<Diagnostic>,
    pub unit: String,
    pub unit_origin: UnitOrigin,
}

pub const MAX_BODIES: usize = 32;
pub const MAX_TRIANGLES: usize = 100_000;
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

mod archive;
mod model;
mod xml;
fn error(code: &str, message: &str) -> FabError {
    FabError::new(format!("threemf.{code}"), message)
}

/// Import Core mesh occurrences; geometry validation and inter-body collision
/// checks are intentionally left to the core and native document pipeline.
pub fn import_3mf(bytes: &[u8]) -> Result<ImportedDocument, FabError> {
    let mut package = archive::Package::open(bytes)?;
    let rels = package.read("_rels/.rels")?;
    let root = xml::root_part(&rels)?;
    xml::content_types(&package.read("[Content_Types].xml")?, &root)?;
    model::read(&package.read(&root)?, &root)
}
