use crate::FabError;

pub(crate) const MAX_TRIANGLES: usize = crate::SOURCE_MAX_TRIANGLES;
const MAX_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn read(bytes: &[u8]) -> Result<Vec<[[f64; 3]; 3]>, FabError> {
    if bytes.len() > MAX_BYTES {
        return Err(FabError::new("input.too_large", "STL limité à 64 Mio."));
    }
    // The exact binary length wins, including headers beginning with `solid`.
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        let expected = 84_u64 + 50 * count as u64;
        if expected == bytes.len() as u64 {
            return binary(bytes, count);
        }
    }
    if let Ok(source) = std::str::from_utf8(bytes) {
        if source.trim_start().starts_with("solid")
            && source.chars().all(|c| !c.is_control() || c.is_whitespace())
        {
            return ascii(source);
        }
    }
    Err(FabError::new(
        "stl.invalid_length",
        "STL binaire tronqué ou longueur incohérente, ou en-tête ASCII invalide.",
    ))
}

fn binary(bytes: &[u8], count: usize) -> Result<Vec<[[f64; 3]; 3]>, FabError> {
    if count > MAX_TRIANGLES {
        return Err(too_many_triangles(count));
    }
    let mut triangles = Vec::with_capacity(count);
    for record in bytes[84..].chunks_exact(50) {
        let mut tri = [[0.0; 3]; 3];
        for (i, point) in tri.iter_mut().enumerate() {
            for (j, value) in point.iter_mut().enumerate() {
                let start = 12 + 12 * i + 4 * j;
                *value = f32::from_le_bytes(record[start..start + 4].try_into().unwrap()) as f64;
            }
        }
        triangles.push(tri);
    }
    Ok(triangles)
}

fn ascii(source: &str) -> Result<Vec<[[f64; 3]; 3]>, FabError> {
    let source = source.trim_start();
    let (header, body) = source.split_once('\n').ok_or_else(syntax)?;
    if header.split_whitespace().next() != Some("solid") {
        return Err(syntax());
    }
    // The optional footer name belongs to its line, not the rest of the file.
    let body = body.trim_end();
    let (facets, footer) = body.rsplit_once('\n').unwrap_or(("", body));
    if footer.split_whitespace().next() != Some("endsolid") {
        return Err(syntax());
    }
    let mut tokens = facets.split_whitespace();
    // A complete facet needs at least 85 bytes, even with single-character
    // numbers and separators. Reserve only this upper bound, capped before any
    // allocation; oversized ASCII is still parsed completely to validate it.
    let mut triangles = Vec::with_capacity(MAX_TRIANGLES.min(facets.len() / 85));
    let mut count = 0;
    while let Some(token) = tokens.next() {
        if token != "facet" {
            return Err(syntax());
        }
        expect(&mut tokens, "normal")?;
        // Stored normals are advisory; geometry determines the actual normals.
        for _ in 0..3 {
            number(&mut tokens)?;
        }
        expect(&mut tokens, "outer")?;
        expect(&mut tokens, "loop")?;
        let mut triangle = [[0.0; 3]; 3];
        for point in &mut triangle {
            expect(&mut tokens, "vertex")?;
            for value in point {
                *value = number(&mut tokens)?;
            }
        }
        expect(&mut tokens, "endloop")?;
        expect(&mut tokens, "endfacet")?;
        count += 1;
        if count <= MAX_TRIANGLES {
            triangles.push(triangle);
        }
    }
    if count > MAX_TRIANGLES {
        return Err(too_many_triangles(count));
    }
    Ok(triangles)
}

fn too_many_triangles(count: usize) -> FabError {
    FabError::new(
        "input.too_many_triangles",
        format!("STL : {count} triangles ; limite : {MAX_TRIANGLES}."),
    )
}

