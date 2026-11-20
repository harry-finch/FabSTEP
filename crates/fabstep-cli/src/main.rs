use fabstep_app::InputFormat;
use fabstep_core::{FabError, InputUnit, Options, ReconstructionMode};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

const HELP: &str = "FabSTEP — reconstruction STL / 3MF → STEP (GPL-3.0-or-later)

Usage :
  fabstep check <entrée.stl|entrée.3mf> [--unit mm|inch] [--json]
  fabstep convert <entrée.stl|entrée.3mf> -o <sortie.step> [--unit mm|inch] [--json] [--force]

Options :
  --mode auto|planar|cylinders|primitives|surfaces|height-patch  Stratégie de reconstruction (défaut : planar)
  --max-deviation <mm>     Budget de borne composée (défaut : 0.05 mm)
  --fit <mm>              Résidu radial maximal d'ajustement (défaut : 0.00001 mm)
  --weld <mm>    Tolérance de soudage (défaut : 0.000001 mm)
  --plane <mm>   Tolérance des régions planes (défaut : 0.000001 mm)
  --unit        Unité des coordonnées STL, absente du format (défaut explicite : mm)
  --force       Autoriser le remplacement du fichier de sortie

En mode auto, toute tolérance fournie explicitement impose les réglages exacts.
Mode cylinders : plans, cylindres complets et partiels ; bossages, trous et encoches.
Les cylindres partiels incluent les parois décalées et les petits raccords contextuels prouvés.
Mode primitives : ajoute les cônes et chanfreins prouvés, dont les chaînes de pentes alternées
séparées par des portées résolues ; pas de congés généraux.
Mode surfaces : ajoute les portions sphériques (dont les zones sans pôle) et toriques prouvées,
les sphères et tores complets, les boîtiers convexes entièrement arrondis à rayon uniforme,
et la combinaison prouvée d’un cône complet et d’un cylindre ouvert.
Mode height-patch : interprétation explicite d’un relief bicubique sur un socle rectangulaire XY.
Les surfaces libres arbitraires et les congés généraux à rayon variable restent hors périmètre.
3MF Core : unité du fichier ; --unit interdit. Corps séparés, sans fusion.
Les extensions requises inconnues et les corps en contact ou imbriqués sont refusés.
Une forme non reconnue est refusée ; le mode planar reste disponible.
Aucun historique paramétrique n'est reconstruit.
check analyse les maillages et la reconnaissance ; convert ajoute validation CAO, séparation des corps et réimport STEP.";

