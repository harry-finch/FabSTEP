//! Indication history is owned by the same immutable source and settings as the STEP.
//! Each entry stores only the inputs to recognition. Face provenance is proved again.
use super::*;

const MAX_STATES: usize = 16;
// Counts selected source and historical STEP indices, across all saved states.
const MAX_INDICES: usize = 400_000;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct GuidanceHistoryStatus {
    pub undo: bool,
    pub redo: bool,
}

#[derive(Debug, Clone)]
struct Snapshot {
    hints: Arc<[GuidanceReport]>,
    indices: usize,
}

impl Snapshot {
    fn from_hints(hints: &[GuidanceReport]) -> Self {
        let indices = hints
            .iter()
            .map(|h| h.selected_source_triangle_ids.len() + h.selected_face_ids.len())
            .sum();
        let hints = hints
            .iter()
            .map(|h| GuidanceReport {
                hint_id: h.hint_id,
                family: h.family,
                preview_body_id: h.preview_body_id.clone(),
                analysis_body_id: h.analysis_body_id.clone(),
                selected_face_ids: h.selected_face_ids.clone(),
                selected_source_triangle_ids: h.selected_source_triangle_ids.clone(),
                reconstructed_source_triangle_ids: Vec::new(),
            })
            .collect();
        Self { hints, indices }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct GuidanceHistory {
    past: std::collections::VecDeque<Snapshot>,
    future: std::collections::VecDeque<Snapshot>,
}

impl GuidanceHistory {
    pub(super) fn status(&self) -> Option<GuidanceHistoryStatus> {
        let undo = !self.past.is_empty();
        let redo = !self.future.is_empty();
        (undo || redo).then_some(GuidanceHistoryStatus { undo, redo })
    }

    fn trim(&mut self) {
        let mut indices: usize = self
            .past
            .iter()
            .chain(&self.future)
            .map(|s| s.indices)
            .sum();
        while self.past.len() + self.future.len() > MAX_STATES || indices > MAX_INDICES {
            // Keep the nearest steps in either direction. On ties evict the oldest edit.
            let farthest = if self.past.len() >= self.future.len() {
                &mut self.past
            } else {
                &mut self.future
            };
            indices -= farthest
                .pop_front()
                .expect("history exceeds a positive bound")
                .indices;
        }
    }

    pub(super) fn record(&mut self, previous: &[GuidanceReport]) {
        self.future.clear();
        self.past.push_back(Snapshot::from_hints(previous));
        self.trim();
    }
}

impl DocumentStep {
    pub(super) fn session_source(&self) -> Result<&ConversionSource, FabError> {
        self.guidance_source().ok_or_else(|| {
            FabError::new(
                "guidance.provenance_unavailable",
                "La session guidée est indisponible.",
            )
        })
    }

    pub fn edit_guidance_controlled(
        &self,
        hint_id: u64,
        family: GuidanceFamily,
        checkpoint: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Result<ProvenancedDocumentConversion, FabError> {
        controlled(checkpoint, |control| {
            control.check(ConversionPhase::Analysis, None)?;
            let previous = self.session_source()?;
            let mut source = previous.clone();
            let hint = source
                .hints
                .iter_mut()
                .find(|hint| hint.hint_id == hint_id)
                .ok_or_else(unknown_hint)?;
            let family = family_name(family);
            if hint.family != family {
                hint.family = family;
                source.history.record(&previous.hints);
            }
            Self::rebuild_guidance(source, control)
        })
    }

    pub fn undo_guidance_controlled(
        &self,
        checkpoint: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Result<ProvenancedDocumentConversion, FabError> {
        self.restore_guidance(false, checkpoint)
    }

    pub fn redo_guidance_controlled(
        &self,
        checkpoint: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Result<ProvenancedDocumentConversion, FabError> {
        self.restore_guidance(true, checkpoint)
    }

    fn restore_guidance(
        &self,
        redo: bool,
        checkpoint: &mut dyn FnMut(ConversionCheckpoint) -> Result<(), FabError>,
    ) -> Result<ProvenancedDocumentConversion, FabError> {
        controlled(checkpoint, |control| {
            control.check(ConversionPhase::Analysis, None)?;
            let mut source = self.session_source()?.clone();
            let (from, to) = if redo {
                (&mut source.history.future, &mut source.history.past)
            } else {
                (&mut source.history.past, &mut source.history.future)
            };
            let target = from.pop_back().ok_or_else(|| {
                FabError::new(
                    "guidance.history_unavailable",
                    "Aucune modification d’indication disponible dans ce sens.",
                )
            })?;
            to.push_back(Snapshot::from_hints(&source.hints));
            source.hints = target.hints.to_vec();
            source.history.trim();
            Self::rebuild_guidance(source, control)
        })
    }
}

pub(super) fn unknown_hint() -> FabError {
    FabError::new(
        "guidance.unknown_indication",
        "Cette indication n’appartient pas à la session courante.",
    )
}

pub(super) fn family_name(family: GuidanceFamily) -> &'static str {
    match family {
        GuidanceFamily::Cylinder => "cylinder",
        GuidanceFamily::Cone => "cone",
        GuidanceFamily::Plane => "plane",
        GuidanceFamily::Auto => "auto",
        GuidanceFamily::Torus => "torus",
        GuidanceFamily::Sphere => "sphere",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_drop_oldest_states_and_never_retain_reconstructed_triangles() {
        let mut history = GuidanceHistory::default();
        let mut hint = GuidanceReport {
            hint_id: 0,
            family: "plane",
            selected_face_ids: vec![1],
            selected_source_triangle_ids: vec![2],
            reconstructed_source_triangle_ids: vec![2, 3, 4],
            preview_body_id: "stl".into(),
            analysis_body_id: "stl/0".into(),
        };
        for id in 0..=MAX_STATES {
            hint.hint_id = id as u64;
            history.record(std::slice::from_ref(&hint));
        }
        assert_eq!(history.past.len(), MAX_STATES);
        let first = &history.past[0].hints[0];
        assert_eq!(first.hint_id, 1);
        assert!(first.reconstructed_source_triangle_ids.is_empty());
        let mut large = GuidanceHistory::default();
        hint.selected_source_triangle_ids = (0..100_000).collect();
        for _ in 0..4 {
            large.record(std::slice::from_ref(&hint));
        }
        assert_eq!(large.past.len(), 3);
    }
}
