use crate::{archive::part_name, error};
use fabstep_core::FabError;
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::BTreeMap;
pub(crate) const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
pub(crate) const PRODUCTION: &str =
    "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPES: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const MODEL_REL: &str = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";
const MODEL_TYPE: &str = "application/vnd.ms-package.3dmanufacturing-3dmodel+xml";
pub(crate) fn invalid() -> FabError {
    error("invalid_xml", "Structure XML 3MF/OPC invalide.")
}
pub(crate) fn parse(source: &str) -> Result<Document<'_>, FabError> {
    preflight_depth(source.as_bytes())?;
    let doc = Document::parse_with_options(
        source,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 1_000_000,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| invalid())?;
    // Recheck the parsed tree; lexical preflight already bounds parser recursion.
    if doc
        .descendants()
        .any(|n| n.ancestors().take(65).count() > 64)
    {
        return Err(error("xml_limit", "Profondeur XML limitée à 64 niveaux."));
    }
    Ok(doc)
}
pub(crate) fn required<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str, FabError> {
    node.attribute(name).ok_or_else(invalid)
}
pub(crate) fn is(node: Node<'_, '_>, ns: &str, name: &str) -> bool {
    node.has_tag_name((ns, name))
}
pub(crate) fn children<'a, 'i>(node: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    node.children().filter(Node::is_element)
}
/// Element-only content in geometry and OPC. Metadata payloads are exempt.
pub(crate) fn structural(node: Node<'_, '_>) -> Result<(), FabError> {
    if node.children().any(|n| {
        n.is_text()
            && n.text()
                .is_some_and(|s| s.chars().any(|c| !matches!(c, ' ' | '\t' | '\r' | '\n')))
    }) {
        return Err(invalid());
    }
    Ok(())
}
fn opc_attributes(node: Node<'_, '_>, allowed: &[&str]) -> Result<(), FabError> {
    structural(node)?;
    if node
        .attributes()
        .any(|a| a.namespace().is_some() || !allowed.contains(&a.name()))
    {
        return Err(invalid());
    }
    Ok(())
}
pub(crate) fn resource_id(text: &str) -> Result<u32, FabError> {
    let id: u32 = text.parse().map_err(|_| invalid())?;
    if id == 0 || id >= 1 << 31 {
        return Err(invalid());
    }
    Ok(id)
}
pub(crate) fn number(text: &str) -> Result<f64, FabError> {
    let n: f64 = text.parse().map_err(|_| invalid())?;
    if !n.is_finite() {
        return Err(error(
            "non_finite",
            "Coordonnée ou transformation non finie.",
        ));
    }
    Ok(n)
}
pub(crate) fn attributes(
    node: Node<'_, '_>,
    allowed: &[&str],
    ignored: &mut usize,
) -> Result<(), FabError> {
    for a in node.attributes() {
        match a.namespace() {
            None => {
                if !allowed.contains(&a.name()) {
                    return Err(invalid());
                }
            }
            Some(PRODUCTION) if a.name() == "path" => {
                return Err(error(
                    "unsupported_extension",
                    "Les références Production vers d'autres modèles ne sont pas prises en charge.",
                ))
            }
            Some("http://www.w3.org/XML/1998/namespace") => {}
            Some(_) => *ignored += 1,
        }
    }
    Ok(())
}
pub(crate) fn root_part(rels: &str) -> Result<String, FabError> {
    let doc = parse(rels)?;
    let root = doc.root_element();
    if !is(root, REL, "Relationships") {
        return Err(invalid());
    }
    opc_attributes(root, &[])?;
    let mut ids = std::collections::BTreeSet::new();
    let mut model = None;
    for node in children(root) {
        if !is(node, REL, "Relationship") || !ids.insert(required(node, "Id")?) {
            return Err(invalid());
        }
        opc_attributes(node, &["Id", "Type", "Target", "TargetMode"])?;
        if children(node).next().is_some() {
            return Err(invalid());
        }
        if node
            .attribute("TargetMode")
            .is_some_and(|v| v != "Internal")
        {
            return Err(error(
                "external_resource",
                "Les ressources externes au paquet 3MF sont refusées.",
            ));
        }
        let target = part_name(required(node, "Target")?, true)?;
        if required(node, "Type")? == MODEL_REL && model.replace(target).is_some() {
            return Err(error(
                "invalid_package",
                "Plusieurs relations StartPart 3MF sont ambiguës.",
            ));
        }
    }
    model.ok_or_else(|| error("missing_part", "Relation StartPart 3MF absente."))
}
pub(crate) fn content_types(source: &str, model: &str) -> Result<(), FabError> {
    let doc = parse(source)?;
    if !is(doc.root_element(), TYPES, "Types") {
        return Err(invalid());
    }
    opc_attributes(doc.root_element(), &[])?;
    let mut defaults = BTreeMap::new();
    let mut overrides = BTreeMap::new();
    for node in children(doc.root_element()) {
        let value = required(node, "ContentType")?;
        if children(node).next().is_some() {
            return Err(invalid());
        }
        if is(node, TYPES, "Default") {
            opc_attributes(node, &["Extension", "ContentType"])?;
            let key = required(node, "Extension")?.to_ascii_lowercase();
            if defaults.insert(key, value).is_some() {
                return Err(invalid());
            }
        } else if is(node, TYPES, "Override") {
            opc_attributes(node, &["PartName", "ContentType"])?;
            let key = part_name(required(node, "PartName")?, true)?.to_ascii_lowercase();
            if overrides.insert(key, value).is_some() {
                return Err(invalid());
            }
        } else {
            return Err(invalid());
        }
    }
    let key = model.to_ascii_lowercase();
    let ext = key.rsplit_once('.').map(|(_, s)| s).unwrap_or("");
    if overrides.get(&key).or_else(|| defaults.get(ext)).copied() != Some(MODEL_TYPE) {
        return Err(error(
            "invalid_package",
            "Type de contenu OPC du modèle 3MF absent ou incorrect.",
        ));
    }
    Ok(())
}