fn main() {
    if let Err(error) = run() {
        if std::env::args().any(|a| a == "--json") {
            eprintln!("{}", serde_json::to_string(&error).unwrap_or_default());
        } else {
            eprintln!("{error}");
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), FabError> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        println!("{HELP}");
        return Ok(());
    };
    if command == "--help" || command == "-h" {
        println!("{HELP}");
        return Ok(());
    }
    if command == "--version" {
        println!("FabSTEP {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if command != "check" && command != "convert" {
        return Err(usage("Commande inconnue."));
    }
    let input = PathBuf::from(
        args.next()
            .ok_or_else(|| usage("Fichier STL ou 3MF manquant."))?,
    );
    let mut output: Option<PathBuf> = None;
    let mut options = Options::default();
    let mut json = false;
    let mut force = false;
    let mut explicit_unit = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json = true,
            "--force" => force = true,
            "-o" | "--output" => {
                output = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| usage("Chemin de sortie manquant."))?,
                ))
            }
            "--unit" => {
                explicit_unit = true;
                options.input_unit = match args.next().as_deref() {
                    Some("mm") => InputUnit::Mm,
                    Some("inch") => InputUnit::Inch,
                    _ => return Err(usage("L'unité doit être mm ou inch.")),
                }
            }
            "--mode" => {
                options.reconstruction =
                    match args.next().as_deref() {
                        Some("auto") => ReconstructionMode::Auto,
                        Some("planar") => ReconstructionMode::Planar,
                        Some("cylinders") => ReconstructionMode::Cylinders,
                        Some("primitives") => ReconstructionMode::Primitives,
                        Some("surfaces") => ReconstructionMode::Surfaces,
                        Some("height-patch") => ReconstructionMode::HeightPatch,
                        _ => return Err(usage(
                            "Le mode doit être auto, planar, cylinders, primitives ou surfaces.",
                        )),
                    }
            }
            "--weld" | "--plane" | "--fit" | "--max-deviation" => {
                let value = args
                    .next()
                    .ok_or_else(|| usage("Tolérance manquante."))?
                    .parse::<f64>()
                    .map_err(|_| usage("Tolérance non numérique."))?;
                options.auto_tune = false;
                match arg.as_str() {
                    "--weld" => options.weld_tolerance_mm = value,
                    "--plane" => options.plane_tolerance_mm = value,
                    "--fit" => options.cylinder_fit_tolerance_mm = value,
                    _ => options.max_deviation_mm = value,
                }
            }
            _ => return Err(usage(format!("Option inconnue : {arg}"))),
        }
    }
    let format = match input
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("stl") => InputFormat::Stl,
        Some("3mf") => InputFormat::ThreeMf,
        _ => return Err(usage("L'entrée doit porter l'extension .stl ou .3mf.")),
    };
    if format == InputFormat::ThreeMf && explicit_unit {
        return Err(FabError::new(
            "import.unit_override",
            "Le 3MF porte son unité ; retirez --unit.",
        ));
    }
    if command == "check" && (output.is_some() || force) {
        return Err(usage(
            "check n'écrit pas de fichier ; retirez -o et --force.",
        ));
    }
    if command == "convert" && output.is_none() {
        return Err(usage("convert exige -o <sortie.step>."));
    }
    if let Some(path) = &output {
        if !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("step") || s.eq_ignore_ascii_case("stp"))
        {
            return Err(usage("La sortie doit porter l'extension .step ou .stp."));
        }
        let exists = path.try_exists().map_err(io_error)?;
        if exists && fs::canonicalize(path).ok() == fs::canonicalize(&input).ok() {
            return Err(usage("La source et la sortie doivent être distinctes."));
        }
        if !force && exists {
            return Err(io_error(std::io::Error::from(
                std::io::ErrorKind::AlreadyExists,
            )));
        }
    }
    const MAX_BYTES: usize = 64 * 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(&input)
        .map_err(io_error)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_BYTES {
        return Err(FabError::new(
            "input.too_large",
            "Le fichier dépasse 64 Mio.",
        ));
    }
    if command == "check" {
        let document = fabstep_app::analyze_document(&bytes, format, &options)?;
        let mut bodies = Vec::with_capacity(document.bodies.len());
        for body in &document.bodies {
            let recognition = match options.reconstruction {
                ReconstructionMode::Planar => None,
                ReconstructionMode::Auto
                | ReconstructionMode::Cylinders
                | ReconstructionMode::Primitives
                | ReconstructionMode::HeightPatch
                | ReconstructionMode::Surfaces => Some(
                    fabstep_app::recognize_surfaces(&body.analysis, &options)
                        .map_err(|e| e.with_body(&body.id, &body.name))?,
                ),
            };
            let mut body_report = serde_json::json!({"id":body.id,"name":body.name,"provenance":body.provenance,"mesh":body.analysis.report,
                "recognition":null,"mixed_recognition":null,"revolved_recognition":null,"rounded_recognition":null,
                "rounded_box_recognition":null,"height_patch_recognition":null,"cubic_prism_recognition":null,"oblique_lip_recognition":null,"oblique_band_recognition":null});
            if let Some(recognized) = &recognition {
                use fabstep_app::Recognized::*;
                let (key, value) = match recognized {
                    Extrusion(r) => ("recognition", serde_json::json!(r)),
                    Mixed(r) => ("mixed_recognition", serde_json::json!(r)),
                    Revolved(r) => ("revolved_recognition", serde_json::json!(r)),
                    Rounded(r) => ("rounded_recognition", serde_json::json!(r)),
                    RoundedBox(r) => ("rounded_box_recognition", serde_json::json!(r)),
                    HeightPatch(r) => ("height_patch_recognition", serde_json::json!(r)),
                    CubicPrism(r) => ("cubic_prism_recognition", serde_json::json!(r)),
                    ObliqueLip(r) => ("oblique_lip_recognition", serde_json::json!(r)),
                    ObliqueBand(r) => ("oblique_band_recognition", serde_json::json!(r)),
                };
                body_report[key] = value;
            }
            bodies.push(body_report);
        }
        if json {
            let mut report = serde_json::json!({"settings":document.settings,"source":document.source,"bodies":bodies,"diagnostics":document.diagnostics,"native_validated":false});
            // Champs historiques conservés uniquement pour un document mono-corps.
            if bodies.len() == 1 {
                for (key, value) in bodies[0]["mesh"].as_object().expect("report object") {
                    if key != "diagnostics" {
                        report[key] = value.clone();
                    }
                }
                let diagnostics = report["diagnostics"]
                    .as_array_mut()
                    .expect("diagnostics array");
                diagnostics.extend(
                    bodies[0]["mesh"]["diagnostics"]
                        .as_array()
                        .expect("diagnostics array")
                        .iter()
                        .cloned(),
                );
                report["recognition"] = bodies[0]["recognition"].clone();
                report["mixed_recognition"] = bodies[0]["mixed_recognition"].clone();
                report["revolved_recognition"] = bodies[0]["revolved_recognition"].clone();
                report["rounded_recognition"] = bodies[0]["rounded_recognition"].clone();
                report["height_patch_recognition"] = bodies[0]["height_patch_recognition"].clone();
                report["cubic_prism_recognition"] = bodies[0]["cubic_prism_recognition"].clone();
                report["oblique_lip_recognition"] = bodies[0]["oblique_lip_recognition"].clone();
                report["oblique_band_recognition"] = bodies[0]["oblique_band_recognition"].clone();
                report["rounded_box_recognition"] = bodies[0]["rounded_box_recognition"].clone();
            }
            print_json(&report)?;
        } else {
            let triangles: usize = document
                .bodies
                .iter()
                .map(|b| b.analysis.report.input_triangles)
                .sum();
            println!("Document analysé : {} corps, {} triangles ; unité source : {}.\nValidité CAO et séparation des corps non vérifiées par check.", document.bodies.len(), triangles, document.source.unit);
            for body in bodies {
                println!(
                    "{} : {} régions planes ; volume {} mm³.",
                    body["name"].as_str().unwrap_or("Corps"),
                    body["mesh"]["planar_faces"],
                    body["mesh"]["volume_mm3"]
                );
            }
        }
    } else {
        let result = fabstep_app::convert_document(&bytes, format, &options)?;
        let path = output.expect("output checked above");
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
        file.write_all(&result.step_bytes).map_err(io_error)?;
        file.as_file().sync_all().map_err(io_error)?;
        if force {
            file.persist(&path).map_err(|e| io_error(e.error))?;
        } else {
            file.persist_noclobber(&path)
                .map_err(|e| io_error(e.error))?;
        }
        if json {
            let bodies: Vec<_> = result.report.bodies.iter().map(|body| {
                let c = &body.conversion;
                let mut json = serde_json::json!({
                    "id": body.id, "name": body.name, "provenance": body.provenance,
                    "mesh": c.analysis.report, "cad": c.cad, "settings": c.settings,
                    "recognition": c.recognition, "mixed_recognition": c.mixed_recognition,
                    "revolved_recognition": c.revolved_recognition, "rounded_recognition": c.rounded_recognition,
                    "rounded_box_recognition": c.rounded_box_recognition, "height_patch_recognition": c.height_patch_recognition,
                    "cubic_prism_recognition": c.cubic_prism_recognition, "oblique_lip_recognition": c.oblique_lip_recognition,
                    "oblique_band_recognition": c.oblique_band_recognition, "fidelity": c.fidelity,
                });
                if let Some(adjustment) = &c.auto_adjustment {
                    json["auto_adjustment"] = serde_json::json!(adjustment);
                }
                json
            }).collect();
            let mut report = serde_json::json!({"output":path,"settings":result.report.settings,"source":result.report.source,"cad":result.report.cad,"bodies":bodies,"diagnostics":result.report.diagnostics,"native_validated":true});
            if bodies.len() == 1 {
                for key in [
                    "mesh",
                    "recognition",
                    "mixed_recognition",
                    "revolved_recognition",
                    "rounded_recognition",
                    "rounded_box_recognition",
                    "height_patch_recognition",
                    "cubic_prism_recognition",
                    "oblique_lip_recognition",
                    "oblique_band_recognition",
                    "fidelity",
                ] {
                    report[key] = bodies[0][key].clone();
                }
            }
            print_json(&report)?;
        } else {
            let reliefs = result
                .report
                .bodies
                .iter()
                .filter(|b| b.conversion.height_patch_recognition.is_some())
                .count();
            let relief_summary = if reliefs > 0 {
                format!(", {reliefs} relief(s) bicubique(s)")
            } else {
                String::new()
            };
            println!("STEP validé et réimporté : {} solide(s), {} plans, {} cylindres, {} cônes, {} sphères, {} tores, {} révolutions{relief_summary} ; volume total {:.6} mm³.\n{}", result.report.cad.solids, result.report.cad.planar_faces, result.report.cad.cylindrical_faces, result.report.cad.conical_faces, result.report.cad.spherical_faces, result.report.cad.toroidal_faces, result.report.cad.revolution_faces, result.report.cad.volume_mm3, path.display());
            for body in result.report.bodies {
                let recognition_report = body
                    .conversion
                    .recognition
                    .as_ref()
                    .map(|r| &r.report)
                    .or_else(|| {
                        body.conversion
                            .mixed_recognition
                            .as_ref()
                            .map(|r| &r.report)
                    })
                    .or_else(|| {
                        body.conversion
                            .revolved_recognition
                            .as_ref()
                            .map(|r| &r.report)
                    })
                    .or_else(|| {
                        body.conversion
                            .rounded_recognition
                            .as_ref()
                            .map(|r| &r.report)
                    })
                    .or_else(|| {
                        body.conversion
                            .rounded_box_recognition
                            .as_ref()
                            .map(|r| &r.report)
                    });
                let recognition_report = recognition_report.or_else(|| {
                    body.conversion
                        .height_patch_recognition
                        .as_ref()
                        .map(|r| &r.report)
                });
                let recognition_report = recognition_report.or_else(|| {
                    body.conversion
                        .cubic_prism_recognition
                        .as_ref()
                        .map(|r| &r.report)
                });
                let recognition_report = recognition_report.or_else(|| {
                    body.conversion
                        .oblique_lip_recognition
                        .as_ref()
                        .map(|r| &r.report)
                });
                let recognition_report = recognition_report.or_else(|| {
                    body.conversion
                        .oblique_band_recognition
                        .as_ref()
                        .map(|r| &r.report)
                });
                if let Some(report) = recognition_report {
                    println!("{} : borne d'écart {:.6} mm. Interprétation analytique explicite ; sans historique CAO.", body.name, report.max_deviation_bound_mm);
                }
                if let Some(f) = body.conversion.fidelity {
                    println!("{} : audit échantillonné source → résultat {:.6} mm ; résultat → source {:.6} mm. Déflexion demandée {:.6} mm ; pas une distance globale certifiée.", body.name, f.source_to_result_mm, f.result_to_source_mm, f.tessellation_deflection_mm);
                }
            }
            if options.reconstruction == ReconstructionMode::Planar {
                println!("Mode planaire : les surfaces courbes restent facettisées.");
            }
        }
    }
    Ok(())
}
fn usage(message: impl Into<String>) -> FabError {
    FabError::new(
        "cli.usage",
        format!("{} Voir fabstep --help.", message.into()),
    )
}
fn io_error(error: std::io::Error) -> FabError {
    FabError::new("io.file", error.to_string())
}

fn print_json(value: &serde_json::Value) -> Result<(), FabError> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|e| FabError::new("report.json", e.to_string()))?
    );
    Ok(())
}
