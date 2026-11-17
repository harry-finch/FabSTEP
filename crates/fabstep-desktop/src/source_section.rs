//! Preparation epochs and immutable trusted source meshes, independent of STEP.
use super::*;
use fabstep_app::{PreparedSourceSection, SourceSectionMetadata};

#[derive(Clone)]
pub(super) struct SourcePreparationContext {
    request_id: u64,
    source_id: u64,
    preparation_id: u64,
    epoch: u64,
    bytes: Arc<Vec<u8>>,
    format: InputFormat,
    options: Options,
}

pub(super) struct CachedSourceSection {
    context: SourcePreparationContext,
    prepared: Arc<PreparedSourceSection>,
}

type SourcePreparationStart = (SourcePreparationContext, Option<Arc<PreparedSourceSection>>);

fn same_options(a: &Options, b: &Options) -> bool {
    a.input_unit == b.input_unit
        && a.reconstruction == b.reconstruction
        && a.weld_tolerance_mm.to_bits() == b.weld_tolerance_mm.to_bits()
        && a.plane_tolerance_mm.to_bits() == b.plane_tolerance_mm.to_bits()
        && a.cylinder_fit_tolerance_mm.to_bits() == b.cylinder_fit_tolerance_mm.to_bits()
        && a.max_deviation_mm.to_bits() == b.max_deviation_mm.to_bits()
}

impl CacheState {
    pub(super) fn begin_source_preparation(
        &mut self,
        request_id: u64,
        source_id: u64,
        preparation_id: u64,
        options: Option<Options>,
    ) -> Result<Option<SourcePreparationStart>, FabError> {
        if preparation_id == 0 || preparation_id <= self.source_section_generation {
            return Err(stale());
        }
        self.source_section_generation = preparation_id;
        let previous = self.source_section.take();
        if self
            .section
            .as_ref()
            .is_some_and(|context| context.is_source())
        {
            self.section = None;
        }
        let Some(options) = options else {
            return Ok(None);
        };
        let (bytes, format, epoch) = self.selection_snapshot(request_id, source_id)?;
        let reused = previous
            .filter(|cache| {
                cache.context.request_id == request_id
                    && cache.context.source_id == source_id
                    && cache.context.epoch == epoch
                    && cache.context.format == format
                    && Arc::ptr_eq(&cache.context.bytes, &bytes)
                    && same_options(&cache.context.options, &options)
            })
            .map(|cache| cache.prepared);
        Ok(Some((
            SourcePreparationContext {
                request_id,
                source_id,
                preparation_id,
                epoch,
                bytes,
                format,
                options,
            },
            reused,
        )))
    }

    pub(super) fn current_source_preparation(&self, context: &SourcePreparationContext) -> bool {
        self.source_section_generation == context.preparation_id
            && self.current(context.request_id, context.source_id, context.epoch)
            && self
                .source
                .as_ref()
                .is_some_and(|source| Arc::ptr_eq(&source.bytes, &context.bytes))
    }

    pub(super) fn finish_source_preparation(
        &mut self,
        context: SourcePreparationContext,
        prepared: Arc<PreparedSourceSection>,
    ) -> Result<(), FabError> {
        if !self.current_source_preparation(&context) {
            return Err(stale());
        }
        self.source_section = Some(CachedSourceSection { context, prepared });
        Ok(())
    }

    pub(super) fn source_section_snapshot(
        &self,
        request_id: u64,
        source_id: u64,
        preparation_id: u64,
    ) -> Result<(Arc<PreparedSourceSection>, u64), FabError> {
        let cache = self
            .source_section
            .as_ref()
            .filter(|cache| {
                cache.context.request_id == request_id
                    && cache.context.source_id == source_id
                    && cache.context.preparation_id == preparation_id
                    && self.current_source_preparation(&cache.context)
            })
            .ok_or_else(stale)?;
        Ok((Arc::clone(&cache.prepared), cache.context.epoch))
    }

    pub(super) fn current_source_section(
        &self,
        request_id: u64,
        source_id: u64,
        epoch: u64,
        preparation_id: u64,
        prepared: &Arc<PreparedSourceSection>,
    ) -> bool {
        self.source_section_snapshot(request_id, source_id, preparation_id)
            .is_ok_and(|(current, current_epoch)| {
                current_epoch == epoch && Arc::ptr_eq(&current, prepared)
            })
    }
}

#[derive(Serialize)]
pub(crate) struct SourcePreparationResponse {
    request_id: u64,
    source_id: u64,
    preparation_id: u64,
    metadata: Option<SourceSectionMetadata>,
}

