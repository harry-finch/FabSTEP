use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src/occt.cpp");
    println!("cargo:rerun-if-env-changed=OCCT_ROOT");
    println!("cargo:rerun-if-env-changed=HOMEBREW_PREFIX");

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

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-isystem")
        .flag(&include)
        .file("src/occt.cpp")
        .compile("fabstep_occt");

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
