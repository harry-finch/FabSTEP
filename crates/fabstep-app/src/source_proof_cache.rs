//! Successful exact-source proofs, private to one parameter search.
//!
//! No analysis, recognition, STEP or native object is retained. Mesh keys use
//! coordinate bits and ordered connectivity, never a tolerance or a digest.
use fabstep_core::{FabError, Mesh};
use fabstep_import::{MAX_BODIES, MAX_TRIANGLES};

const MAX_VERTICES: usize = 3 * MAX_TRIANGLES;

struct BodyProof {
    mesh: Mesh,
    validated: bool,
}

#[derive(Default)]
pub(crate) struct SourceProofCache {
    bodies: Vec<Option<BodyProof>>,
    separated: bool,
    #[cfg(test)]
    pub(crate) source_calls: usize,
    #[cfg(test)]
    pub(crate) separation_calls: usize,
}

fn exact_mesh(a: &Mesh, b: &Mesh) -> bool {
    a.triangles == b.triangles
        && a.vertices.len() == b.vertices.len()
        && a.vertices
            .iter()
            .zip(&b.vertices)
            .all(|(a, b)| a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits()))
}

fn bounded(meshes: impl Iterator<Item = (usize, usize)>) -> bool {
    let mut bodies = 0usize;
    let mut triangles = 0usize;
    let mut vertices = 0usize;
    for (t, v) in meshes {
        bodies += 1;
        triangles = triangles.saturating_add(t);
        vertices = vertices.saturating_add(v);
        if bodies > MAX_BODIES || triangles > MAX_TRIANGLES || vertices > MAX_VERTICES {
            return false;
        }
    }
    true
}

