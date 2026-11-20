//! Public single-solid corpus probe. Uses the desktop's Auto conversion and exact source map.
use fabstep_app::{convert_document_with_provenance, InputFormat};
use fabstep_core::{FabError, Options, ReconstructionMode};
use serde_json::{json, Value};
use std::io::{Read, Write};

fn measure(bytes: &[u8]) -> Result<Value, FabError> {
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let converted = convert_document_with_provenance(bytes, InputFormat::Stl, &options)?;
    let [body] = converted.report.bodies.as_slice() else {
        return Err(FabError::new("corpus.body_count", "Un solide attendu."));
    };
    let inspected = converted.step.result_faces()?;
    let fidelity = match &body.conversion.fidelity {
        Some(report) => report.clone(),
        None => fabstep_verify::compare_meshes(
            &body.conversion.analysis.mesh,
            &inspected.result.mesh,
            inspected.result.deflection_mm,
        )?,
    };
    let families: Vec<_> = inspected.result.faces.iter().map(|f| f.family).collect();
    Ok(json!({
        "cad": converted.report.cad, "families": families, "fidelity": fidelity,
        "mapping": inspected.source_correspondence, "settings": converted.report.settings,
        "atlas": body.conversion.analysis.report.diagnostics.iter().any(|d| d.code == "recognition.trimmed_tori_fitted"),
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The runner supplies binary STL only, bounded to the ordinary source budget.
    let limit = (84 + 50 * fabstep_app::SOURCE_MAX_TRIANGLES) as u64;
    let mut source = Vec::new();
    std::io::stdin().take(limit + 1).read_to_end(&mut source)?;
    if source.len() as u64 > limit {
        return Err("corpus STL exceeds the source budget".into());
    }
    let report = match measure(&source) {
        Ok(report) => json!({"ok":report}),
        Err(error) => json!({"refusal":error}),
    };
    // Native kernel messages may appear on stdout. A separate path carries the result.
    let output = std::env::args_os().nth(1).ok_or("missing report path")?;
    std::fs::File::create_new(output)?.write_all(&serde_json::to_vec(&report)?)?;
    Ok(())
}
