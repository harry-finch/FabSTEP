//! One read-only snapshot of an actual conversion checkpoint, never a counter.
use super::{lock_error, stale, CacheState, EngineState};
use fabstep_app::{ConversionCheckpoint, ConversionPhase, InspectedDocumentStep};
use fabstep_core::FabError;
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct ConversionProgress {
    request_id: u64,
    source_id: u64,
    phase: &'static str,
    /// Analysis document index, starting at zero, exactly as reported by app.
    body_index: Option<usize>,
    /// The current checkpoint API does not report the document's body count.
    body_count: Option<usize>,
}

pub(super) struct ActiveProgress {
    request_id: u64,
    source_id: u64,
    epoch: u64,
    generation: u64,
    finished: bool,
    body_started: bool,
    snapshot: Option<ConversionProgress>,
    queued_display: Option<Arc<InspectedDocumentStep>>,
}

fn phase_name(phase: ConversionPhase) -> &'static str {
    match phase {
        ConversionPhase::Analysis => "analysis",
        ConversionPhase::SourceSeparation => "source_separation",
        ConversionPhase::Body => "body",
        ConversionPhase::Recognition => "recognition",
        ConversionPhase::SourceValidation => "source_validation",
        ConversionPhase::Build => "build",
        ConversionPhase::AuditTessellation => "audit_tessellation",
        ConversionPhase::AuditComparison => "audit_comparison",
        ConversionPhase::Combine => "combine",
        ConversionPhase::ResultInspection => "result_inspection",
        ConversionPhase::Finish => "finish",
    }
}

impl CacheState {
    pub(super) fn conversion_progress(
        &self,
        request_id: u64,
        source_id: u64,
    ) -> Option<ConversionProgress> {
        let active = self.progress.as_ref()?;
        if active.request_id != request_id
            || active.source_id != source_id
            || !self.current(request_id, source_id, active.epoch)
        {
            return None;
        }
        active.snapshot.clone()
    }
}

/// The command owns this guard until publication or error, including a failed
/// blocking task. Its observer cannot publish after this guard has been dropped.
pub(super) struct ConversionGuard {
    observer: ProgressObserver,
}

#[derive(Clone)]
pub(super) struct ProgressObserver {
    state: Arc<EngineState>,
    request_id: u64,
    source_id: u64,
    epoch: u64,
    generation: u64,
}

impl ConversionGuard {
    pub(super) fn new(
        state: Arc<EngineState>,
        request_id: u64,
        source_id: u64,
        epoch: u64,
    ) -> Result<Self, FabError> {
        Self::with_queued_display(state, request_id, source_id, epoch, None)
    }

    pub(super) fn with_queued_display(
        state: Arc<EngineState>,
        request_id: u64,
        source_id: u64,
        epoch: u64,
        queued_display: Option<Arc<InspectedDocumentStep>>,
    ) -> Result<Self, FabError> {
        let generation = {
            let mut cache = state.inner.lock().map_err(lock_error)?;
            if !cache.current(request_id, source_id, epoch) {
                return Err(stale());
            }
            cache.progress_generation = cache.progress_generation.wrapping_add(1);
            let generation = cache.progress_generation;
            cache.progress = Some(ActiveProgress {
                request_id,
                source_id,
                epoch,
                generation,
                finished: false,
                body_started: false,
                snapshot: None,
                queued_display,
            });
            generation
        };
        Ok(Self {
            observer: ProgressObserver {
                state,
                request_id,
                source_id,
                epoch,
                generation,
            },
        })
    }

    /// File replacement and guard cancellation both retire the queued inspection.
    pub(super) fn take_queued_display(&self) -> Result<Arc<InspectedDocumentStep>, FabError> {
        let observer = &self.observer;
        let mut cache = observer.state.inner.lock().map_err(lock_error)?;
        cache
            .progress
            .as_mut()
            .filter(|p| p.generation == observer.generation)
            .and_then(|p| p.queued_display.take())
            .ok_or_else(stale)
    }

    pub(super) fn observer(&self) -> ProgressObserver {
        self.observer.clone()
    }
}

