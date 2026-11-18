use std::env;
use std::path::{Path, PathBuf};

#[path = "build_support/occt_version.rs"]
mod occt_version;

fn main() {
    println!("cargo:rerun-if-changed=src/occt.cpp");
    for file in [
        "cross_drill.hxx",
        "smooth_revolution.hxx",
        "height_patch.hxx",
        "torus_sector.hxx",
        "trim_native_step.hxx",
        "oblique_pocket.hxx",
        "unequal_pocket.hxx",
        "cross_drill_build.hxx",
        "cross_drill_verify.cpp",
        "cross_drill_certificate.hxx",
    ] {
        println!("cargo:rerun-if-changed=src/{file}");
    }
    println!("cargo:rerun-if-env-changed=OCCT_ROOT");
    println!("cargo:rerun-if-env-changed=HOMEBREW_PREFIX");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_LARGE_STL");

    let candidates = if let Some(root) = env::var_os("OCCT_ROOT") {
        vec![PathBuf::from(root)]
    } else {
        let mut roots = Vec::new();
        if let Some(prefix) = env::var_os("HOMEBREW_PREFIX") {
            roots.push(PathBuf::from(prefix).join("opt/opencascade"));
        }
        roots.extend(
            [
                "/opt/homebrew/opt/opencascade",
                "/usr/local/opt/opencascade",
                "/usr/local",
            ]
            .map(PathBuf::from),
        );
        roots
    };
    let (include, lib) = candidates
        .iter()
        .find_map(|root| sdk_paths(root))
        .expect("Open CASCADE 7.9 SDK introuvable. Installer opencascade ou définir OCCT_ROOT (include/opencascade et lib).");

    let version_header = include.join("Standard_Version.hxx");
    println!("cargo:rerun-if-changed={}", version_header.display());
    let header = std::fs::read_to_string(&version_header).unwrap_or_else(|error| {
        panic!(
            "Lecture du SDK OCCT {} impossible : {error}",
            version_header.display()
        )
    });
    let version = occt_version::supported_version(&header)
        .unwrap_or_else(|error| panic!("SDK OCCT {} : {error}", version_header.display()));
    println!("cargo:metadata=occt_version={version}");

    let mut native = cc::Build::new();
    if env::var_os("CARGO_FEATURE_LARGE_STL").is_some() {
        native.define("FABSTEP_LARGE_STL_SOURCE", None);
    }
    native
        .cpp(true)
        .std("c++17")
        .flag("-isystem")
        .flag(&include)
        .file("src/occt.cpp")
        .compile("fabstep_occt");

    // Keep outward-rounded interval operations separate from the normal CAD TU.
    let mut proof = cc::Build::new();
    proof.cpp(true).std("c++17");
    if proof.get_compiler().is_like_msvc() {
        proof.include(&include).flag("/fp:strict");
    } else {
        proof
            .flag("-fno-fast-math")
            .flag("-ffp-contract=off")
            .flag("-isystem")
            .flag(&include);
    }
    proof
        .file("src/cross_drill_verify.cpp")
        .compile("fabstep_cross_drill_proof");

    println!("cargo:rustc-link-search=native={}", lib.display());
    for toolkit in [
        "TKDESTEP",
        "TKXSBase",
        "TKPrim",
        "TKFillet",
        "TKMesh",
        "TKBO",
        "TKTopAlgo",
        "TKBRep",
        "TKG3d",
        "TKG2d",
        "TKMath",
        "TKernel",
    ] {
        println!("cargo:rustc-link-lib=dylib={toolkit}");
    }
}

fn sdk_paths(root: &Path) -> Option<(PathBuf, PathBuf)> {
    let include = root.join("include/opencascade");
    let lib = root.join("lib");
    let has_step = ["libTKDESTEP.dylib", "libTKDESTEP.so"]
        .iter()
        .any(|name| lib.join(name).is_file());
    (include.join("STEPControl_Writer.hxx").is_file() && has_step).then_some((include, lib))
}
