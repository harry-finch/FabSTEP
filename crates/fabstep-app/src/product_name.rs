use unicode_segmentation::UnicodeSegmentation;

/// The CAD boundary accepts at most 60 UTF-8 bytes per product name.
pub(crate) fn product_name(name: &str, fallback: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    let clean = clean.trim();
    let end = if clean.len() <= 60 {
        clean.len()
    } else {
        clean
            .grapheme_indices(true)
            .map(|(start, cluster)| start + cluster.len())
            .take_while(|&end| end <= 60)
            .last()
            .unwrap_or(0)
    };
    let truncated = clean[..end].trim_end();
    if truncated.is_empty() {
        fallback.into()
    } else {
        truncated.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_clusters_on_either_side_of_the_byte_limit() {
        for cluster in ["e\u{301}", "🇫🇷", "👩🏽‍🔧", "界"] {
            let prefix = "a".repeat(60 - cluster.len());
            let exact = format!("{prefix}{cluster}");
            assert_eq!(product_name(&format!("{exact}x"), "FabSTEP"), exact);
            let too_long = format!("{prefix}x{cluster}");
            assert_eq!(product_name(&too_long, "FabSTEP"), format!("{prefix}x"));
        }
    }

    #[test]
    fn preserves_cleaning_and_handles_one_oversize_grapheme() {
        assert_eq!(product_name("  A\\B\t ", "FabSTEP"), "A/B");
        assert_eq!(product_name("\n ", "FabSTEP"), "FabSTEP");
        let prefix = "a".repeat(59);
        assert_eq!(product_name(&format!("{prefix} é"), "FabSTEP"), prefix);
        let cluster = format!("a{}", "\u{301}".repeat(30));
        assert_eq!(product_name(&cluster, "FabSTEP"), "FabSTEP");
    }
}