impl SourceProofCache {
    pub(crate) fn validate_source(
        &mut self,
        body_index: usize,
        mesh: &Mesh,
        prove: impl FnOnce() -> Result<(), FabError>,
    ) -> Result<(), FabError> {
        let previous = self.bodies.get(body_index).and_then(Option::as_ref);
        if previous.is_some_and(|body| body.validated && exact_mesh(&body.mesh, mesh)) {
            return Ok(());
        }
        #[cfg(test)]
        {
            self.source_calls += 1;
        }
        prove()?;
        // Bounds limit retained data only. A miss always follows the ordinary
        // native validation path, including its errors and limits.
        if body_index >= MAX_BODIES
            || !bounded(std::iter::once((mesh.triangles.len(), mesh.vertices.len())))
        {
            return Ok(());
        }
        if let Some(previous) = self.bodies.get_mut(body_index).and_then(Option::as_mut) {
            if exact_mesh(&previous.mesh, mesh) {
                previous.validated = true;
                return Ok(());
            }
        }
        if !bounded(
            self.bodies
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != body_index)
                .filter_map(|(_, b)| b.as_ref())
                .map(|b| (b.mesh.triangles.len(), b.mesh.vertices.len()))
                .chain(std::iter::once((mesh.triangles.len(), mesh.vertices.len()))),
        ) {
            self.bodies.clear();
        }
        self.separated = false;
        if self.bodies.len() <= body_index {
            self.bodies.resize_with(body_index + 1, || None);
        }
        self.bodies[body_index] = Some(BodyProof {
            mesh: mesh.clone(),
            validated: true,
        });
        Ok(())
    }

    pub(crate) fn validate_separation(
        &mut self,
        meshes: &[&Mesh],
        prove: impl FnOnce() -> Result<(), FabError>,
    ) -> Result<(), FabError> {
        if self.separated
            && self.bodies.len() == meshes.len()
            && self.bodies.iter().zip(meshes).all(|(body, mesh)| {
                body.as_ref()
                    .is_some_and(|body| exact_mesh(&body.mesh, mesh))
            })
        {
            return Ok(());
        }
        #[cfg(test)]
        {
            self.separation_calls += 1;
        }
        prove()?;
        if !bounded(meshes.iter().map(|m| (m.triangles.len(), m.vertices.len()))) {
            return Ok(());
        }
        // Reuse owned keys where possible; a separation proof alone does not
        // constitute the per-body solid-validity proof.
        let mut old = std::mem::take(&mut self.bodies).into_iter();
        self.bodies = meshes
            .iter()
            .map(|mesh| {
                Some(match old.next().flatten() {
                    Some(body) if exact_mesh(&body.mesh, mesh) => body,
                    _ => BodyProof {
                        mesh: (*mesh).clone(),
                        validated: false,
                    },
                })
            })
            .collect();
        self.separated = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh() -> Mesh {
        Mesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn only_successful_exact_ordered_source_proofs_are_reused() {
        let mut cache = SourceProofCache::default();
        let mesh = mesh();
        for _ in 0..2 {
            let error = cache.validate_source(0, &mesh, || {
                Err(FabError::new("test.invalid_source", "source refused"))
            });
            assert_eq!(error.unwrap_err().code, "test.invalid_source");
        }
        assert_eq!(cache.source_calls, 2);
        cache.validate_source(0, &mesh, || Ok(())).unwrap();
        cache
            .validate_source(0, &mesh, || panic!("successful proof should be reused"))
            .unwrap();
        assert_eq!(cache.source_calls, 3);
        let mut changes = Vec::new();
        let mut moved = mesh.clone();
        moved.vertices[0][0] = f64::from_bits(1);
        changes.push(moved);
        let mut signed_zero = mesh.clone();
        signed_zero.vertices[0][0] = -0.0;
        changes.push(signed_zero);
        let mut winding = mesh.clone();
        winding.triangles[0].swap(1, 2);
        changes.push(winding);
        let mut reindexed = mesh.clone();
        reindexed.vertices.swap(0, 1);
        reindexed.triangles[0] = [1, 0, 2];
        changes.push(reindexed);
        for changed in changes {
            cache.validate_source(0, &changed, || Ok(())).unwrap();
        }
        assert_eq!(cache.source_calls, 7);
        let fresh = SourceProofCache::default();
        assert!(
            fresh.bodies.is_empty(),
            "a new search has no retained proof"
        );
    }

    #[test]
    fn separation_is_ordered_independent_and_failures_remain_retryable() {
        let mut cache = SourceProofCache::default();
        let a = mesh();
        let mut b = mesh();
        b.vertices[0][0] = 2.0;
        for _ in 0..2 {
            assert!(cache
                .validate_separation(&[&a, &b], || {
                    Err(FabError::new("test.contact", "contact"))
                })
                .is_err());
        }
        assert!(cache.bodies.is_empty());
        cache.validate_separation(&[&a, &b], || Ok(())).unwrap();
        cache
            .validate_separation(&[&a, &b], || panic!("separation hit"))
            .unwrap();
        cache.validate_source(0, &a, || Ok(())).unwrap();
        assert_eq!(cache.source_calls, 1, "separation is no solid proof");
        cache
            .validate_separation(&[&a, &b], || panic!("validity preserves separation"))
            .unwrap();
        cache.validate_separation(&[&b, &a], || Ok(())).unwrap();
        cache.validate_source(0, &b, || Ok(())).unwrap();
        assert_eq!(cache.source_calls, 2);
        assert_eq!(cache.separation_calls, 4);
        cache.validate_source(0, &a, || Ok(())).unwrap();
        cache.validate_separation(&[&a, &b], || Ok(())).unwrap();
        assert_eq!(cache.separation_calls, 5);
    }

    #[test]
    fn retained_keys_obey_document_bounds_even_when_geometry_changes() {
        let mut cache = SourceProofCache::default();
        let mut large = mesh();
        large.triangles.resize(MAX_TRIANGLES, [0, 1, 2]);
        cache.validate_source(0, &large, || Ok(())).unwrap();
        let small = mesh();
        cache.validate_source(1, &small, || Ok(())).unwrap();
        assert!(cache.bodies[0].is_none(), "evict before exceeding the cap");
        assert_eq!(cache.bodies[1].as_ref().unwrap().mesh.triangles.len(), 1);
        large.triangles.push([0, 1, 2]);
        for _ in 0..2 {
            cache.validate_source(0, &large, || Ok(())).unwrap();
            cache
                .validate_source(MAX_BODIES, &small, || Ok(()))
                .unwrap();
        }
        assert_eq!(cache.source_calls, 6);
        assert_eq!(cache.bodies.len(), 2);
        cache
            .validate_separation(&vec![&small; MAX_BODIES + 1], || Ok(()))
            .unwrap();
        assert!(!cache.separated);
        assert_eq!(cache.bodies.len(), 2);
    }
}
