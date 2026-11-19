use std::collections::{BTreeMap, BTreeSet};
use tauri::ipc::Origin;
use tauri::utils::acl::{get_capabilities, resolved::Resolved, ExecutionContext};
use tauri::utils::platform::Target;

#[test]
fn atelier_is_available_only_in_the_local_main_window() {
    let manifests: BTreeMap<_, _> = serde_json::from_str(include_str!(concat!(
        env!("OUT_DIR"),
        "/acl-manifests.json"
    )))
    .unwrap();
    let mut context: tauri::Context<tauri::Wry> = tauri::generate_context!();
    let from_files =
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/capabilities.json"))).unwrap();
    let capabilities = get_capabilities(context.config(), from_files, None).unwrap();
    let acl = Resolved::resolve(&manifests, capabilities, Target::current()).unwrap();
    // Tauri enforces its ACL for local application commands when this manifest exists.
    assert!(acl.has_app_acl);
    let (_, handlers) = include_str!("../src/main.rs")
        .split_once("tauri::generate_handler![")
        .unwrap();
    let registered: BTreeSet<_> = handlers
        .split_once(']')
        .unwrap()
        .0
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|path| path.rsplit("::").next().unwrap())
        .collect();
    let allowed: BTreeSet<_> = acl.allowed_commands.keys().map(String::as_str).collect();
    assert_eq!(allowed, registered);
    for rules in acl.allowed_commands.values() {
        for rule in rules {
            assert_eq!(rule.context, ExecutionContext::Local);
            assert_eq!(rule.windows.len(), 1);
            assert_eq!(rule.windows[0].as_str(), "main");
            assert!(rule.webviews.is_empty());
        }
    }

    let authority = context.runtime_authority_mut();
    let remote = Origin::Remote {
        url: "https://example.invalid/".parse().unwrap(),
    };
    for cmd in allowed {
        for (window, origin, expected) in [
            ("main", &Origin::Local, true),
            ("secondary", &Origin::Local, false),
            ("main", &remote, false),
        ] {
            assert_eq!(
                authority
                    .resolve_access(cmd, window, window, origin)
                    .is_some(),
                expected,
                "{cmd} in {window} ({origin})"
            );
        }
    }
    for cmd in [
        "unlisted_command",
        "plugin:fs|read_file",
        "plugin:fs|write_file",
        "plugin:dialog|open",
        "plugin:dialog|save",
        "plugin:shell|execute",
        "plugin:http|fetch",
    ] {
        assert!(
            authority
                .resolve_access(cmd, "main", "main", &Origin::Local)
                .is_none(),
            "{cmd}"
        );
    }
}
