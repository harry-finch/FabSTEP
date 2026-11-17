//! Version policy shared by the build script and its standalone unit tests.

pub fn supported_version(header: &str) -> Result<String, String> {
    fn component(header: &str, name: &str) -> Result<u32, String> {
        for line in header.lines() {
            let Some(directive) = line.trim_start().strip_prefix('#') else {
                continue;
            };
            let mut words = directive.split_whitespace();
            if words.next() == Some("define") && words.next() == Some(name) {
                return words
                    .next()
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(|| format!("macro {name} invalide dans Standard_Version.hxx"));
            }
        }
        Err(format!("macro {name} absente de Standard_Version.hxx"))
    }

    let major = component(header, "OCC_VERSION_MAJOR")?;
    let minor = component(header, "OCC_VERSION_MINOR")?;
    let maintenance = component(header, "OCC_VERSION_MAINTENANCE")?;
    let version = format!("{major}.{minor}.{maintenance}");
    if major != 7 || minor != 9 {
        return Err(format!(
            "Open CASCADE {version} non pris en charge ; SDK 7.9.x requis"
        ));
    }
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::supported_version;

    fn header(major: &str, minor: &str, patch: &str) -> String {
        format!(
            "// SDK version\n#define OCC_VERSION_MAJOR {major}\n#define OCC_VERSION_MINOR {minor}\n#define OCC_VERSION_MAINTENANCE {patch}\n"
        )
    }

    #[test]
    fn accepts_supported_family_without_pinning_patch() {
        for patch in ["0", "3", "12"] {
            assert_eq!(
                supported_version(&header("7", "9", patch)).unwrap(),
                format!("7.9.{patch}")
            );
        }
    }

    #[test]
    fn rejects_other_families_with_detected_version() {
        for (major, minor) in [("7", "8"), ("7", "10"), ("8", "9")] {
            let error = supported_version(&header(major, minor, "3")).unwrap_err();
            assert!(error.contains(&format!("{major}.{minor}.3")));
            assert!(error.contains("7.9.x requis"));
        }
    }

    #[test]
    fn rejects_missing_or_malformed_numeric_macros() {
        assert!(supported_version("").unwrap_err().contains("absente"));
        for (major, minor, patch) in [("bad", "9", "3"), ("7", "bad", "3"), ("7", "9", "bad")] {
            assert!(supported_version(&header(major, minor, patch))
                .unwrap_err()
                .contains("invalide"));
        }
    }

    #[test]
    fn tolerates_preprocessor_whitespace_and_trailing_comments() {
        assert_eq!(
            supported_version(
                "  # define OCC_VERSION_MAJOR 7 // major\n\t#define\tOCC_VERSION_MINOR\t9\n#define OCC_VERSION_MAINTENANCE 3 /* patch */\n"
            )
            .unwrap(),
            "7.9.3"
        );
    }
}
