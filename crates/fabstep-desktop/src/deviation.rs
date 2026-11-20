//! Latest read-only heatmap job on the immutable, displayed STEP.
use super::*;
use fabstep_app::DisplayDeviationMap;

static DEVIATION_WORKER: tauri::async_runtime::Mutex<()> =
    tauri::async_runtime::Mutex::const_new(());

#[derive(Serialize)]
pub(super) struct DeviationResponse {
    request_id: u64,
    source_id: u64,
    map_id: u64,
    report: DisplayDeviationMap,
}

impl CacheState {
    pub(super) fn current_deviation(
        &self,
        request_id: u64,
        source_id: u64,
        epoch: u64,
        step: &Arc<DocumentStep>,
        map_id: u64,
    ) -> bool {
        self.current_result(request_id, source_id, epoch, step)
            && self
                .result
                .as_ref()
                .is_some_and(|r| r.deviation_generation == map_id && !r.deviation_cancelled)
    }
}

impl CachedStep {
    pub(super) fn start_deviation(
        &mut self,
        map_id: u64,
    ) -> Result<Arc<InspectedDocumentStep>, FabError> {
        if map_id <= self.deviation_generation {
            return Err(stale());
        }
        let display = self.display.clone().ok_or_else(|| {
            FabError::new(
                "step.deviation_unavailable",
                "Afficher les faces STEP avant de mesurer leurs écarts.",
            )
        })?;
        self.deviation_generation = map_id;
        self.deviation_cancelled = false;
        Ok(display)
    }

    pub(super) fn cancel_deviation(&mut self, map_id: u64) {
        // Cancellation may arrive before the start IPC, or after a later start.
        if map_id >= self.deviation_generation {
            self.deviation_generation = map_id;
            self.deviation_cancelled = true;
        }
    }
}

#[tauri::command]
pub(super) fn cancel_deviation_map(
    request_id: u64,
    source_id: u64,
    map_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<(), FabError> {
    let mut cache = state.inner.lock().map_err(lock_error)?;
    cache.result_snapshot(request_id, source_id)?;
    cache
        .result
        .as_mut()
        .ok_or_else(stale)?
        .cancel_deviation(map_id);
    Ok(())
}

#[tauri::command]
pub(super) async fn result_deviation_map(
    request_id: u64,
    source_id: u64,
    map_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<DeviationResponse, FabError> {
    let state = Arc::clone(state.inner());
    let (step, epoch, display) = {
        let mut cache = state.inner.lock().map_err(lock_error)?;
        let (step, epoch) = cache.result_snapshot(request_id, source_id)?;
        let display = cache
            .result
            .as_mut()
            .ok_or_else(stale)?
            .start_deviation(map_id)?;
        (step, epoch, display)
    };
    let job_current = {
        let state = Arc::clone(&state);
        let step = Arc::clone(&step);
        move || {
            let cache = state.inner.lock().map_err(lock_error)?;
            if cache.current_deviation(request_id, source_id, epoch, &step, map_id) {
                Ok(())
            } else {
                Err(stale())
            }
        }
    };

    // One BVH at a time, including during cancellation/replacement. Waiting
    // uses the async runtime, so repeated toggles do not occupy blocking threads.
    let _permit = DEVIATION_WORKER.lock().await;
    job_current()?;
    let mut check_worker = job_current.clone();
    let task_step = Arc::clone(&step);
    let report = tauri::async_runtime::spawn_blocking(move || {
        task_step.display_deviations_controlled(display.faces(), &mut check_worker)
    })
    .await;
    job_current()?;
    let report = report.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    Ok(DeviationResponse {
        request_id,
        source_id,
        map_id,
        report,
    })
}
