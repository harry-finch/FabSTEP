//! Offline R-080 qualification. Paths are supplied by the caller so no private
//! input or working-directory convention enters the ordinary test suite.
#![cfg(feature = "large-stl")]
use fabstep_core::{
    analyze_stl, recognize_automatic, Mesh, Options, ReconstructionMode, RoundedFaceRef,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::PathBuf,
    time::Instant,
};

#[derive(Default)]
struct Patch {
    triangles: usize,
    owners: BTreeSet<usize>,
    bounds: Option<[[f64; 3]; 2]>,
    area: f64,
    normal_z: [usize; 3], // near horizontal (|nz|>=.99), near vertical (<=.01), other
    seed: usize,
}

impl Patch {
    fn include(&mut self, mesh: &Mesh, tri: usize, owner: usize, normal: [f64; 3], area: f64) {
        if self.triangles == 0 {
            self.seed = tri;
        }
        self.triangles += 1;
        self.owners.insert(owner);
        self.area += area;
        let z = normal[2].abs();
        self.normal_z[if z >= 0.99 {
            0
        } else if z <= 0.01 {
            1
        } else {
            2
        }] += 1;
        for index in mesh.triangles[tri] {
            let point = mesh.vertices[index];
            let bounds = self.bounds.get_or_insert([point, point]);
            for (i, x) in point.into_iter().enumerate() {
                bounds[0][i] = bounds[0][i].min(x);
                bounds[1][i] = bounds[1][i].max(x);
            }
        }
    }
}

fn normal_and_area(mesh: &Mesh, t: [usize; 3]) -> ([f64; 3], f64) {
    let [a, b, c] = t.map(|i| mesh.vertices[i]);
    let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
    let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = cross.iter().map(|x| x * x).sum::<f64>().sqrt();
    assert!(length > 0. && length.is_finite());
    (cross.map(|x| x / length), length * 0.5)
}

struct Neighbors {
    pair: [usize; 2],
    dot: f64,
}
fn shared_edges(
    mesh: &Mesh,
    geometry: &[([f64; 3], f64)],
    owners: &[RoundedFaceRef],
    mut analytic_seams: Option<&mut std::io::BufWriter<fs::File>>,
) -> Vec<Neighbors> {
    let mut edges = Vec::with_capacity(mesh.triangles.len() * 3);
    for (i, t) in mesh.triangles.iter().enumerate() {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            edges.push((a.min(b), a.max(b), i));
        }
    }
    edges.sort_unstable();
    let mut neighbors = Vec::new();
    for uses in edges.chunk_by(|a, b| (a.0, a.1) == (b.0, b.1)) {
        assert_eq!(uses.len(), 2, "analysis must be a closed two-manifold");
        let pair = [uses[0].2, uses[1].2];
        if let Some(output) = analytic_seams.as_mut() {
            use std::io::Write as _;
            for (plane, other) in [(pair[0], pair[1]), (pair[1], pair[0])] {
                if matches!(owners[plane], RoundedFaceRef::Plane { .. })
                    && !matches!(owners[other], RoundedFaceRef::Plane { .. })
                {
                    let (a, b, _) = uses[0];
                    let tri = mesh.triangles[plane];
                    let edge = (0..3)
                        .find(|&i| {
                            let (u, v) = (tri[i], tri[(i + 1) % 3]);
                            (u.min(v), u.max(v)) == (a, b)
                        })
                        .unwrap();
                    writeln!(output, "{plane}\t{edge}\t{:?}", owners[other]).unwrap();
                }
            }
        }
        if pair
            .iter()
            .all(|&i| matches!(owners[i], RoundedFaceRef::Plane { .. }))
        {
            let [a, b] = pair.map(|i| geometry[i].0);
            neighbors.push(Neighbors {
                pair,
                dot: (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1., 1.),
            });
        }
    }
    neighbors
}

fn root(parents: &mut [usize], mut i: usize) -> usize {
    while parents[i] != i {
        parents[i] = parents[parents[i]];
        i = parents[i];
    }
    i
}

