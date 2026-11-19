//! Find missing repetitions of the last accepted wall on the immutable STEP.
//! Candidate IDs are local to this document. Applying them repeats discovery
//! and submits the complete set of indications to the ordinary validator.
use super::*;
use fabstep_cad::ResultFaceFamily;
use fabstep_core::{RoundedFaceRef, RoundedRecognition};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct SimilarCandidate {
    pub id: usize,
    pub family: &'static str,
    pub face_ids: Vec<usize>,
    pub radius_mm: Option<f64>,
    pub semi_angle_rad: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SimilarRegions {
    pub candidates: Vec<SimilarCandidate>,
    pub truncated: bool,
}

fn unavailable() -> FabError {
    FabError::new(
        "guidance.provenance_unavailable",
        "La recherche exige une indication acceptée et une correspondance exacte.",
    )
}

fn invalid() -> FabError {
    FabError::new(
        "guidance.invalid_selection",
        "La sélection de parois similaires est invalide.",
    )
}

fn limit() -> FabError {
    FabError::new(
        "guidance.limit",
        "La recherche ou la sélection dépasse les limites de guidage.",
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum End {
    Plane,
    Pole,
    Cylinder { inner: bool },
}

#[derive(Clone, Copy)]
struct Rim {
    radius: f64,
    axial: f64,
    end: End,
}

#[derive(Clone, Copy)]
struct Meridian {
    ends: [Rim; 2],
    length: f64,
}

impl Meridian {
    fn matches(self, other: Self, tolerance: f64) -> bool {
        if (self.length - other.length).abs() > tolerance {
            return false;
        }
        // A rigid placement can reverse the measured axis and the order of the rims.
        for direction in [1., -1.] {
            for ends in [other.ends, [other.ends[1], other.ends[0]]] {
                if self.ends.iter().zip(ends).all(|(a, b)| {
                    a.end == b.end
                        && (a.radius - b.radius).hypot(a.axial - direction * b.axial) <= tolerance
                }) {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone, Copy)]
struct Support {
    family: ResultFaceFamily,
    partial: bool,
    inner: bool,
    radius: f64,
    other_radius: f64,
    height: f64,
    meridian: Option<Meridian>,
}
impl Support {
    fn family_name(self) -> &'static str {
        match self.family {
            ResultFaceFamily::Cylinder => "cylinder",
            ResultFaceFamily::Cone => "cone",
            ResultFaceFamily::Sphere => "sphere",
            ResultFaceFamily::Torus => "torus",
            _ => "plane",
        }
    }
    fn guidance(self) -> GuidanceFamily {
        match self.family {
            ResultFaceFamily::Cylinder => GuidanceFamily::Cylinder,
            ResultFaceFamily::Cone => GuidanceFamily::Cone,
            ResultFaceFamily::Sphere => GuidanceFamily::Sphere,
            ResultFaceFamily::Torus => GuidanceFamily::Torus,
            _ => GuidanceFamily::Plane,
        }
    }
    fn slope(self) -> f64 {
        (self.other_radius - self.radius) / self.height
    }
    fn matches(self, other: Self, tolerance: f64) -> bool {
        self.family == other.family
            && self.partial == other.partial
            && self.inner == other.inner
            && match (self.meridian, other.meridian) {
                (Some(a), Some(b)) => a.matches(b, tolerance),
                (None, None) => true,
                _ => false,
            }
            && (self.radius - other.radius).abs() <= tolerance
            && (self.other_radius - other.other_radius).abs() <= tolerance
            && (self.family != ResultFaceFamily::Cone
                || (self.slope() - other.slope()).abs() * self.height.max(other.height)
                    <= tolerance)
    }
}

fn coupled(r: &RoundedRecognition) -> bool {
    let m = &r.model;
    !m.octants.is_empty()
        || m.linear_blend.is_some()
        || m.drilled_shaft.is_some()
        || m.cross_drill.is_some()
        || m.unequal_pocket.is_some()
        || m.smooth_revolution.is_some()
        || !m.torus_sectors.is_empty()
        || m.revolved
            .mixed
            .partial_cylinders
            .iter()
            .any(|wall| wall.elliptic_end || wall.elliptic_start)
}

fn cylinder_ends(r: &RoundedRecognition) -> Vec<End> {
    let m = &r.model.revolved.mixed;
    let mut ends = vec![End::Plane; m.rings.len()];
    for wall in &m.cylinders {
        for &ring in &wall.rings {
            ends[ring] = End::Cylinder { inner: wall.inner };
        }
    }
    ends
}
fn support(
    r: &RoundedRecognition,
    ends: &[End],
    owner: RoundedFaceRef,
) -> Option<(usize, Support)> {
    use RoundedFaceRef as Face;
    let model = &r.model;
    let mixed = &model.revolved.mixed;
    let single = |family, partial, inner, radius| Support {
        family,
        partial,
        inner,
        radius,
        other_radius: radius,
        height: 1.,
        meridian: None,
    };
    let rims = |rings: [usize; 2], center: [f64; 3]| {
        let axis = mixed.rings[rings[0]].axis;
        rings.map(|id| {
            let ring = &mixed.rings[id];
            let axial = (0..3)
                .map(|k| (ring.center[k] - center[k]) * axis[k])
                .sum::<f64>();
            Rim {
                radius: ring.radius_mm,
                axial,
                end: if ring.radius_mm == 0. {
                    End::Pole
                } else {
                    ends[id]
                },
            }
        })
    };
    let cone = |partial, inner, a: [f64; 3], b: [f64; 3], ra: f64, rb: f64| Support {
        family: ResultFaceFamily::Cone,
        partial,
        inner,
        radius: ra.min(rb),
        other_radius: ra.max(rb),
        meridian: None,
        height: a
            .iter()
            .zip(b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt(),
    };
    Some(match owner {
        Face::Cylinder { index } => {
            let c = mixed.cylinders.get(index)?;
            (
                index,
                single(
                    ResultFaceFamily::Cylinder,
                    false,
                    c.inner,
                    mixed.rings.get(c.rings[0])?.radius_mm,
                ),
            )
        }
        Face::PartialCylinder { index } => {
            let c = mixed.partial_cylinders.get(index)?;
            (
                index,
                single(
                    ResultFaceFamily::Cylinder,
                    true,
                    c.inner,
                    mixed.arcs.get(c.arcs[0])?.radius_mm,
                ),
            )
        }
        Face::Cone { index } => {
            let c = model.revolved.cones.get(index)?;
            let a = mixed.rings.get(c.rings[0])?;
            let b = mixed.rings.get(c.rings[1])?;
            (
                index,
                cone(false, c.inner, a.center, b.center, a.radius_mm, b.radius_mm),
            )
        }
        Face::PartialCone { index } => {
            let c = model.partial_cones.get(index)?;
            let a = mixed.arcs.get(c.arcs[0])?;
            let b = mixed.arcs.get(c.arcs[1])?;
            (
                index,
                cone(true, c.inner, a.center, b.center, a.radius_mm, b.radius_mm),
            )
        }
        Face::Sphere { index } => {
            let c = model.spheres.get(index)?;
            (
                index,
                Support {
                    meridian: c.rings.map(|ids| {
                        let ends = rims(ids, c.center);
                        let latitude = |end: Rim| end.axial.atan2(end.radius);
                        Meridian {
                            ends,
                            length: c.radius_mm * (latitude(ends[1]) - latitude(ends[0])).abs(),
                        }
                    }),
                    ..single(
                        ResultFaceFamily::Sphere,
                        c.rings.is_some(),
                        c.inner,
                        c.radius_mm,
                    )
                },
            )
        }
        Face::Torus { index } => {
            let c = model.tori.get(index)?;
            (
                index,
                Support {
                    other_radius: c.major_radius_mm,
                    meridian: c.rings.map(|ids| Meridian {
                        ends: rims(ids, c.center),
                        length: c.minor_radius_mm * (c.meridian_rad[1] - c.meridian_rad[0]).abs(),
                    }),
                    ..single(
                        ResultFaceFamily::Torus,
                        c.rings.is_some(),
                        c.inner,
                        c.minor_radius_mm,
                    )
                },
            )
        }
        _ => return None,
    })
}

struct Proposal {
    display: SimilarCandidate,
    hint: GuidanceReport,
}

struct Search {
    proposals: Vec<Proposal>,
    capacity: usize,
    truncated: bool,
    work: usize,
}

impl Search {
    fn spend(&mut self, n: usize) -> Result<(), FabError> {
        self.work = self.work.checked_sub(n).ok_or_else(limit)?;
        Ok(())
    }

    fn record(
        &mut self,
        support: Support,
        body: &AnalyzedBody,
        source: &BodySourceTriangles,
        region: &[usize],
        faces: Vec<usize>,
    ) {
        if self.proposals.len() == self.capacity {
            self.truncated = true;
            return;
        }
        let ids: Vec<_> = region
            .iter()
            .map(|&t| source.analysis_to_source_triangles[t])
            .collect();
        let family = support.family_name();
        self.proposals.push(Proposal {
            display: SimilarCandidate {
                id: self.proposals.len(),
                family,
                face_ids: faces.clone(),
                radius_mm: (support.family != ResultFaceFamily::Plane).then_some(support.radius),
                semi_angle_rad: (support.family == ResultFaceFamily::Cone)
                    .then(|| support.slope().atan()),
            },
            hint: GuidanceReport {
                hint_id: 0,
                family,
                selected_face_ids: faces,
                preview_body_id: source.preview_body_id.clone(),
                analysis_body_id: body.id.clone(),
                selected_source_triangle_ids: ids,
                reconstructed_source_triangle_ids: Vec::new(),
            },
        });
    }
}

/// Retain only topology and provenance while recognising the source.
struct NativeFaces {
    faces: Vec<fabstep_cad::ResultFace>,
    source: Vec<ResultFaceSource>,
}

struct BodyFaces {
    // Indexed by analysis triangle, values are exact current STEP face IDs.
    native: Vec<usize>,
    claimed: Vec<bool>,
}
impl BodyFaces {
    fn new(
        body: &AnalyzedBody,
        mapping: &[usize],
        rows: &[&ResultFaceSource],
        hints: &[GuidanceReport],
    ) -> Result<Self, FabError> {
        let mut native = vec![usize::MAX; mapping.len()];
        for row in rows {
            for t in &row.source_triangle_ids {
                let i = mapping.binary_search(t).map_err(|_| unavailable())?;
                if native[i] != usize::MAX {
                    return Err(unavailable());
                }
                native[i] = row.face_id;
            }
        }
        if native.contains(&usize::MAX) {
            return Err(unavailable());
        }
        let mut claimed = vec![false; mapping.len()];
        for hint in hints.iter().filter(|hint| hint.analysis_body_id == body.id) {
            for t in &hint.reconstructed_source_triangle_ids {
                claimed[mapping.binary_search(t).map_err(|_| unavailable())?] = true;
            }
        }
        Ok(Self { native, claimed })
    }

    fn whole_faces(&self, region: &[usize], inspected: &NativeFaces) -> Option<Vec<usize>> {
        if region.iter().any(|&i| self.claimed[i]) {
            return None;
        }
        let faces: BTreeSet<_> = region.iter().map(|&i| self.native[i]).collect();
        let count = faces
            .iter()
            .map(|&f| inspected.source[f].source_triangle_ids.len())
            .sum::<usize>();
        (count == region.len()).then(|| faces.into_iter().collect())
    }
}

impl DocumentStep {
    pub fn similar_regions_controlled(
        &self,
        checkpoint: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Result<SimilarRegions, FabError> {
        controlled(checkpoint, |control| {
            let search = self.find_similar(control)?;
            Ok(SimilarRegions {
                candidates: search.proposals.into_iter().map(|p| p.display).collect(),
                truncated: search.truncated,
            })
        })
    }

    pub fn apply_similar_controlled(
        &self,
        chosen: &[usize],
        checkpoint: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Result<ProvenancedDocumentConversion, FabError> {
        controlled(checkpoint, |control| {
            if chosen.is_empty() || chosen.len() > 16 {
                return Err(invalid());
            }
            let ids: BTreeSet<_> = chosen.iter().copied().collect();
            if ids.len() != chosen.len() {
                return Err(invalid());
            }
            let search = self.find_similar(control)?;
            if ids.last().is_none_or(|&i| i >= search.proposals.len()) {
                return Err(invalid());
            }
            let mut next = self.guidance_source().cloned().ok_or_else(unavailable)?;
            next.history.record(&next.hints);
            for p in search
                .proposals
                .into_iter()
                .filter(|p| ids.contains(&p.display.id))
            {
                if next.next_hint_id > 9_007_199_254_740_991 || next.hints.len() >= 16 {
                    return Err(limit());
                }
                next.hints.push(GuidanceReport {
                    hint_id: next.next_hint_id,
                    ..p.hint
                });
                next.next_hint_id += 1;
            }
            if next
                .hints
                .iter()
                .map(|h| h.selected_source_triangle_ids.len())
                .sum::<usize>()
                > 100_000
            {
                return Err(limit());
            }
            Self::rebuild_guidance(next, control)
        })
    }

    fn find_similar(&self, control: &mut ConversionControl<'_>) -> Result<Search, FabError> {
        control.check(ConversionPhase::ResultInspection, None)?;
        let source = self.guidance_source().ok_or_else(unavailable)?;
        let last = source.hints.last().ok_or_else(unavailable)?;
        let inspected = control.run(ConversionPhase::ResultInspection, None, || {
            let inspected = self.result_faces()?;
            if inspected.source_correspondence.status != "exact" {
                return Err(unavailable());
            }
            Ok(NativeFaces {
                faces: inspected.result.faces,
                source: inspected.source_correspondence.faces,
            })
        })?;
        let mut rows = BTreeMap::<&str, Vec<&ResultFaceSource>>::new();
        for row in &inspected.source {
            rows.entry(&row.analysis_body_id).or_default().push(row);
        }
        let (doc, mappings) = control.run(ConversionPhase::Analysis, None, || {
            analyze_document_with_provenance(&source.bytes, source.format, &source.options)
        })?;
        let first = doc
            .bodies
            .iter()
            .position(|b| b.id == last.analysis_body_id)
            .ok_or_else(unavailable)?;
        let mut reference = None;
        let mut plane_normal = None;
        let mut search = Search {
            proposals: Vec::new(),
            capacity: 16usize.saturating_sub(source.hints.len()),
            truncated: false,
            work: 5_000_000,
        };
        for i in std::iter::once(first).chain((0..doc.bodies.len()).filter(|&i| i != first)) {
            let body = &doc.bodies[i];
            let component = &mappings[i];
            let mapping = &component.analysis_to_source_triangles;
            let body_rows = rows.get(body.id.as_str()).ok_or_else(unavailable)?;
            let mut faces = BodyFaces::new(body, mapping, body_rows, &source.hints)?;
            if i == first {
                let triangle = mapping
                    .binary_search(
                        last.selected_source_triangle_ids
                            .first()
                            .ok_or_else(unavailable)?,
                    )
                    .map_err(|_| unavailable())?;
                let id = faces.native[triangle];
                if last.family == "plane"
                    || (last.family == "auto"
                        && inspected.faces[id].family == ResultFaceFamily::Plane)
                {
                    plane_normal = Some(self.planar_face_frame(id)?.normal);
                }
            }
            let r = control.run(ConversionPhase::Recognition, Some(i), || {
                fabstep_core::recognize_automatic(&body.analysis, &source.options)
            })?;
            if coupled(&r) {
                if i == first {
                    break;
                }
                continue;
            }
            if let Some(normal) = plane_normal {
                search.planes(
                    body,
                    component,
                    &inspected,
                    &mut faces,
                    normal,
                    &source.options,
                )?;
            } else {
                let ends = cylinder_ends(&r);
                if i == first {
                    // An accepted torus/chain may also own its neighbouring support.
                    reference = last
                        .selected_source_triangle_ids
                        .iter()
                        .chain(&last.reconstructed_source_triangle_ids)
                        .filter_map(|s| mapping.binary_search(s).ok())
                        .filter_map(|t| support(&r, &ends, r.triangle_faces[t]))
                        .map(|(_, s)| s)
                        .find(|s| last.family == "auto" || s.family_name() == last.family);
                    if reference.is_none() {
                        return Err(unavailable());
                    }
                }
                let template = reference.ok_or_else(unavailable)?;
                let mut seen = BTreeSet::new();
                for (seed, &owner) in r.triangle_faces.iter().enumerate() {
                    let Some((index, s)) = support(&r, &ends, owner) else {
                        continue;
                    };
                    if faces.claimed[seed]
                        || !s.matches(template, source.options.cylinder_fit_tolerance_mm)
                        || inspected.faces[faces.native[seed]].family == s.family
                        || !seen.insert((s.partial, index))
                    {
                        continue;
                    }
                    search.spend(r.triangle_faces.len())?;
                    let region =
                        match fabstep_core::guided_rounded_region(&r, s.guidance(), &[seed]) {
                            Ok(region) => region,
                            Err(e) if e.code == "guidance.family_unproven" => continue,
                            Err(e) => return Err(e),
                        };
                    let Some(ids) = faces.whole_faces(&region, &inspected) else {
                        continue;
                    };
                    search.record(s, body, component, &region, ids);
                    for t in region {
                        faces.claimed[t] = true;
                    }
                    if search.truncated {
                        break;
                    }
                }
            }
            control.check(ConversionPhase::Recognition, Some(i))?;
            if search.truncated {
                break;
            }
        }
        control.check(ConversionPhase::Finish, None)?;
        Ok(search)
    }
}

fn parallel_within(a: [f64; 3], b: [f64; 3], span: f64, tolerance: f64) -> bool {
    let sine = (a[1] * b[2] - a[2] * b[1])
        .hypot(a[2] * b[0] - a[0] * b[2])
        .hypot(a[0] * b[1] - a[1] * b[0]);
    sine * span <= tolerance
}

impl Search {
    /// Find connected fragments within a slab parallel to the accepted plane.
    /// Only whole unclaimed STEP faces enter the ordinary guided plane proof.
    fn planes(
        &mut self,
        body: &AnalyzedBody,
        component: &BodySourceTriangles,
        inspected: &NativeFaces,
        faces: &mut BodyFaces,
        normal: [f64; 3],
        options: &Options,
    ) -> Result<(), FabError> {
        let mut parts: BTreeMap<usize, (Vec<usize>, f64, f64)> = BTreeMap::new();
        let mesh = &body.analysis.mesh;
        for (i, &triangle) in mesh.triangles.iter().enumerate() {
            let face = faces.native[i];
            if inspected.faces[face].family != ResultFaceFamily::Plane {
                continue;
            }
            let part = parts
                .entry(face)
                .or_insert_with(|| (Vec::new(), f64::INFINITY, f64::NEG_INFINITY));
            part.0.push(i);
            for v in triangle {
                let p = mesh.vertices[v];
                let h = p[0] * normal[0] + p[1] * normal[1] + p[2] * normal[2];
                part.1 = part.1.min(h);
                part.2 = part.2.max(h);
            }
        }
        let tolerance = options.plane_tolerance_mm;
        let mut seen = BTreeSet::new();
        for (&id, part) in &parts {
            if part.2 - part.1 > 2. * tolerance || seen.contains(&id) {
                continue;
            }
            let height = (part.1 + part.2) * 0.5;
            let mut region = Vec::new();
            let mut pending = vec![id];
            while let Some(face) = pending.pop() {
                self.spend(1)?;
                let Some((triangles, low, high)) = parts.get(&face) else {
                    continue;
                };
                if height - low > tolerance || high - height > tolerance || !seen.insert(face) {
                    continue;
                }
                region.extend_from_slice(triangles);
                pending.extend(&inspected.faces[face].adjacent_face_ids);
            }
            let Some(ids) = faces.whole_faces(&region, inspected) else {
                continue;
            };
            if ids.len() < 2 {
                continue;
            }
            self.spend(mesh.triangles.len())?;
            region.sort_unstable();
            let recognized = match fabstep_core::recognize_guided_surfaces(
                &body.analysis,
                options,
                &[(GuidanceFamily::Plane, region.clone())],
            ) {
                Ok(value) => value,
                Err(e) if e.code.starts_with("guidance.plane_") => continue,
                Err(e) => return Err(e),
            };
            let fabstep_core::RevolvedFaceRef::Plane { index } =
                recognized.triangle_faces[region[0]]
            else {
                continue;
            };
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for &i in &region {
                for v in mesh.triangles[i] {
                    for k in 0..3 {
                        min[k] = min[k].min(mesh.vertices[v][k]);
                        max[k] = max[k].max(mesh.vertices[v][k]);
                    }
                }
            }
            let span = (0..3)
                .map(|k| (max[k] - min[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            if !parallel_within(
                normal,
                recognized.model.mixed.planes[index].normal,
                span,
                tolerance,
            ) {
                continue;
            }
            let plane = Support {
                family: ResultFaceFamily::Plane,
                partial: false,
                inner: false,
                radius: 0.,
                other_radius: 0.,
                height: 1.,
                meridian: None,
            };
            self.record(plane, body, component, &region, ids);
            if self.truncated {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
