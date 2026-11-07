use crate::FabError;

pub(crate) const MAX_TRIANGLES: usize = 100_000;
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
        return Err(FabError::new(
            "input.too_many_triangles",
            "STL limité à 100 000 triangles.",
        ));
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
    let mut triangles = Vec::new();
    while let Some(token) = tokens.next() {
        if token != "facet" {
            return Err(syntax());
        }
        if triangles.len() >= MAX_TRIANGLES {
            return Err(FabError::new(
                "input.too_many_triangles",
                "STL limité à 100 000 triangles.",
            ));
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
        triangles.push(triangle);
    }
    Ok(triangles)
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