fn partitions(
    mesh: &Mesh,
    owners: &[RoundedFaceRef],
    geometry: &[([f64; 3], f64)],
    edges: &[Neighbors],
    degree: f64,
    dump: Option<&std::path::Path>,
) -> String {
    let mut parents: Vec<_> = (0..owners.len()).collect();
    let threshold = degree.to_radians().cos();
    for e in edges.iter().filter(|e| e.dot >= threshold) {
        let [a, b] = e.pair.map(|i| root(&mut parents, i));
        parents[a] = b;
    }
    let mut patches = BTreeMap::<usize, Patch>::new();
    for (i, owner) in owners.iter().enumerate() {
        if let RoundedFaceRef::Plane { index } = owner {
            let (n, area) = geometry[i];
            patches
                .entry(root(&mut parents, i))
                .or_default()
                .include(mesh, i, *index, n, area);
        }
    }
    if let Some(path) = dump {
        use std::io::{BufWriter, Write};
        let mut file = BufWriter::new(fs::File::create(path).unwrap());
        // Little endian, no header: analysis triangle, component seed, planar
        // owner (3 x u32), then its three vertices in mm (9 x f64).
        for (tri, owner) in owners.iter().enumerate() {
            if let RoundedFaceRef::Plane { index } = owner {
                for id in [tri, patches[&root(&mut parents, tri)].seed, *index] {
                    file.write_all(&u32::try_from(id).unwrap().to_le_bytes())
                        .unwrap();
                }
                for v in mesh.triangles[tri] {
                    for coord in mesh.vertices[v] {
                        file.write_all(&coord.to_le_bytes()).unwrap();
                    }
                }
            }
        }
        file.flush().unwrap();
    }
    let mut patches: Vec<_> = patches.into_values().collect();
    patches.sort_by_key(|p| (std::cmp::Reverse(p.owners.len()), p.seed));
    let triangle_count: usize = patches.iter().map(|p| p.triangles).sum();
    assert_eq!(
        triangle_count,
        owners
            .iter()
            .filter(|o| matches!(o, RoundedFaceRef::Plane { .. }))
            .count()
    );
    let mut out=format!("{{\"max_dihedral_deg\":{degree},\"components\":{},\"triangles\":{triangle_count},\"largest_by_planar_owners\":[",patches.len());
    for (rank, p) in patches.iter().take(32).enumerate() {
        if rank > 0 {
            out.push(',');
        }
        write!(out,"{{\"first_analysis_triangle\":{},\"triangles\":{},\"owners\":{},\"bounds_mm\":{:?},\"area_mm2\":{},\"normal_z_bins\":{:?}}}",p.seed,p.triangles,p.owners.len(),p.bounds.unwrap(),p.area,p.normal_z).unwrap();
    }
    out.push_str("]}");
    out
}

#[test]
#[ignore = "private R-080 qualification; set FABSTEP_PROFILE_STL and FABSTEP_PROFILE_DIR"]
fn residual_planar_profile() {
    let source = PathBuf::from(std::env::var_os("FABSTEP_PROFILE_STL").unwrap());
    let dir = PathBuf::from(std::env::var_os("FABSTEP_PROFILE_DIR").unwrap());
    assert!(fs::metadata(&source).unwrap().len() <= 64 * 1024 * 1024);
    let bytes = fs::read(source).unwrap();
    let options = Options {
        reconstruction: ReconstructionMode::Auto,
        ..Options::default()
    };
    let start = Instant::now();
    let analysis = analyze_stl(&bytes, &options).unwrap();
    drop(bytes);
    let analyzed = start.elapsed();
    let r = recognize_automatic(&analysis, &options).unwrap();
    let recognized = start.elapsed() - analyzed;
    let owners = &r.triangle_faces;
    let mesh = &analysis.mesh;
    assert_eq!(mesh.triangles.len(), owners.len());
    let mut planar_usage = vec![0; r.model.revolved.mixed.planes.len()];
    let mut analytic = BTreeMap::new();
    for o in owners {
        match o {
            RoundedFaceRef::Plane { index } => planar_usage[*index] += 1,
            o => *analytic.entry(format!("{o:?}")).or_insert(0) += 1,
        }
    }
    assert!(planar_usage.iter().all(|&n| n > 0));
    let mut usage = BTreeMap::new();
    for n in &planar_usage {
        *usage.entry(*n).or_insert(0) += 1;
    }
    let geometry: Vec<_> = mesh
        .triangles
        .iter()
        .map(|&t| normal_and_area(mesh, t))
        .collect();
    fs::create_dir_all(&dir).unwrap();
    let mut seams = std::env::var_os("FABSTEP_DUMP_ANALYTIC_SEAMS").map(|_| {
        std::io::BufWriter::new(fs::File::create(dir.join("analytic-seams.tsv")).unwrap())
    });
    let edges = shared_edges(mesh, &geometry, owners, seams.as_mut());
    if let Some(mut writer) = seams {
        std::io::Write::flush(&mut writer).unwrap();
    }
    let dump = std::env::var_os("FABSTEP_DUMP_PLANES").map(|_| dir.join("planes-3deg.bin"));
    let splits = [180., 12., 3.].map(|degrees| {
        let output = if degrees == 3. { dump.as_deref() } else { None };
        partitions(mesh, owners, &geometry, &edges, degrees, output)
    });
    let pairs: Vec<_> = usage.iter().map(|(n, count)| [n, count]).collect();
    let mut angles: Vec<_> = edges.iter().map(|e| e.dot.acos().to_degrees()).collect();
    angles.sort_by(f64::total_cmp);
    let quantiles = [0, 50, 90, 99, 100].map(|q| {
        angles
            .get(q * angles.len().saturating_sub(1) / 100)
            .copied()
    });
    let angle_json = quantiles
        .map(|q| q.map_or("null".to_owned(), |x| x.to_string()))
        .join(",");
    let data=format!("{{\"source_triangles\":{},\"analysis_seconds\":{},\"recognition_seconds\":{},\"planar_owners\":{},\"planar_triangles\":{},\"triangles_per_owner_histogram\":{:?},\"analytic_owners\":{:?},\"planar_edges\":{},\"planar_dihedral_deg_min_p50_p90_p99_max\":[{}],\"partitions\":[{}]}}\n",
        owners.len(), analyzed.as_secs_f64(), recognized.as_secs_f64(),planar_usage.len(),planar_usage.iter().sum::<usize>(),pairs,analytic,edges.len(),angle_json,splits.join(","));
    fs::write(dir.join("residual-planar.json"), data).unwrap();
}