fn syntax() -> FabError {
    FabError::new(
        "stl.invalid_ascii",
        "Structure STL ASCII invalide ou tronquée.",
    )
}
fn expect(tokens: &mut std::str::SplitWhitespace<'_>, wanted: &str) -> Result<(), FabError> {
    if tokens.next() == Some(wanted) {
        Ok(())
    } else {
        Err(syntax())
    }
}
fn number(tokens: &mut std::str::SplitWhitespace<'_>) -> Result<f64, FabError> {
    tokens
        .next()
        .ok_or_else(syntax)?
        .parse()
        .map_err(|_| syntax())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACET: &str =
        "facet normal 0 0 0 outer loop vertex 0 0 0 vertex 1 0 0 vertex 0 1 0 endloop endfacet\n";

    fn ascii_source(count: usize, trailing: &str) -> String {
        let mut source = String::from("solid generated\n");
        for _ in 0..count {
            source.push_str(FACET);
        }
        source.push_str(trailing);
        source.push_str("endsolid generated\n");
        source
    }

    #[test]
    #[cfg(not(feature = "large-stl"))]
    fn binary_oversize_reports_the_exact_length_validated_triangle_count() {
        let count = 540_170u32;
        let mut source = vec![0; 84 + 50 * count as usize];
        source[..5].copy_from_slice(b"solid");
        source[80..84].copy_from_slice(&count.to_le_bytes());
        let error = read(&source).unwrap_err();
        assert_eq!(error.code, "input.too_many_triangles");
        assert_eq!(error.message, "STL : 540170 triangles ; limite : 100000.");
    }

    #[test]
    fn an_invalid_binary_length_never_reports_an_unverified_header_count() {
        let mut source = vec![0; 84];
        source[80..84].copy_from_slice(&540_170u32.to_le_bytes());
        let error = read(&source).unwrap_err();
        assert_eq!(error.code, "stl.invalid_length");
        assert!(!error.message.contains("540170"));
    }

    #[test]
    fn ascii_oversize_counts_every_complete_facet_before_refusing() {
        let count = MAX_TRIANGLES + 17;
        let error = read(ascii_source(count, "").as_bytes()).unwrap_err();
        assert_eq!(error.code, "input.too_many_triangles");
        assert_eq!(
            error.message,
            format!("STL : {count} triangles ; limite : {MAX_TRIANGLES}.")
        );
    }

    #[test]
    fn ascii_syntax_after_the_limit_is_not_hidden_by_a_partial_count() {
        let source = ascii_source(
            MAX_TRIANGLES + 1,
            "facet normal 0 0 0 outer loop vertex 0 0 0 endloop endfacet\n",
        );
        let error = read(source.as_bytes()).unwrap_err();
        assert_eq!(error.code, "stl.invalid_ascii");
        assert!(!error.message.contains("triangles"));
    }

    #[test]
    fn ascii_triangle_storage_capacity_does_not_grow_beyond_the_cap() {
        // Long names/whitespace must not inflate the retained triangle buffer.
        let source = ascii_source(MAX_TRIANGLES, &" ".repeat(1024));
        let triangles = read(source.as_bytes()).unwrap();
        assert_eq!(triangles.len(), MAX_TRIANGLES);
        assert_eq!(triangles.capacity(), MAX_TRIANGLES);
        let tiny = read(ascii_source(1, "").as_bytes()).unwrap();
        assert_eq!(tiny.len(), 1);
        assert_eq!(tiny.capacity(), 1);
    }

    #[test]
    fn binary_source_cap_is_checked_before_triangle_allocation() {
        let count = MAX_TRIANGLES + 1;
        let mut source = vec![0; 84 + 50 * count];
        source[80..84].copy_from_slice(&(count as u32).to_le_bytes());
        let error = read(&source).unwrap_err();
        assert_eq!(error.code, "input.too_many_triangles");
        assert_eq!(
            error.message,
            format!("STL : {count} triangles ; limite : {MAX_TRIANGLES}.")
        );
    }

    #[test]
    #[cfg(feature = "large-stl")]
    fn isolated_reader_accepts_the_540170_record_count_without_claiming_valid_geometry() {
        let count = 540_170usize;
        let mut source = vec![0; 84 + 50 * count];
        source[80..84].copy_from_slice(&(count as u32).to_le_bytes());
        let raw = read(&source).unwrap();
        assert_eq!(raw.len(), count);
        assert_eq!(raw.capacity(), count);
        // Zero records only prove reader storage/count handling. Topology and
        // native validation still reject this degenerate synthetic input.
    }
}