#[tauri::command]
pub(crate) async fn prepare_source_section(
    request_id: u64,
    source_id: u64,
    preparation_id: u64,
    options: Option<Options>,
    state: State<'_, Arc<EngineState>>,
) -> Result<SourcePreparationResponse, FabError> {
    let state = Arc::clone(state.inner());
    let started = state
        .inner
        .lock()
        .map_err(lock_error)?
        .begin_source_preparation(request_id, source_id, preparation_id, options)?;
    let Some((context, reused)) = started else {
        return Ok(SourcePreparationResponse {
            request_id,
            source_id,
            preparation_id,
            metadata: None,
        });
    };
    let worker_context = context.clone();
    let worker_state = Arc::clone(&state);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        let check = || {
            if worker_state
                .inner
                .lock()
                .map_err(lock_error)?
                .current_source_preparation(&worker_context)
            {
                Ok(())
            } else {
                Err(stale())
            }
        };
        check()?;
        if let Some(prepared) = reused {
            return Ok(prepared);
        }
        fabstep_app::prepare_source_section_controlled(
            &worker_context.bytes,
            worker_context.format,
            &worker_context.options,
            &mut |_| check(),
        )
        .map(Arc::new)
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current_source_preparation(&context) {
        return Err(stale());
    }
    let prepared = computed.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    let metadata = prepared.metadata().clone();
    cache.finish_source_preparation(context, prepared)?;
    Ok(SourcePreparationResponse {
        request_id,
        source_id,
        preparation_id,
        metadata: Some(metadata),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn installed() -> CacheState {
        let mut cache = CacheState::default();
        cache.expect_upload(400).unwrap();
        cache.source = Some(CachedSource {
            source_id: 400,
            format: InputFormat::Stl,
            bytes: Arc::new(include_bytes!("../../../fixtures/cube.stl").to_vec()),
        });
        cache
            .finish(400, 400, cache.epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        cache
    }
    fn prepare(context: &SourcePreparationContext) -> Arc<PreparedSourceSection> {
        Arc::new(
            fabstep_app::prepare_source_section(&context.bytes, context.format, &context.options)
                .unwrap(),
        )
    }
    #[test]
    fn source_preparation_is_read_only_and_reuses_only_exact_source_options() {
        let mut cache = installed();
        let step = Arc::clone(&cache.result.as_ref().unwrap().step);
        let (first, reused) = cache
            .begin_source_preparation(400, 400, 1, Some(Options::default()))
            .unwrap()
            .unwrap();
        assert!(reused.is_none());
        let prepared = prepare(&first);
        cache
            .finish_source_preparation(first.clone(), Arc::clone(&prepared))
            .unwrap();
        assert!(Arc::ptr_eq(&step, &cache.result.as_ref().unwrap().step));
        let (second, reused) = cache
            .begin_source_preparation(400, 400, 2, Some(Options::default()))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(reused.as_ref().unwrap(), &prepared));
        assert!(!cache.current_source_preparation(&first));
        assert!(cache
            .finish_source_preparation(first, Arc::clone(&prepared))
            .is_err());
        cache
            .finish_source_preparation(second, Arc::clone(&prepared))
            .unwrap();
        let (_, reused) = cache
            .begin_source_preparation(
                400,
                400,
                3,
                Some(Options {
                    input_unit: fabstep_core::InputUnit::Inch,
                    ..Options::default()
                }),
            )
            .unwrap()
            .unwrap();
        assert!(reused.is_none());
        assert!(Arc::ptr_eq(&step, &cache.result.as_ref().unwrap().step));
    }
    #[test]
    fn preparation_cancel_and_revision_reject_late_success_without_removing_step() {
        let mut cache = installed();
        let (first, _) = cache
            .begin_source_preparation(400, 400, 1, Some(Options::default()))
            .unwrap()
            .unwrap();
        let prepared = prepare(&first);
        cache.begin_source_preparation(0, 0, 3, None).unwrap();
        assert!(cache
            .begin_source_preparation(400, 400, 2, Some(Options::default()))
            .is_err());
        assert!(cache.finish_source_preparation(first, prepared).is_err());
        assert_eq!(cache.result.as_ref().unwrap().step.bytes(), &[9]);
        let (next, _) = cache
            .begin_source_preparation(400, 400, 4, Some(Options::default()))
            .unwrap()
            .unwrap();
        cache.invalidate(401, Some(400)).unwrap();
        assert!(!cache.current_source_preparation(&next));
        assert!(cache.source_section.is_none());
    }
    #[test]
    fn source_preparation_checks_every_option_field_exactly() {
        let base = Options::default();
        for changed in [
            Options {
                input_unit: fabstep_core::InputUnit::Inch,
                ..base.clone()
            },
            Options {
                reconstruction: fabstep_core::ReconstructionMode::Cylinders,
                ..base.clone()
            },
            Options {
                weld_tolerance_mm: base.weld_tolerance_mm * 2.,
                ..base.clone()
            },
            Options {
                plane_tolerance_mm: base.plane_tolerance_mm * 2.,
                ..base.clone()
            },
            Options {
                cylinder_fit_tolerance_mm: base.cylinder_fit_tolerance_mm * 2.,
                ..base.clone()
            },
            Options {
                max_deviation_mm: base.max_deviation_mm * 2.,
                ..base.clone()
            },
        ] {
            assert!(!same_options(&base, &changed));
        }
        assert!(same_options(&base, &base));
    }
}