// Bound nesting BEFORE roxmltree's recursive tokenizer. This is only a lexical
// guard: roxmltree remains responsible for complete syntax/namespace validation.
fn preflight_depth(bytes: &[u8]) -> Result<(), FabError> {
    let mut i = 0;
    let mut depth = 0_usize;
    let mut attributes = 0_usize;
    let mut namespace_stack: Vec<usize> = Vec::new();
    let mut namespace_work = 0_usize;
    let mut namespace_allocations = 0_usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let tail = &bytes[i..];
        let special: Option<(&[u8], usize)> = if tail.starts_with(b"<!--") {
            Some((b"-->", 4))
        } else if tail.starts_with(b"<![CDATA[") {
            Some((b"]]>", 9))
        } else if tail.starts_with(b"<?") {
            Some((b"?>", 2))
        } else {
            None
        };
        if let Some((end, offset)) = special {
            let n = tail[offset..]
                .windows(end.len())
                .position(|w| w == end)
                .ok_or_else(invalid)?;
            i += offset + n + end.len();
            continue;
        }
        if tail.starts_with(b"<!") {
            return Err(invalid());
        }
        let closing = tail.starts_with(b"</");
        let start = i;
        i += 1;
        let mut quote = None;
        let mut tag_attributes = 0;
        let mut declarations = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if let Some(q) = quote {
                if b == q {
                    quote = None;
                }
            } else if b == b'\'' || b == b'"' {
                quote = Some(b);
            } else if b == b'=' {
                tag_attributes += 1;
                attributes += 1;
                if tag_attributes > 64 || attributes > 2_000_000 {
                    return Err(error("xml_limit", "Trop d'attributs XML."));
                }
                // Count declarations lexically, including repeated prefixes.
                // This deliberately overestimates a scope that shadows names.
                let mut end = i;
                while end > start && bytes[end - 1].is_ascii_whitespace() {
                    end -= 1;
                }
                let mut begin = end;
                while begin > start
                    && !bytes[begin - 1].is_ascii_whitespace()
                    && bytes[begin - 1] != b'<'
                {
                    begin -= 1;
                }
                let name = &bytes[begin..end];
                if name == b"xmlns" || name.starts_with(b"xmlns:") {
                    declarations += 1;
                }
            } else if b == b'>' {
                break;
            }
            i += 1;
        }
        if i == bytes.len() {
            return Err(invalid());
        }
        if i - start > 16 * 1024 {
            return Err(error("xml_limit", "Balise XML limitée à 16 Kio."));
        }
        if closing {
            depth = depth.checked_sub(1).ok_or_else(invalid)?;
            namespace_stack.pop().ok_or_else(invalid)?;
        } else {
            let active = namespace_stack.last().copied().unwrap_or(0) + declarations;
            if active > 32 {
                return Err(error(
                    "xml_limit",
                    "Portée XML limitée à 32 déclarations d'espaces de noms.",
                ));
            }
            if declarations > 0 {
                // roxmltree copies inherited namespaces on declarations and
                // checks duplicates against the growing scope (quadratic).
                namespace_work += active * active;
                namespace_allocations += active;
                if namespace_work > 10_000_000 || namespace_allocations > 1_000_000 {
                    return Err(error(
                        "xml_limit",
                        "Budget de résolution des espaces de noms XML dépassé.",
                    ));
                }
            }
            if i == start || bytes[i - 1] != b'/' {
                depth += 1;
                namespace_stack.push(active);
            }
            if depth > 62 {
                return Err(error(
                    "xml_limit",
                    "Profondeur XML limitée à 64 niveaux, racine et texte compris.",
                ));
            }
        }
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn namespace_scope_and_inherited_copy_work_are_bounded_before_parsing() {
        // The previous guard admitted 384 inherited namespaces, making each
        // redeclaring sibling cause a quadratic scan in roxmltree.
        let mut nested = String::new();
        for level in 0..12 {
            nested.push_str("<e");
            for i in 0..32 {
                nested.push_str(&format!(" xmlns:n{level}_{i}=\"urn:{level}:{i}\""));
            }
            nested.push('>');
        }
        nested.push_str(&"<e xmlns:new=\"urn:x\"/>".repeat(500));
        nested.push_str(&"</e>".repeat(12));
        assert_eq!(parse(&nested).unwrap_err().code, "threemf.xml_limit");
        let mut wide = String::from("<root");
        for i in 0..31 {
            wide.push_str(&format!(" xmlns:n{i}=\"urn:{i}\""));
        }
        wide.push('>');
        let suffix = "<e xmlns:new=\"urn:x\"/>";
        let small = format!("{wide}{suffix}{suffix}</root>");
        assert!(parse(&small).is_ok());
        wide.push_str(&suffix.repeat(10_000));
        wide.push_str("</root>");
        assert_eq!(parse(&wide).unwrap_err().code, "threemf.xml_limit");
    }
    #[test]
    fn opc_relationships_and_content_types_are_element_only_with_leaf_entries() {
        let rel="<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"r\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\" Target=\"/3D/main.model\"/></Relationships>";
        assert!(root_part(rel).is_ok());
        for bad in [
            rel.replace(
                "/></Relationships>",
                "><Unexpected/></Relationship></Relationships>",
            ),
            rel.replace("<Relationship Id", "bad<Relationship Id"),
            rel.replace("<Relationship Id", "<Relationship bogus=\"1\" Id"),
        ] {
            assert_eq!(root_part(&bad).unwrap_err().code, "threemf.invalid_xml");
        }
        let types="<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"><Unexpected/></Default></Types>";
        assert_eq!(
            content_types(types, "3D/main.model").unwrap_err().code,
            "threemf.invalid_xml"
        );
    }
}
