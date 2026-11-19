//! Read-only discovery and explicit, atomic application of similar walls.
use super::*;

#[derive(Serialize)]
pub(super) struct SimilarResponse {
    request_id: u64,
    source_id: u64,
    search_id: u64,
    regions: fabstep_app::SimilarRegions,
}

// Search uses considerable scratch space. Recheck the immutable snapshot after
// acquiring the permit, including when a new source replaced a queued request.
static SEARCH_WORKER: tauri::async_runtime::Mutex<()> = tauri::async_runtime::Mutex::const_new(());

impl CacheState {
    fn check_similar_selection(&self, search_id: u64, ids: &[usize]) -> Result<(), FabError> {
        let count = self
            .result
            .as_ref()
            .filter(|r| r.similar_generation == search_id && !r.similar_cancelled)
            .and_then(|r| r.similar_count)
            .ok_or_else(stale)?;
        if ids.is_empty()
            || ids.len() > 16
            || ids
                .iter()
                .enumerate()
                .any(|(i, id)| *id >= count || ids[..i].contains(id))
        {
            return Err(FabError::new(
                "guidance.invalid_selection",
                "Choisir une ou plusieurs parois proposées.",
            ));
        }
        Ok(())
    }
}

#[tauri::command]
pub(super) fn cancel_similar_search(
    request_id: u64,
    source_id: u64,
    search_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<(), FabError> {
    let mut cache = state.inner.lock().map_err(lock_error)?;
    cache.result_snapshot(request_id, source_id)?;
    let result = cache.result.as_mut().ok_or_else(stale)?;
    if search_id >= result.similar_generation {
        result.similar_generation = search_id;
        result.similar_count = None;
        result.similar_cancelled = true;
    }
    Ok(())
}

#[tauri::command]
pub(super) async fn find_similar_guided_regions(
    request_id: u64,
    source_id: u64,
    search_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<SimilarResponse, FabError> {
    let state = Arc::clone(state.inner());
    let (step, epoch) = {
        let mut cache = state.inner.lock().map_err(lock_error)?;
        let snapshot = cache.result_snapshot(request_id, source_id)?;
        let result = cache.result.as_mut().ok_or_else(stale)?;
        if search_id <= result.similar_generation || result.display.is_none() {
            return Err(stale());
        }
        result.similar_generation = search_id;
        result.similar_count = None;
        result.similar_cancelled = false;
        snapshot
    };
    let _permit = SEARCH_WORKER.lock().await;
    let current = {
        let state = Arc::clone(&state);
        let step = Arc::clone(&step);
        move || {
            let cache = state.inner.lock().map_err(lock_error)?;
            if cache.current_result(request_id, source_id, epoch, &step)
                && cache
                    .result
                    .as_ref()
                    .is_some_and(|r| r.similar_generation == search_id && !r.similar_cancelled)
            {
                Ok(())
            } else {
                Err(stale())
            }
        }
    };
    current()?;
    let worker_current = current.clone();
    let task_step = Arc::clone(&step);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        task_step.similar_regions_controlled(&mut |_| worker_current())
    })
    .await;
    current()?;
    let regions = computed.map_err(|error| FabError::new("engine.task", error.to_string()))??;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current_result(request_id, source_id, epoch, &step) {
        return Err(stale());
    }
    let result = cache
        .result
        .as_mut()
        .filter(|r| r.similar_generation == search_id && !r.similar_cancelled)
        .ok_or_else(stale)?;
    result.similar_count = Some(regions.candidates.len());
    Ok(SimilarResponse {
        search_id,
        request_id,
        source_id,
        regions,
    })
}

#[tauri::command]
pub(super) async fn apply_similar_guided_regions(
    request_id: u64,
    source_id: u64,
    base_request_id: u64,
    candidate_ids: Vec<usize>,
    search_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<DocumentReport>, FabError> {
    let state = Arc::clone(state.inner());
    let (step, epoch) = {
        let mut cache = state.inner.lock().map_err(lock_error)?;
        cache.check_similar_selection(search_id, &candidate_ids)?;
        cache.begin_guidance(request_id, source_id, base_request_id)?
    };
    let progress =
        progress::ConversionGuard::new(Arc::clone(&state), request_id, source_id, epoch)?;
    let observer = progress.observer();
    let computed = tauri::async_runtime::spawn_blocking(move || {
        step.apply_similar_controlled(&candidate_ids, &mut |checkpoint| {
            observer.checkpoint(checkpoint)
        })
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current(request_id, source_id, epoch) {
        return Err(stale());
    }
    let conversion =
        computed.map_err(|error| FabError::new("engine.task", error.to_string()))??;
    cache.finish(request_id, source_id, epoch, conversion.step)?;
    Ok(Response {
        request_id,
        report: conversion.report,
    })
}