impl ProgressObserver {
    pub(super) fn checkpoint(&self, checkpoint: ConversionCheckpoint) -> Result<(), FabError> {
        let mut cache = self.state.inner.lock().map_err(lock_error)?;
        if !cache.current(self.request_id, self.source_id, self.epoch) {
            return Err(stale());
        }
        let active = cache
            .progress
            .as_mut()
            .filter(|active| active.generation == self.generation)
            .ok_or_else(stale)?;
        if checkpoint.body_index.is_some() {
            active.body_started = true;
        }
        if checkpoint.phase == ConversionPhase::Finish {
            active.snapshot = None;
            active.finished = true;
        } else if !active.finished {
            active.snapshot = Some(ConversionProgress {
                request_id: self.request_id,
                source_id: self.source_id,
                // Guided conversions inspect the previous STEP before analysis.
                // That preflight must not claim the new reconstruction is done.
                phase: if checkpoint.phase == ConversionPhase::ResultInspection
                    && !active.body_started
                {
                    "guidance_inspection"
                } else {
                    phase_name(checkpoint.phase)
                },
                body_index: checkpoint.body_index,
                body_count: None,
            });
        }
        Ok(())
    }
}

impl Drop for ConversionGuard {
    fn drop(&mut self) {
        if let Ok(mut cache) = self.observer.state.inner.lock() {
            if cache
                .progress
                .as_ref()
                .is_some_and(|active| active.generation == self.observer.generation)
            {
                cache.progress = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CachedSource;
    use fabstep_app::{DocumentStep, InputFormat};

    fn source(id: u64) -> (Arc<EngineState>, u64) {
        let state = Arc::new(EngineState::default());
        let epoch = {
            let mut cache = state.inner.lock().unwrap();
            cache.expect_upload(id).unwrap();
            cache.source = Some(CachedSource {
                source_id: id,
                format: InputFormat::Stl,
                bytes: Arc::new(vec![1, 2, 3]),
            });
            cache.begin(id, id).unwrap().2
        };
        (state, epoch)
    }

    fn snapshot(state: &EngineState, request: u64, source: u64) -> Option<ConversionProgress> {
        state
            .inner
            .lock()
            .unwrap()
            .conversion_progress(request, source)
    }

    fn check(observer: &ProgressObserver, phase: ConversionPhase, body_index: Option<usize>) {
        observer
            .checkpoint(ConversionCheckpoint { phase, body_index })
            .unwrap();
    }

    #[test]
    fn guided_preflight_is_not_final_validation() {
        let (state, epoch) = source(60);
        let guard = ConversionGuard::new(Arc::clone(&state), 60, 60, epoch).unwrap();
        let observer = guard.observer();
        check(&observer, ConversionPhase::ResultInspection, None);
        assert_eq!(
            snapshot(&state, 60, 60).unwrap().phase,
            "guidance_inspection"
        );
        check(&observer, ConversionPhase::Analysis, None);
        check(&observer, ConversionPhase::Body, Some(0));
        check(&observer, ConversionPhase::ResultInspection, None);
        assert_eq!(snapshot(&state, 60, 60).unwrap().phase, "result_inspection");
    }

    #[test]
    fn actual_phases_replace_one_snapshot_and_repeated_callbacks_are_not_counts() {
        let (state, epoch) = source(10);
        let guard = ConversionGuard::new(Arc::clone(&state), 10, 10, epoch).unwrap();
        let observer = guard.observer();
        assert!(snapshot(&state, 10, 10).is_none());
        for (phase, name) in [
            (ConversionPhase::Analysis, "analysis"),
            (ConversionPhase::SourceSeparation, "source_separation"),
            (ConversionPhase::Body, "body"),
            (ConversionPhase::Recognition, "recognition"),
            (ConversionPhase::SourceValidation, "source_validation"),
            (ConversionPhase::Build, "build"),
            (ConversionPhase::AuditTessellation, "audit_tessellation"),
            (ConversionPhase::AuditComparison, "audit_comparison"),
            (ConversionPhase::Combine, "combine"),
            (ConversionPhase::ResultInspection, "result_inspection"),
        ] {
            for _ in 0..3 {
                check(&observer, phase, Some(2));
                let current = snapshot(&state, 10, 10).unwrap();
                assert_eq!(current.phase, name);
                assert_eq!(current.body_index, Some(2));
                assert_eq!(current.body_count, None);
            }
        }
        check(&observer, ConversionPhase::Combine, None);
        assert_eq!(snapshot(&state, 10, 10).unwrap().body_index, None);
        for (request, source) in [(9, 10), (11, 10), (10, 9), (10, 11)] {
            assert!(snapshot(&state, request, source).is_none());
        }
        assert_eq!(snapshot(&state, 10, 10).unwrap().phase, "combine");
        check(&observer, ConversionPhase::Finish, None);
        assert!(snapshot(&state, 10, 10).is_none());
        check(&observer, ConversionPhase::Finish, None);
        assert!(snapshot(&state, 10, 10).is_none());
    }

    #[test]
    fn invalidation_source_replacement_and_upload_conflict_clear_progress() {
        for replacement in [false, true] {
            let (state, epoch) = source(20);
            let guard = ConversionGuard::new(Arc::clone(&state), 20, 20, epoch).unwrap();
            let observer = guard.observer();
            check(&observer, ConversionPhase::Build, Some(0));
            state
                .inner
                .lock()
                .unwrap()
                .invalidate(21, Some(if replacement { 21 } else { 20 }))
                .unwrap();
            assert!(snapshot(&state, 20, 20).is_none());
            assert!(state.inner.lock().unwrap().progress.is_none());
            assert_eq!(
                observer
                    .checkpoint(ConversionCheckpoint {
                        phase: ConversionPhase::Build,
                        body_index: Some(0)
                    })
                    .unwrap_err()
                    .code,
                "engine.stale"
            );
        }
        let (state, epoch) = source(30);
        let guard = ConversionGuard::new(Arc::clone(&state), 30, 30, epoch).unwrap();
        let observer = guard.observer();
        check(&observer, ConversionPhase::Recognition, Some(0));
        state
            .inner
            .lock()
            .unwrap()
            .resolve_duplicate(30, false)
            .unwrap_err();
        assert!(snapshot(&state, 30, 30).is_none());
        assert_eq!(
            observer
                .checkpoint(ConversionCheckpoint {
                    phase: ConversionPhase::Build,
                    body_index: None
                })
                .unwrap_err()
                .code,
            "engine.stale"
        );
    }

    #[test]
    fn finishing_or_exiting_with_an_error_removes_the_snapshot() {
        let (state, epoch) = source(40);
        {
            let guard = ConversionGuard::new(Arc::clone(&state), 40, 40, epoch).unwrap();
            check(&guard.observer(), ConversionPhase::AuditComparison, Some(0));
            state
                .inner
                .lock()
                .unwrap()
                .finish(40, 40, epoch, DocumentStep::untracked(vec![9]))
                .unwrap();
            drop(guard);
            assert!(snapshot(&state, 40, 40).is_none());
        }
        state.inner.lock().unwrap().begin(40, 40).unwrap();
        let failed = || -> Result<(), FabError> {
            let guard = ConversionGuard::new(Arc::clone(&state), 40, 40, epoch)?;
            check(
                &guard.observer(),
                ConversionPhase::SourceValidation,
                Some(0),
            );
            Err(FabError::new("test.refused", "refused"))
        };
        assert!(failed().is_err());
        assert!(snapshot(&state, 40, 40).is_none());
    }

    #[test]
    fn an_old_guard_cannot_publish_or_clear_a_new_operation_at_the_same_revision() {
        let (state, epoch) = source(50);
        let old = ConversionGuard::new(Arc::clone(&state), 50, 50, epoch).unwrap();
        let old_observer = old.observer();
        check(&old_observer, ConversionPhase::Recognition, Some(0));
        state.inner.lock().unwrap().begin(50, 50).unwrap();
        let current = ConversionGuard::new(Arc::clone(&state), 50, 50, epoch).unwrap();
        check(&current.observer(), ConversionPhase::Build, Some(1));
        assert_eq!(
            old_observer
                .checkpoint(ConversionCheckpoint {
                    phase: ConversionPhase::Finish,
                    body_index: None
                })
                .unwrap_err()
                .code,
            "engine.stale"
        );
        drop(old);
        assert_eq!(snapshot(&state, 50, 50).unwrap().phase, "build");
        drop(current);
        assert!(snapshot(&state, 50, 50).is_none());
    }

    #[test]
    fn a_panicking_worker_cannot_leave_a_pollable_snapshot() {
        let (state, epoch) = source(60);
        let guard = ConversionGuard::new(Arc::clone(&state), 60, 60, epoch).unwrap();
        let observer = guard.observer();
        let worker = std::panic::catch_unwind(move || {
            let _guard = guard;
            check(&observer, ConversionPhase::Build, Some(0));
            panic!("simulated worker failure");
        });
        assert!(worker.is_err());
        assert!(snapshot(&state, 60, 60).is_none());
    }
}
