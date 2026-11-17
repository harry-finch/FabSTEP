use fabstep_app::{preview_document, InputFormat, SOURCE_MAX_TRIANGLES, SOURCE_MAX_VERTICES};
use fabstep_core::Options;

// Minimal OPC package with stored ZIP members. This avoids adding a test
// dependency merely to exercise the existing 3MF limits in both feature builds.
fn package(model: &str) -> Vec<u8> {
    let members = [
        ("[Content_Types].xml", "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/></Types>"),
        ("_rels/.rels", "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"r\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\" Target=\"/3D/3dmodel.model\"/></Relationships>"),
        ("3D/3dmodel.model", model),
    ];
    fn crc(bytes: &[u8]) -> u32 {
        let mut value = !0u32;
        for &b in bytes {
            value ^= b as u32;
            for _ in 0..8 {
                value = (value >> 1) ^ (0xedb88320u32 & (0u32.wrapping_sub(value & 1)));
            }
        }
        !value
    }
    fn u16le(out: &mut Vec<u8>, n: u16) {
        out.extend(n.to_le_bytes());
    }
    fn u32le(out: &mut Vec<u8>, n: u32) {
        out.extend(n.to_le_bytes());
    }
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, text) in members {
        let offset = out.len() as u32;
        let checksum = crc(text.as_bytes());
        u32le(&mut out, 0x04034b50);
        for n in [20, 0, 0, 0, 33] {
            u16le(&mut out, n);
        }
        for n in [checksum, text.len() as u32, text.len() as u32] {
            u32le(&mut out, n);
        }
        u16le(&mut out, name.len() as u16);
        u16le(&mut out, 0);
        out.extend(name.as_bytes());
        out.extend(text.as_bytes());
        u32le(&mut central, 0x02014b50);
        for n in [20, 20, 0, 0, 0, 33] {
            u16le(&mut central, n);
        }
        for n in [checksum, text.len() as u32, text.len() as u32] {
            u32le(&mut central, n);
        }
        for n in [name.len() as u16, 0, 0, 0, 0] {
            u16le(&mut central, n);
        }
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend(name.as_bytes());
    }
    let offset = out.len() as u32;
    out.extend(&central);
    u32le(&mut out, 0x06054b50);
    for n in [0, 0, 3, 3] {
        u16le(&mut out, n);
    }
    u32le(&mut out, central.len() as u32);
    u32le(&mut out, offset);
    u16le(&mut out, 0);
    out
}
fn model(triangles: usize, instances: usize) -> Vec<u8> {
    let triangles = "<triangle v1=\"0\" v2=\"1\" v3=\"2\"/>".repeat(triangles);
    let items = "<item objectid=\"1\"/>".repeat(instances);
    package(&format!("<model xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" unit=\"millimeter\"><resources><object id=\"1\" type=\"model\"><mesh><vertices><vertex x=\"0\" y=\"0\" z=\"0\"/><vertex x=\"1\" y=\"0\" z=\"0\"/><vertex x=\"0\" y=\"1\" z=\"0\"/></vertices><triangles>{triangles}</triangles></mesh></object></resources><build>{items}</build></model>"))
}

#[test]
fn isolated_stl_profile_preserves_3mf_raw_and_placed_triangle_limits() {
    assert_eq!(SOURCE_MAX_VERTICES, 3 * SOURCE_MAX_TRIANGLES);
    assert_eq!(
        SOURCE_MAX_TRIANGLES,
        if cfg!(feature = "large-stl") {
            600_000
        } else {
            100_000
        }
    );
    let o = Options::default();
    assert_eq!(
        preview_document(&model(1, 1), InputFormat::ThreeMf, &o)
            .unwrap()
            .bodies
            .len(),
        1
    );
    for input in [model(100_001, 1), model(3_334, 31)] {
        assert_eq!(
            preview_document(&input, InputFormat::ThreeMf, &o)
                .unwrap_err()
                .code,
            "threemf.complexity_limit"
        );
    }
    assert_eq!(fabstep_import::MAX_TRIANGLES, 100_000);
}
