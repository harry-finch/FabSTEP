//! Coque Tauri et accès aux fichiers. Le calcul ne bloque jamais le fil de l'interface.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use fabstep_app::{
    DocumentPreview, DocumentReport, DocumentResultFaces, DocumentSelectionTopology, DocumentStep,
    InputFormat,
};
use fabstep_core::{export_planar_face_dxf, FabError, Options, PlanarFaceProfile};
use serde::Serialize;
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{
    ipc::{InvokeBody, Request},
    Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_dialog::DialogExt;

mod progress;
mod section;
mod source_section;

const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Default)]
struct EngineState {
    inner: Mutex<CacheState>,
    uploading: AtomicBool,
}
#[derive(Default)]
struct CacheState {
    revision: u64,
    expected_source: Option<u64>,
    epoch: u64,
    source: Option<CachedSource>,
    result: Option<CachedStep>,
    guidance_session: Option<Arc<DocumentStep>>,
    progress_generation: u64,
    progress: Option<progress::ActiveProgress>,
    face_profile_generation: u64,
    face_profile: Option<FaceProfileContext>,
    section_generation: u64,
    section: Option<section::SectionContext>,
    source_section_generation: u64,
    source_section: Option<source_section::CachedSourceSection>,
}
#[derive(Clone)]
struct FaceProfileContext {
    request_id: u64,
    source_id: u64,
    selection_id: u64,
    face_id: usize,
    epoch: u64,
    step: Arc<DocumentStep>,
    profile: Option<Arc<PlanarFaceProfile>>,
}

struct CachedSource {
    source_id: u64,
    format: InputFormat,
    bytes: Arc<Vec<u8>>,
}
struct CachedStep {
    request_id: u64,
    source_id: u64,
    step: Arc<DocumentStep>,
}
impl CacheState {
    // Les messages IPC peuvent arriver dans un ordre différent de leur émission.
    fn invalidate(&mut self, request_id: u64, source_id: Option<u64>) -> Result<(), FabError> {
        if source_id.is_some_and(|id| id == 0 || id > request_id) {
            return Err(stale());
        }
        if request_id > self.revision {
            self.revision = request_id;
            self.epoch = self.epoch.wrapping_add(1);
            self.result = None;
            self.section = None;
            self.source_section = None;
            self.face_profile = None;
            self.guidance_session = None;
            self.progress = None;
            if self.expected_source != source_id {
                self.source = None;
            }
            self.expected_source = source_id;
        } else if request_id == self.revision && self.expected_source != source_id {
            return Err(stale());
        }
        Ok(())
    }
    fn expect_upload(&mut self, source_id: u64) -> Result<(), FabError> {
        if source_id == 0 {
            return Err(stale());
        }
        if source_id > self.revision {
            self.invalidate(source_id, Some(source_id))?;
        }
        if self.expected_source != Some(source_id) {
            return Err(stale());
        }
        Ok(())
    }
    fn resolve_duplicate(&mut self, source_id: u64, same: bool) -> Result<(), FabError> {
        self.expect_upload(source_id)?;
        if !same {
            self.result = None;
            self.section = None;
            self.source_section = None;
            self.face_profile = None;
            self.guidance_session = None;
            self.progress = None;
            self.epoch = self.epoch.wrapping_add(1);
            return Err(FabError::new(
                "engine.source_conflict",
                "Cette identité de source désigne des données différentes.",
            ));
        }
        Ok(())
    }
    fn begin(
        &mut self,
        request_id: u64,
        source_id: u64,
    ) -> Result<(Arc<Vec<u8>>, InputFormat, u64), FabError> {
        if request_id < self.revision || self.expected_source != Some(source_id) {
            return Err(stale());
        }
        if request_id > self.revision {
            self.invalidate(request_id, Some(source_id))?;
        }
        self.result = None;
        self.section = None;
        self.source_section = None;
        self.face_profile = None;
        self.guidance_session = None;
        self.progress = None;
        let source = self
            .source
            .as_ref()
            .filter(|s| s.source_id == source_id)
            .ok_or_else(stale)?;
        Ok((Arc::clone(&source.bytes), source.format, self.epoch))
    }
    fn current(&self, request_id: u64, source_id: u64, epoch: u64) -> bool {
        self.revision == request_id
            && self.expected_source == Some(source_id)
            && self.epoch == epoch
            && self
                .source
                .as_ref()
                .is_some_and(|s| s.source_id == source_id)
    }
    /// Read-only work must never clear a validated STEP or advance a revision.
    /// The caller first synchronizes its revision with preview/invalidation.
    fn selection_snapshot(
        &self,
        request_id: u64,
        source_id: u64,
    ) -> Result<(Arc<Vec<u8>>, InputFormat, u64), FabError> {
        if !self.current(request_id, source_id, self.epoch) {
            return Err(stale());
        }
        let source = self.source.as_ref().ok_or_else(stale)?;
        Ok((Arc::clone(&source.bytes), source.format, self.epoch))
    }
    fn parameter_search_snapshot(
        &self,
        request_id: u64,
        source_id: u64,
    ) -> Result<(Arc<Vec<u8>>, InputFormat, u64), FabError> {
        let snapshot = self.selection_snapshot(request_id, source_id)?;
        if self
            .guidance_session
            .as_ref()
            .is_some_and(|step| step.has_guidance_hints())
        {
            return Err(FabError::new(
                "search.guidance_active",
                "Retirer les indications manuelles avant de rechercher des réglages.",
            ));
        }
        Ok(snapshot)
    }

    fn result_snapshot(
        &self,
        request_id: u64,
        source_id: u64,
    ) -> Result<(Arc<DocumentStep>, u64), FabError> {
        if !self.current(request_id, source_id, self.epoch) {
            return Err(stale());
        }
        let result = self
            .result
            .as_ref()
            .filter(|r| r.request_id == request_id && r.source_id == source_id)
            .ok_or_else(stale)?;
        Ok((Arc::clone(&result.step), self.epoch))
    }
    fn current_result(
        &self,
        request_id: u64,
        source_id: u64,
        epoch: u64,
        step: &Arc<DocumentStep>,
    ) -> bool {
        self.current(request_id, source_id, epoch)
            && self.result.as_ref().is_some_and(|result| {
                result.request_id == request_id
                    && result.source_id == source_id
                    && Arc::ptr_eq(&result.step, step)
            })
    }
    fn set_face_context(
        &mut self,
        request_id: u64,
        source_id: u64,
        selection_id: u64,
        face_id: Option<usize>,
    ) -> Result<(), FabError> {
        // Selection epochs are independent from conversion revisions. Late
        // clear messages cannot erase a newer preview, including A -> B -> A.
        if selection_id == 0 || selection_id <= self.face_profile_generation {
            return Err(stale());
        }
        self.face_profile_generation = selection_id;
        self.face_profile = None;
        if let Some(face_id) = face_id {
            let (step, epoch) = self.result_snapshot(request_id, source_id)?;
            self.face_profile = Some(FaceProfileContext {
                request_id,
                source_id,
                selection_id,
                face_id,
                epoch,
                step,
                profile: None,
            });
        }
        Ok(())
    }

    fn face_context(
        &self,
        request_id: u64,
        source_id: u64,
        selection_id: u64,
        face_id: usize,
    ) -> Result<FaceProfileContext, FabError> {
        let context = self
            .face_profile
            .as_ref()
            .filter(|context| {
                context.request_id == request_id
                    && context.source_id == source_id
                    && context.selection_id == selection_id
                    && context.face_id == face_id
                    && self.current_face_context(context)
            })
            .ok_or_else(stale)?;
        Ok(context.clone())
    }

    fn current_face_context(&self, context: &FaceProfileContext) -> bool {
        self.face_profile_generation == context.selection_id
            && self.face_profile.as_ref().is_some_and(|current| {
                current.selection_id == context.selection_id
                    && current.face_id == context.face_id
                    && Arc::ptr_eq(&current.step, &context.step)
            })
            && self.current_result(
                context.request_id,
                context.source_id,
                context.epoch,
                &context.step,
            )
    }

    /// Atomically consume the old result before work starts. A caller must not
    /// pre-invalidate the new revision: that would destroy the trusted base.
    fn begin_guidance(
        &mut self,
        request_id: u64,
        source_id: u64,
        base_request_id: u64,
    ) -> Result<(Arc<DocumentStep>, u64), FabError> {
        if request_id <= self.revision || base_request_id != self.revision {
            return Err(stale());
        }
        let (step, _) = self.result_snapshot(base_request_id, source_id)?;
        let source = self.source.as_ref().ok_or_else(stale)?;
        if !step.is_guidance_source(&source.bytes, source.format) {
            return Err(FabError::new(
                "guidance.provenance_unavailable",
                "L’indication exige la provenance exacte du résultat Plans et de sa source courante.",
            ));
        }
        self.invalidate(request_id, Some(source_id))?;
        self.guidance_session = Some(Arc::clone(&step));
        Ok((step, self.epoch))
    }

    /// A failed mutation retains only the accepted native session, never export.
    fn begin_guidance_session(
        &mut self,
        request_id: u64,
        source_id: u64,
        base_request_id: u64,
    ) -> Result<(Arc<DocumentStep>, u64), FabError> {
        if request_id <= self.revision
            || base_request_id != self.revision
            || !self.current(base_request_id, source_id, self.epoch)
        {
            return Err(stale());
        }
        let step = self.guidance_session.as_ref().ok_or_else(stale)?.clone();
        let source = self.source.as_ref().ok_or_else(stale)?;
        if !step.is_guidance_source(&source.bytes, source.format) {
            return Err(stale());
        }
        self.invalidate(request_id, Some(source_id))?;
        self.guidance_session = Some(Arc::clone(&step));
        Ok((step, self.epoch))
    }

    fn finish(
        &mut self,
        request_id: u64,
        source_id: u64,
        epoch: u64,
        step: DocumentStep,
    ) -> Result<(), FabError> {
        if !self.current(request_id, source_id, epoch) {
            return Err(stale());
        }
        let step = Arc::new(step);
        self.guidance_session = Some(Arc::clone(&step));
        self.result = Some(CachedStep {
            request_id,
            source_id,
            step,
        });
        Ok(())
    }
}
#[tauri::command]
fn invalidate_result(
    request_id: u64,
    source_id: Option<u64>,
    state: State<'_, Arc<EngineState>>,
) -> Result<(), FabError> {
    state
        .inner
        .lock()
        .map_err(lock_error)?
        .invalidate(request_id, source_id)
}

#[tauri::command]
fn get_conversion_progress(
    request_id: u64,
    source_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<Option<progress::ConversionProgress>, FabError> {
    Ok(state
        .inner
        .lock()
        .map_err(lock_error)?
        .conversion_progress(request_id, source_id))
}
struct UploadGuard<'a>(&'a AtomicBool);
impl Drop for UploadGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn validate_source_size(size: usize) -> Result<(), FabError> {
    if size > MAX_SOURCE_BYTES {
        return Err(FabError::new("input.too_large", "Fichier limité à 64 Mio."));
    }
    Ok(())
}

#[tauri::command]
fn upload_source(request: Request<'_>, state: State<'_, Arc<EngineState>>) -> Result<(), FabError> {
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .filter(|v| v.len() <= 20)
            .ok_or_else(|| {
                FabError::new("engine.source_metadata", "Métadonnées de source invalides.")
            })
    };
    let source_id: u64 = header("x-fabstep-source-id")?
        .parse()
        .map_err(|_| FabError::new("engine.source_metadata", "Identifiant de source invalide."))?;
    let format = match header("x-fabstep-format")? {
        "stl" => InputFormat::Stl,
        "3mf" => InputFormat::ThreeMf,
        _ => {
            return Err(FabError::new(
                "engine.source_metadata",
                "Format de source invalide.",
            ))
        }
    };
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => {
            return Err(FabError::new(
                "engine.source_transport",
                "Transport binaire de la source indisponible.",
            ))
        }
    };
    upload_bytes(state.inner(), source_id, format, bytes)
}

fn upload_bytes(
    state: &EngineState,
    source_id: u64,
    format: InputFormat,
    bytes: &[u8],
) -> Result<(), FabError> {
    validate_source_size(bytes.len())?;
    if state
        .uploading
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err(FabError::new(
            "engine.source_busy",
            "Un transfert de source est déjà en cours.",
        ));
    }
    let _guard = UploadGuard(&state.uploading);
    let existing = {
        let mut cache = state.inner.lock().map_err(lock_error)?;
        cache.expect_upload(source_id)?;
        cache
            .source
            .as_ref()
            .map(|s| (s.format, Arc::clone(&s.bytes)))
    };
    if let Some((old_format, old_bytes)) = existing {
        let same = old_format == format && old_bytes.as_slice() == bytes;
        let mut cache = state.inner.lock().map_err(lock_error)?;
        return cache.resolve_duplicate(source_id, same);
    }
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len()).map_err(|_| {
        FabError::new(
            "engine.source_allocation",
            "Mémoire insuffisante pour la source.",
        )
    })?;
    copy.extend_from_slice(bytes);
    let mut cache = state.inner.lock().map_err(lock_error)?;
    cache.expect_upload(source_id)?;
    if cache.source.is_some() {
        return Err(stale());
    }
    cache.source = Some(CachedSource {
        source_id,
        format,
        bytes: Arc::new(copy),
    });
    Ok(())
}
#[derive(Serialize)]
struct Response<T> {
    request_id: u64,
    #[serde(flatten)]
    report: T,
}

#[derive(Serialize)]
struct SelectionResponse {
    request_id: u64,
    source_id: u64,
    #[serde(flatten)]
    report: DocumentSelectionTopology,
}

#[derive(Serialize)]
struct ResultFacesResponse {
    request_id: u64,
    source_id: u64,
    #[serde(flatten)]
    report: DocumentResultFaces,
}

#[derive(Serialize)]
struct RegionResponse {
    request_id: u64,
    face_ids: Vec<usize>,
}

#[tauri::command]
async fn suggest_region(
    request_id: u64,
    face_id: usize,
    state: State<'_, Arc<EngineState>>,
) -> Result<RegionResponse, FabError> {
    let state = Arc::clone(state.inner());
    let (step, source_id, epoch) = {
        let cache = state.inner.lock().map_err(lock_error)?;
        let source_id = cache.expected_source.ok_or_else(stale)?;
        let (step, epoch) = cache.result_snapshot(request_id, source_id)?;
        (step, source_id, epoch)
    };
    let task_step = Arc::clone(&step);
    let worker_state = Arc::clone(&state);
    let task = tauri::async_runtime::spawn_blocking(move || {
        task_step.suggest_region_controlled(face_id, &mut |_| {
            if worker_state
                .inner
                .lock()
                .map_err(lock_error)?
                .current_result(request_id, source_id, epoch, &task_step)
            {
                Ok(())
            } else {
                Err(stale())
            }
        })
    })
    .await;
    if !state
        .inner
        .lock()
        .map_err(lock_error)?
        .current_result(request_id, source_id, epoch, &step)
    {
        return Err(stale());
    }
    let face_ids =
        task.map_err(|e| FabError::new("engine.task", format!("Sélection interrompue : {e}")))??;
    Ok(RegionResponse {
        request_id,
        face_ids,
    })
}

#[tauri::command]
async fn result_faces(
    request_id: u64,
    source_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<ResultFacesResponse, FabError> {
    let state = Arc::clone(state.inner());
    let (step, epoch) = state
        .inner
        .lock()
        .map_err(lock_error)?
        .result_snapshot(request_id, source_id)?;
    let task_step = Arc::clone(&step);
    let worker_state = Arc::clone(&state);
    let report = tauri::async_runtime::spawn_blocking(move || {
        task_step.result_faces_controlled(&mut |_| {
            if worker_state
                .inner
                .lock()
                .map_err(lock_error)?
                .current_result(request_id, source_id, epoch, &task_step)
            {
                Ok(())
            } else {
                Err(stale())
            }
        })
    })
    .await;
    // Reject changed result identity, including a replacement at the same revision,
    // before returning either the inspection or its failure. Never mutate the cache.
    if !state
        .inner
        .lock()
        .map_err(lock_error)?
        .current_result(request_id, source_id, epoch, &step)
    {
        return Err(stale());
    }
    Ok(ResultFacesResponse {
        request_id,
        source_id,
        report: report.map_err(|e| FabError::new("engine.task", e.to_string()))??,
    })
}

#[tauri::command]
async fn selection_topology(
    request_id: u64,
    source_id: u64,
    options: Options,
    state: State<'_, Arc<EngineState>>,
) -> Result<SelectionResponse, FabError> {
    let state = Arc::clone(state.inner());
    let (bytes, format, epoch) = state
        .inner
        .lock()
        .map_err(lock_error)?
        .selection_snapshot(request_id, source_id)?;
    let report = tauri::async_runtime::spawn_blocking(move || {
        fabstep_app::selection_topology(&bytes, format, &options)
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))?;
    // Check staleness before propagating either graph or topology failure.
    if !state
        .inner
        .lock()
        .map_err(lock_error)?
        .current(request_id, source_id, epoch)
    {
        return Err(stale());
    }
    Ok(SelectionResponse {
        request_id,
        source_id,
        report: report?,
    })
}

#[tauri::command]
async fn preview_document(
    request_id: u64,
    source_id: u64,
    options: Options,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<DocumentPreview>, FabError> {
    let state = Arc::clone(state.inner());
    let (bytes, format, epoch) = state
        .inner
        .lock()
        .map_err(lock_error)?
        .begin(request_id, source_id)?;
    let report = tauri::async_runtime::spawn_blocking(move || {
        fabstep_app::preview_document(&bytes, format, &options)
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))??;
    if !state
        .inner
        .lock()
        .map_err(lock_error)?
        .current(request_id, source_id, epoch)
    {
        return Err(stale());
    }
    Ok(Response { request_id, report })
}

/// Read-only proposals: applying one uses the ordinary conversion transaction.
#[tauri::command]
async fn search_parameters(
    request_id: u64,
    source_id: u64,
    options: Options,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<fabstep_app::ParameterSearchReport>, FabError> {
    let state = Arc::clone(state.inner());
    let (bytes, format, epoch) = state
        .inner
        .lock()
        .map_err(lock_error)?
        .parameter_search_snapshot(request_id, source_id)?;
    let worker_state = Arc::clone(&state);
    let report = tauri::async_runtime::spawn_blocking(move || {
        fabstep_app::search_parameters(&bytes, format, &options, || {
            if worker_state
                .inner
                .lock()
                .map_err(lock_error)?
                .current(request_id, source_id, epoch)
            {
                Ok(())
            } else {
                Err(stale())
            }
        })
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))??;
    if !state
        .inner
        .lock()
        .map_err(lock_error)?
        .current(request_id, source_id, epoch)
    {
        return Err(stale());
    }
    Ok(Response { request_id, report })
}

#[tauri::command]
async fn convert_document(
    request_id: u64,
    source_id: u64,
    options: Options,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<DocumentReport>, FabError> {
    let state = Arc::clone(state.inner());
    let (bytes, format, epoch) = state
        .inner
        .lock()
        .map_err(lock_error)?
        .begin(request_id, source_id)?;
    let progress =
        progress::ConversionGuard::new(Arc::clone(&state), request_id, source_id, epoch)?;
    let observer = progress.observer();
    let result = tauri::async_runtime::spawn_blocking(move || {
        fabstep_app::convert_shared_document_with_provenance_controlled(
            bytes,
            format,
            &options,
            &mut |checkpoint| observer.checkpoint(checkpoint),
        )
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))??;
    state
        .inner
        .lock()
        .map_err(lock_error)?
        .finish(request_id, source_id, epoch, result.step)?;
    Ok(Response {
        request_id,
        report: result.report,
    })
}

// Missing family preserves the historical cylinder command. Unknown hypotheses
// are refused inside the consumed revision, never leaving an old STEP exportable.
fn parse_guidance_family(family: Option<&str>) -> Result<fabstep_core::GuidanceFamily, FabError> {
    match family.unwrap_or("cylinder") {
        "auto" => Ok(fabstep_core::GuidanceFamily::Auto),
        "torus" => Ok(fabstep_core::GuidanceFamily::Torus),
        "plane" => Ok(fabstep_core::GuidanceFamily::Plane),
        "cylinder" => Ok(fabstep_core::GuidanceFamily::Cylinder),
        "cone" => Ok(fabstep_core::GuidanceFamily::Cone),
        _ => Err(FabError::new(
            "guidance.unsupported_family",
            "Famille guidée non prise en charge.",
        )),
    }
}

#[tauri::command]
async fn guided_convert(
    request_id: u64,
    source_id: u64,
    base_request_id: u64,
    face_ids: Vec<usize>,
    family: Option<String>,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<DocumentReport>, FabError> {
    let state = Arc::clone(state.inner());
    let (step, epoch) = state.inner.lock().map_err(lock_error)?.begin_guidance(
        request_id,
        source_id,
        base_request_id,
    )?;
    let progress =
        progress::ConversionGuard::new(Arc::clone(&state), request_id, source_id, epoch)?;
    let observer = progress.observer();
    let computed = tauri::async_runtime::spawn_blocking(move || {
        let family = parse_guidance_family(family.as_deref())?;
        step.guided_surface_controlled(&face_ids, family, &mut |checkpoint| {
            observer.checkpoint(checkpoint)
        })
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    // Superseded failures and successes have the same stale semantics.
    if !cache.current(request_id, source_id, epoch) {
        return Err(stale());
    }
    let conversion = computed.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    cache.finish(request_id, source_id, epoch, conversion.step)?;
    Ok(Response {
        request_id,
        report: conversion.report,
    })
}

#[tauri::command]
async fn remove_guidance(
    request_id: u64,
    source_id: u64,
    base_request_id: u64,
    hint_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<DocumentReport>, FabError> {
    mutate_guidance_session(
        request_id,
        source_id,
        base_request_id,
        Some(hint_id),
        Arc::clone(state.inner()),
    )
    .await
}

#[tauri::command]
async fn recompute_guidance(
    request_id: u64,
    source_id: u64,
    base_request_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<DocumentReport>, FabError> {
    mutate_guidance_session(
        request_id,
        source_id,
        base_request_id,
        None,
        Arc::clone(state.inner()),
    )
    .await
}

async fn mutate_guidance_session(
    request_id: u64,
    source_id: u64,
    base_request_id: u64,
    remove_hint_id: Option<u64>,
    state: Arc<EngineState>,
) -> Result<Response<DocumentReport>, FabError> {
    let (step, epoch) = state
        .inner
        .lock()
        .map_err(lock_error)?
        .begin_guidance_session(request_id, source_id, base_request_id)?;
    let progress =
        progress::ConversionGuard::new(Arc::clone(&state), request_id, source_id, epoch)?;
    let observer = progress.observer();
    let computed = tauri::async_runtime::spawn_blocking(move || {
        let mut check_current = |checkpoint| observer.checkpoint(checkpoint);
        match remove_hint_id {
            Some(id) => step.remove_guidance_controlled(id, &mut check_current),
            None => step.recompute_guidance_controlled(&mut check_current),
        }
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current(request_id, source_id, epoch) {
        return Err(stale());
    }
    let conversion = computed.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    cache.finish(request_id, source_id, epoch, conversion.step)?;
    Ok(Response {
        request_id,
        report: conversion.report,
    })
}

#[derive(Serialize)]
struct FaceProfileResponse {
    request_id: u64,
    source_id: u64,
    selection_id: u64,
    profile: PlanarFaceProfile,
}

#[tauri::command]
fn set_face_profile_context(
    request_id: u64,
    source_id: u64,
    selection_id: u64,
    face_id: Option<usize>,
    state: State<'_, Arc<EngineState>>,
) -> Result<(), FabError> {
    state.inner.lock().map_err(lock_error)?.set_face_context(
        request_id,
        source_id,
        selection_id,
        face_id,
    )
}

#[tauri::command]
async fn face_profile(
    request_id: u64,
    source_id: u64,
    selection_id: u64,
    face_id: usize,
    state: State<'_, Arc<EngineState>>,
) -> Result<FaceProfileResponse, FabError> {
    let state = Arc::clone(state.inner());
    let context = state.inner.lock().map_err(lock_error)?.face_context(
        request_id,
        source_id,
        selection_id,
        face_id,
    )?;
    let worker_context = context.clone();
    let worker_state = Arc::clone(&state);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        if !worker_state
            .inner
            .lock()
            .map_err(lock_error)?
            .current_face_context(&worker_context)
        {
            return Err(stale());
        }
        worker_context
            .step
            .planar_face_profile(worker_context.face_id)
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current_face_context(&context) {
        return Err(stale());
    }
    let profile = computed.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    cache.face_profile.as_mut().ok_or_else(stale)?.profile = Some(Arc::new(profile.clone()));
    Ok(FaceProfileResponse {
        request_id,
        source_id,
        selection_id,
        profile,
    })
}

#[tauri::command]
async fn export_face_dxf(
    request_id: u64,
    source_id: u64,
    selection_id: u64,
    face_id: usize,
    suggested_name: String,
    app: tauri::AppHandle,
    state: State<'_, Arc<EngineState>>,
) -> Result<Option<String>, FabError> {
    let state = Arc::clone(state.inner());
    let context = state.inner.lock().map_err(lock_error)?.face_context(
        request_id,
        source_id,
        selection_id,
        face_id,
    )?;
    let profile = context.profile.as_ref().cloned().ok_or_else(|| {
        FabError::new(
            "dxf.profile_not_ready",
            "Attendez le chargement du profil avant d’exporter.",
        )
    })?;
    let name = std::path::Path::new(&suggested_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("FabSTEP.dxf")
        .to_string();
    tauri::async_runtime::spawn_blocking(move || {
        // Serialize only trusted native geometry from the displayed result.
        let bytes = export_planar_face_dxf(&profile)?;
        if !state
            .inner
            .lock()
            .map_err(lock_error)?
            .current_face_context(&context)
        {
            return Err(stale());
        }
        let Some(destination) = app
            .dialog()
            .file()
            .add_filter("DXF", &["dxf"])
            .set_file_name(name)
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = destination
            .into_path()
            .map_err(|e| FabError::new("io.path", e.to_string()))?;
        if !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("dxf"))
        {
            return Err(FabError::new("io.extension", "Choisissez un fichier .dxf."));
        }
        // Keep the identity guard through the atomic destination replacement,
        // as for STEP export. A changed selection during the dialog is refused.
        let cache = state.inner.lock().map_err(lock_error)?;
        if !cache.current_face_context(&context) {
            return Err(stale());
        }
        let mut file = tempfile::NamedTempFile::new_in(
            path.parent()
                .ok_or_else(|| FabError::new("io.path", "Dossier de destination absent."))?,
        )
        .map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        file.as_file().sync_all().map_err(io_error)?;
        file.persist(&path).map_err(|e| io_error(e.error))?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))?
}

#[tauri::command]
async fn export_step(
    request_id: u64,
    suggested_name: String,
    app: tauri::AppHandle,
    state: State<'_, Arc<EngineState>>,
) -> Result<Option<String>, FabError> {
    let state = Arc::clone(state.inner());
    {
        let cache = state.inner.lock().map_err(lock_error)?;
        if cache.revision != request_id
            || cache.result.as_ref().is_none_or(|r| {
                r.request_id != request_id || Some(r.source_id) != cache.expected_source
            })
        {
            return Err(stale());
        }
    }
    let name = std::path::Path::new(&suggested_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("FabSTEP.step")
        .to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(destination) = app
            .dialog()
            .file()
            .add_filter("STEP", &["step", "stp"])
            .set_file_name(name)
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = destination
            .into_path()
            .map_err(|e| FabError::new("io.path", e.to_string()))?;
        if !path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("step") || s.eq_ignore_ascii_case("stp"))
        {
            return Err(FabError::new(
                "io.extension",
                "Choisissez un fichier .step ou .stp.",
            ));
        }
        let cache = state.inner.lock().map_err(lock_error)?;
        if cache.revision != request_id {
            return Err(stale());
        }
        let result = cache
            .result
            .as_ref()
            .filter(|r| r.request_id == request_id && Some(r.source_id) == cache.expected_source)
            .ok_or_else(stale)?;
        let mut file = tempfile::NamedTempFile::new_in(
            path.parent()
                .ok_or_else(|| FabError::new("io.path", "Dossier de destination absent."))?,
        )
        .map_err(io_error)?;
        file.write_all(result.step.bytes()).map_err(io_error)?;
        file.as_file().sync_all().map_err(io_error)?;
        file.persist(&path).map_err(|e| io_error(e.error))?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))?
}
fn stale() -> FabError {
    FabError::new(
        "engine.stale",
        "Le résultat a changé. Attendez la fin de l'analyse avant d'exporter.",
    )
}
fn lock_error<T>(_: std::sync::PoisonError<T>) -> FabError {
    FabError::new(
        "engine.state",
        "L'état du moteur est indisponible ; relancez FabSTEP.",
    )
}
fn io_error(e: std::io::Error) -> FabError {
    FabError::new("io.file", e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(EngineState::default()))
        .invoke_handler(tauri::generate_handler![
            suggest_region,
            upload_source,
            preview_document,
            selection_topology,
            result_faces,
            set_face_profile_context,
            face_profile,
            export_face_dxf,
            source_section::prepare_source_section,
            section::set_section_context,
            section::section_profile,
            section::export_section_dxf,
            convert_document,
            search_parameters,
            guided_convert,
            remove_guidance,
            recompute_guidance,
            invalidate_result,
            get_conversion_progress,
            export_step
        ])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("FabSTEP — STL / 3MF vers STEP")
                .inner_size(1440.0, 900.0)
                .min_inner_size(1024.0, 680.0)
                .disable_drag_drop_handler()
                .build()?;
            // N'expose aucun accès générique au système de fichiers au JavaScript.
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer FabSTEP");
}

#[cfg(test)]
mod tests {
    use super::{
        upload_bytes, validate_source_size, CacheState, CachedSource, DocumentStep, EngineState,
        InputFormat, MAX_SOURCE_BYTES,
    };
    use std::sync::Arc;

    #[test]
    fn guidance_family_defaults_and_unknown_family_consumes_only_current_revision() {
        use fabstep_core::GuidanceFamily;
        assert_eq!(
            super::parse_guidance_family(None).unwrap(),
            GuidanceFamily::Cylinder
        );
        assert_eq!(
            super::parse_guidance_family(Some("cylinder")).unwrap(),
            GuidanceFamily::Cylinder
        );
        assert_eq!(
            super::parse_guidance_family(Some("cone")).unwrap(),
            GuidanceFamily::Cone
        );
        assert_eq!(
            super::parse_guidance_family(Some("plane")).unwrap(),
            GuidanceFamily::Plane
        );
        assert_eq!(
            super::parse_guidance_family(Some("auto")).unwrap(),
            GuidanceFamily::Auto
        );
        assert_eq!(
            super::parse_guidance_family(Some("torus")).unwrap(),
            GuidanceFamily::Torus
        );
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 300);
        let (_, epoch) = cache.begin_guidance(301, 300, 300).unwrap();
        assert_eq!(
            super::parse_guidance_family(Some("sphere"))
                .unwrap_err()
                .code,
            "guidance.unsupported_family"
        );
        assert!(cache.result.is_none());
        assert!(cache.guidance_session.is_some());
        assert!(cache.current(301, 300, epoch));
        cache.invalidate(302, Some(300)).unwrap();
        assert!(!cache.current(301, 300, epoch));
        assert!(cache.guidance_session.is_none());
    }

    fn install(cache: &mut CacheState, id: u64, data: &[u8]) {
        cache.expect_upload(id).unwrap();
        cache.source = Some(CachedSource {
            source_id: id,
            format: InputFormat::Stl,
            bytes: Arc::new(data.to_vec()),
        });
    }

    #[test]
    fn revisions_and_source_identity_reject_late_upload_and_result() {
        let mut c = CacheState::default();
        install(&mut c, 10, &[1, 2]);
        let (_, _, epoch) = c.begin(10, 10).unwrap();
        c.invalidate(11, Some(10)).unwrap(); // same file, changed options/unit
        assert!(c.source.is_some());
        assert!(c
            .finish(10, 10, epoch, DocumentStep::untracked(vec![1]))
            .is_err());
        let (snapshot, _, next) = c.begin(11, 10).unwrap();
        c.finish(11, 10, next, DocumentStep::untracked(vec![2]))
            .unwrap();
        c.invalidate(10, Some(10)).unwrap(); // late invalidation is ignored
        assert_eq!(c.result.as_ref().unwrap().step.bytes(), &[2]);
        c.invalidate(12, Some(12)).unwrap(); // different file
        assert!(c.source.is_none());
        assert_eq!(snapshot.as_slice(), &[1, 2]); // running task owns old bounded snapshot
        assert!(c.expect_upload(10).is_err());
        assert!(c.begin(12, 10).is_err());
        assert!(c
            .finish(11, 10, next, DocumentStep::untracked(vec![3]))
            .is_err());
        install(&mut c, 12, &[3]);
        assert!(c.begin(12, 12).is_ok());
    }

    #[test]
    fn upload_before_invalidation_and_late_same_source_upload() {
        let mut c = CacheState::default();
        c.expect_upload(20).unwrap(); // upload arrived first
        c.invalidate(20, Some(20)).unwrap();
        c.invalidate(21, Some(20)).unwrap(); // options changed during upload
        assert!(c.expect_upload(20).is_ok());
        c.invalidate(22, None).unwrap(); // file selection or cancellation
        assert!(c.expect_upload(20).is_err());
        assert!(c.invalidate(22, Some(20)).is_err());
    }
    #[test]
    fn conflicting_duplicate_invalidates_cached_and_inflight_results() {
        let mut c = CacheState::default();
        install(&mut c, 30, &[1, 2]);
        let (_, _, epoch) = c.begin(30, 30).unwrap();
        c.finish(30, 30, epoch, DocumentStep::untracked(vec![4]))
            .unwrap();
        c.resolve_duplicate(30, true).unwrap();
        assert!(c.result.is_some());
        assert_eq!(
            c.resolve_duplicate(30, false).unwrap_err().code,
            "engine.source_conflict"
        );
        assert!(c.result.is_none());
        assert!(c
            .finish(30, 30, epoch, DocumentStep::untracked(vec![5]))
            .is_err());
        assert_eq!(c.source.as_ref().unwrap().bytes.as_slice(), &[1, 2]);
    }

    #[test]
    fn source_size_is_bounded_before_copy() {
        assert!(validate_source_size(0).is_ok());
        assert!(validate_source_size(MAX_SOURCE_BYTES).is_ok());
        assert_eq!(
            validate_source_size(MAX_SOURCE_BYTES + 1).unwrap_err().code,
            "input.too_large"
        );
    }
    #[test]
    fn selection_snapshot_is_read_only_and_requires_current_request_and_source() {
        let mut c = CacheState::default();
        install(&mut c, 50, &[1, 2]);
        let (_, _, epoch) = c.begin(50, 50).unwrap();
        c.finish(50, 50, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        let (bytes, _, selected_epoch) = c.selection_snapshot(50, 50).unwrap();
        assert_eq!(bytes.as_slice(), &[1, 2]);
        assert_eq!(selected_epoch, epoch);
        for (request, source) in [(49, 50), (51, 50), (50, 49), (50, 51)] {
            assert_eq!(
                c.selection_snapshot(request, source).unwrap_err().code,
                "engine.stale"
            );
        }
        assert_eq!(c.revision, 50);
        assert_eq!(c.epoch, epoch);
        assert_eq!(c.result.as_ref().unwrap().step.bytes(), &[9]);
        assert!(c.current(50, 50, epoch));
    }

    #[test]
    fn selection_completion_rejects_changed_options_source_and_conflicting_upload_epoch() {
        let mut c = CacheState::default();
        install(&mut c, 60, &[1]);
        let (_, _, old) = c.selection_snapshot(60, 60).unwrap();
        c.invalidate(61, Some(60)).unwrap();
        assert!(!c.current(60, 60, old));
        let (_, _, epoch) = c.selection_snapshot(61, 60).unwrap();
        c.resolve_duplicate(60, false).unwrap_err();
        assert!(!c.current(61, 60, epoch));
        let (_, _, epoch) = c.selection_snapshot(61, 60).unwrap();
        c.invalidate(62, Some(62)).unwrap();
        install(&mut c, 62, &[1]); // same bytes, new identity
        assert!(!c.current(61, 60, epoch));
        assert!(c.selection_snapshot(62, 62).is_ok());
    }

    #[test]
    fn result_snapshot_requires_validated_current_step_and_rejects_same_revision_replacement() {
        let mut c = CacheState::default();
        install(&mut c, 70, &[1]);
        assert_eq!(c.result_snapshot(70, 70).unwrap_err().code, "engine.stale");
        let (_, _, epoch) = c.begin(70, 70).unwrap();
        c.finish(70, 70, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        let (snapshot, saved_epoch) = c.result_snapshot(70, 70).unwrap();
        assert!(c.current_result(70, 70, saved_epoch, &snapshot));
        for (request, source) in [(69, 70), (71, 70), (70, 69), (70, 71)] {
            assert_eq!(
                c.result_snapshot(request, source).unwrap_err().code,
                "engine.stale"
            );
        }
        // An inspection failure is independent from the validated STEP cache.
        assert!(snapshot.result_faces().is_err());
        assert!(c.current_result(70, 70, saved_epoch, &snapshot));
        assert_eq!(c.result.as_ref().unwrap().step.bytes(), &[9]);
        c.begin(70, 70).unwrap();
        assert!(!c.current_result(70, 70, saved_epoch, &snapshot));
        c.finish(70, 70, epoch, DocumentStep::untracked(vec![9]))
            .unwrap(); // equal bytes are a new result
        assert!(!c.current_result(70, 70, saved_epoch, &snapshot));
        let (snapshot, epoch) = c.result_snapshot(70, 70).unwrap();
        c.invalidate(71, Some(70)).unwrap();
        assert!(!c.current_result(70, 70, epoch, &snapshot));
    }

    #[test]
    fn result_completion_rejects_source_change_and_conflicting_upload() {
        let mut c = CacheState::default();
        install(&mut c, 80, &[1]);
        let (_, _, epoch) = c.begin(80, 80).unwrap();
        c.finish(80, 80, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        let (snapshot, epoch) = c.result_snapshot(80, 80).unwrap();
        c.resolve_duplicate(80, false).unwrap_err();
        assert!(!c.current_result(80, 80, epoch, &snapshot));
        let (_, _, epoch) = c.begin(80, 80).unwrap();
        c.finish(80, 80, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        let (snapshot, epoch) = c.result_snapshot(80, 80).unwrap();
        install(&mut c, 81, &[1]);
        assert!(!c.current_result(80, 80, epoch, &snapshot));
    }
    #[test]
    fn provenance_and_step_bytes_share_one_cache_identity_and_invalidation() {
        let bytes = include_bytes!("../../../fixtures/cube.stl");
        let mut cache = CacheState::default();
        install(&mut cache, 90, bytes);
        let (_, _, epoch) = cache.begin(90, 90).unwrap();
        let conversion = fabstep_app::convert_document_with_provenance(
            bytes,
            InputFormat::Stl,
            &fabstep_core::Options::default(),
        )
        .unwrap();
        let same_bytes = conversion.step.bytes().to_vec();
        cache.finish(90, 90, epoch, conversion.step).unwrap();
        let (snapshot, saved_epoch) = cache.result_snapshot(90, 90).unwrap();
        assert_eq!(
            snapshot
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "exact"
        );
        // Identical export bytes without the original trusted source record
        // must neither inherit correspondence nor retain the old identity.
        cache
            .finish(90, 90, epoch, DocumentStep::untracked(same_bytes))
            .unwrap();
        assert!(!cache.current_result(90, 90, saved_epoch, &snapshot));
        let (replacement, _) = cache.result_snapshot(90, 90).unwrap();
        assert_eq!(
            replacement
                .result_faces()
                .unwrap()
                .source_correspondence
                .status,
            "unavailable"
        );
        assert_eq!(replacement.bytes(), snapshot.bytes());
        cache.invalidate(91, Some(90)).unwrap();
        assert!(cache.result.is_none());
        assert!(!cache.current_result(90, 90, saved_epoch, &snapshot));
        assert_eq!(
            cache.result_snapshot(91, 90).unwrap_err().code,
            "engine.stale"
        );
    }

    #[test]
    fn parameter_search_preserves_export_and_obeys_revision_and_guidance_guards() {
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 200);
        let (result, epoch) = cache.result_snapshot(200, 200).unwrap();
        assert!(cache.parameter_search_snapshot(200, 200).is_ok());
        assert!(cache.current_result(200, 200, epoch, &result));
        cache.invalidate(201, Some(200)).unwrap();
        assert_eq!(
            cache.parameter_search_snapshot(200, 200).unwrap_err().code,
            "engine.stale"
        );
        assert!(!cache.current(200, 200, epoch));
        let (_, _, epoch) = cache.parameter_search_snapshot(201, 200).unwrap();
        cache.resolve_duplicate(200, false).unwrap_err();
        assert!(!cache.current(201, 200, epoch));

        let bytes = include_bytes!("../../../fixtures/m2b_boss_blind.stl");
        install(&mut cache, 210, bytes);
        let (source, format, epoch) = cache.begin(210, 210).unwrap();
        let planar = fabstep_app::convert_shared_document_with_provenance(
            source,
            format,
            &fabstep_core::Options::default(),
        )
        .unwrap();
        let options = fabstep_core::Options {
            reconstruction: fabstep_core::ReconstructionMode::Cylinders,
            ..fabstep_core::Options::default()
        };
        let analysis = fabstep_core::analyze_stl(bytes, &options).unwrap();
        let mixed = fabstep_core::recognize_mixed(&analysis, &options).unwrap();
        let triangle = mixed
            .triangle_faces
            .iter()
            .position(|f| matches!(f, fabstep_core::MixedFaceRef::Cylinder { .. }))
            .unwrap();
        let faces = planar.step.result_faces().unwrap();
        let seed = faces
            .source_correspondence
            .faces
            .iter()
            .find(|f| f.source_triangle_ids.contains(&triangle))
            .unwrap()
            .face_id;
        let guided = planar.step.guided_cylinder(&[seed]).unwrap();
        cache.finish(210, 210, epoch, guided.step).unwrap();
        let (result, _) = cache.result_snapshot(210, 210).unwrap();
        assert_eq!(
            cache.parameter_search_snapshot(210, 210).unwrap_err().code,
            "search.guidance_active"
        );
        assert!(cache.current_result(210, 210, epoch, &result));
        // A failed guided attempt retains accepted hints but no export.
        cache.begin_guidance(211, 210, 210).unwrap();
        assert!(cache.result.is_none());
        assert_eq!(
            cache.parameter_search_snapshot(211, 210).unwrap_err().code,
            "search.guidance_active"
        );
    }

    fn install_guidance_base(cache: &mut CacheState, id: u64) {
        install(cache, id, include_bytes!("../../../fixtures/cube.stl"));
        let (bytes, format, epoch) = cache.begin(id, id).unwrap();
        let converted = fabstep_app::convert_shared_document_with_provenance(
            bytes,
            format,
            &fabstep_core::Options::default(),
        )
        .unwrap();
        cache.finish(id, id, epoch, converted.step).unwrap();
    }

    #[test]
    fn refused_cone_session_recomputes_instead_of_restoring_old_export() {
        use fabstep_core::{GuidanceFamily, Options, ReconstructionMode, RevolvedFaceRef};
        let data = include_bytes!("../../../fixtures/m4_frustum.stl");
        let mut cache = CacheState::default();
        install(&mut cache, 400, data);
        let (bytes, format, epoch) = cache.begin(400, 400).unwrap();
        let base = fabstep_app::convert_shared_document_with_provenance(
            bytes,
            format,
            &Options::default(),
        )
        .unwrap();
        let analysis = fabstep_core::analyze_stl(data, &Options::default()).unwrap();
        let recognition = fabstep_core::recognize_revolved(
            &analysis,
            &Options {
                reconstruction: ReconstructionMode::Primitives,
                ..Options::default()
            },
        )
        .unwrap();
        let triangle = recognition
            .triangle_faces
            .iter()
            .position(|f| matches!(f, RevolvedFaceRef::Cone { .. }))
            .unwrap();
        let inspected = base.step.result_faces().unwrap();
        let seed = inspected
            .source_correspondence
            .faces
            .iter()
            .find(|face| face.source_triangle_ids.contains(&triangle))
            .unwrap()
            .face_id;
        cache.finish(400, 400, epoch, base.step).unwrap();
        let (base, epoch) = cache.begin_guidance(401, 400, 400).unwrap();
        let converted = base.guided_surface(&[seed], GuidanceFamily::Cone).unwrap();
        let faces = converted.step.result_faces().unwrap();
        let cap = faces
            .source_correspondence
            .faces
            .iter()
            .find(|face| {
                face.source_triangle_ids.iter().all(|&t| {
                    matches!(recognition.triangle_faces[t], RevolvedFaceRef::Plane { .. })
                })
            })
            .unwrap()
            .face_id;
        cache.finish(401, 400, epoch, converted.step).unwrap();
        let (accepted, epoch) = cache.result_snapshot(401, 400).unwrap();
        let (base, _) = cache.begin_guidance(402, 400, 401).unwrap();
        assert!(base.guided_surface(&[cap], GuidanceFamily::Cone).is_err());
        assert!(cache.result.is_none());
        assert!(!cache.current_result(401, 400, epoch, &accepted));
        let (session, epoch) = cache.begin_guidance_session(403, 400, 402).unwrap();
        let recalculated = session.recompute_guidance().unwrap();
        assert_eq!(recalculated.report.cad.conical_faces, 1);
        assert_eq!(recalculated.report.guidance_hints.len(), 1);
        assert_eq!(recalculated.report.guidance_hints[0].family, "cone");
        cache.finish(403, 400, epoch, recalculated.step).unwrap();
        assert!(cache.result_snapshot(403, 400).is_ok());
    }

    #[test]
    fn guidance_consumes_current_base_and_failure_keeps_only_source() {
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 100);
        let (old, old_epoch) = cache.result_snapshot(100, 100).unwrap();
        for (request, source, base) in [(100, 100, 100), (101, 99, 100), (101, 100, 99)] {
            assert_eq!(
                cache
                    .begin_guidance(request, source, base)
                    .unwrap_err()
                    .code,
                "engine.stale"
            );
            assert!(cache.current_result(100, 100, old_epoch, &old));
        }
        let (base, epoch) = cache.begin_guidance(101, 100, 100).unwrap();
        assert!(Arc::ptr_eq(&base, &old));
        assert!(cache.result.is_none());
        assert!(cache.source.is_some());
        assert!(cache.current(101, 100, epoch));
        assert!(!cache.current_result(100, 100, old_epoch, &old));
        // The cube cannot become a cylinder. Nothing restores the old export.
        assert_eq!(
            base.guided_cylinder(&[0]).err().unwrap().code,
            "guidance.cylinder_unproven"
        );
        assert!(cache.result.is_none());
        assert!(cache.source.is_some());
        assert!(cache.begin_guidance(101, 100, 100).is_err());
    }

    #[test]
    fn guidance_completion_is_cancelled_by_options_source_and_conflicting_upload() {
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 110);
        let (_, epoch) = cache.begin_guidance(111, 110, 110).unwrap();
        cache.invalidate(112, Some(110)).unwrap();
        assert!(cache
            .finish(111, 110, epoch, DocumentStep::untracked(vec![1]))
            .is_err());
        install_guidance_base(&mut cache, 120);
        let (_, epoch) = cache.begin_guidance(121, 120, 120).unwrap();
        cache.resolve_duplicate(120, false).unwrap_err();
        assert!(cache
            .finish(121, 120, epoch, DocumentStep::untracked(vec![1]))
            .is_err());
        install_guidance_base(&mut cache, 130);
        let (_, epoch) = cache.begin_guidance(131, 130, 130).unwrap();
        install(&mut cache, 132, &[3]);
        assert!(cache
            .finish(131, 130, epoch, DocumentStep::untracked(vec![1]))
            .is_err());
        assert_eq!(cache.source.as_ref().unwrap().source_id, 132);
        assert!(cache.result.is_none());
    }

    #[test]
    fn guidance_rejects_equal_bytes_from_a_different_source_allocation() {
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 140);
        let source = cache.source.as_mut().unwrap();
        source.bytes = Arc::new(source.bytes.as_ref().clone());
        assert_eq!(
            cache.begin_guidance(141, 140, 140).unwrap_err().code,
            "guidance.provenance_unavailable"
        );
        assert_eq!(cache.revision, 140);
    }

    #[test]
    fn raw_upload_is_idempotent_but_rejects_different_bytes_or_format() {
        let state = EngineState::default();
        upload_bytes(&state, 40, InputFormat::Stl, &[1, 2, 3]).unwrap();
        upload_bytes(&state, 40, InputFormat::Stl, &[1, 2, 3]).unwrap();
        let epoch = {
            let mut cache = state.inner.lock().unwrap();
            let (_, _, epoch) = cache.begin(40, 40).unwrap();
            cache
                .finish(40, 40, epoch, DocumentStep::untracked(vec![9]))
                .unwrap();
            epoch
        };
        assert_eq!(
            upload_bytes(&state, 40, InputFormat::Stl, &[1, 2, 4])
                .unwrap_err()
                .code,
            "engine.source_conflict"
        );
        {
            let mut cache = state.inner.lock().unwrap();
            assert!(cache.result.is_none());
            assert!(cache
                .finish(40, 40, epoch, DocumentStep::untracked(vec![9]))
                .is_err());
        }
        assert_eq!(
            upload_bytes(&state, 40, InputFormat::ThreeMf, &[1, 2, 3])
                .unwrap_err()
                .code,
            "engine.source_conflict"
        );
        state
            .inner
            .lock()
            .unwrap()
            .invalidate(41, Some(41))
            .unwrap();
        assert!(upload_bytes(&state, 40, InputFormat::Stl, &[1, 2, 3]).is_err());
        upload_bytes(&state, 41, InputFormat::ThreeMf, &[7]).unwrap();
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .source
                .as_ref()
                .unwrap()
                .bytes
                .as_slice(),
            &[7]
        );
    }
    #[test]
    fn failed_guidance_preserves_accepted_session_but_recompute_must_revalidate() {
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 200);
        let (old, _) = cache.result_snapshot(200, 200).unwrap();
        let (base, _) = cache.begin_guidance(201, 200, 200).unwrap();
        assert!(base.guided_cylinder(&[0]).is_err());
        assert!(cache.result.is_none());
        assert!(cache.begin_guidance(202, 200, 201).is_err());
        assert!(cache.begin_guidance_session(202, 200, 200).is_err());
        let (session, epoch) = cache.begin_guidance_session(202, 200, 201).unwrap();
        assert!(Arc::ptr_eq(&session, &old));
        assert!(cache.result.is_none());
        let recomputed = session.recompute_guidance().unwrap();
        assert!(recomputed.report.guidance_hints.is_empty());
        cache.finish(202, 200, epoch, recomputed.step).unwrap();
        let (current, _) = cache.result_snapshot(202, 200).unwrap();
        assert!(!Arc::ptr_eq(&current, &old));
        let (session, _) = cache.begin_guidance_session(203, 200, 202).unwrap();
        assert_eq!(
            session.remove_guidance(99).err().unwrap().code,
            "guidance.unknown_indication"
        );
        assert!(cache.result.is_none());
        assert!(Arc::ptr_eq(
            cache.guidance_session.as_ref().unwrap(),
            &current
        ));
        cache.invalidate(204, Some(200)).unwrap();
        assert!(cache.guidance_session.is_none());
        assert!(cache.begin_guidance_session(205, 200, 204).is_err());
    }

    #[test]
    fn retained_guidance_session_cannot_survive_new_options_source_or_conflict() {
        let mut cache = CacheState::default();
        install_guidance_base(&mut cache, 210);
        cache.begin_guidance(211, 210, 210).unwrap();
        let (session, epoch) = cache.begin_guidance_session(212, 210, 211).unwrap();
        cache.invalidate(213, Some(210)).unwrap();
        assert!(cache.guidance_session.is_none());
        assert!(cache
            .finish(212, 210, epoch, session.recompute_guidance().unwrap().step)
            .is_err());
        install_guidance_base(&mut cache, 220);
        cache.begin_guidance(221, 220, 220).unwrap();
        cache.resolve_duplicate(220, false).unwrap_err();
        assert!(cache.guidance_session.is_none());
        assert!(cache.begin_guidance_session(222, 220, 221).is_err());
        install_guidance_base(&mut cache, 230);
        cache.begin_guidance(231, 230, 230).unwrap();
        install(&mut cache, 232, &[1]);
        assert!(cache.guidance_session.is_none());
        assert!(cache.begin_guidance_session(233, 230, 232).is_err());
    }

    #[test]
    fn face_profile_context_rejects_late_selection_and_aba_without_mutating_step() {
        let mut cache = CacheState::default();
        install(&mut cache, 300, &[1]);
        let (_, _, epoch) = cache.begin(300, 300).unwrap();
        cache
            .finish(300, 300, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        cache.set_face_context(300, 300, 1, Some(0)).unwrap();
        let original = cache.face_context(300, 300, 1, 0).unwrap();
        assert!(cache.current_face_context(&original));
        cache.set_face_context(300, 300, 3, Some(1)).unwrap();
        assert!(cache.set_face_context(300, 300, 2, None).is_err());
        assert!(cache.face_context(300, 300, 3, 1).is_ok());
        assert!(!cache.current_face_context(&original));
        cache.set_face_context(300, 300, 4, Some(0)).unwrap();
        assert!(
            !cache.current_face_context(&original),
            "A -> B -> A cannot revive an export"
        );
        let latest = cache.face_context(300, 300, 4, 0).unwrap();
        cache.set_face_context(0, 0, 5, None).unwrap();
        assert!(
            !cache.current_face_context(&latest),
            "leaving result cancels pending save"
        );
        assert_eq!(cache.result.as_ref().unwrap().step.bytes(), &[9]);
    }

    #[test]
    fn face_profile_context_rejects_source_options_and_same_revision_result_changes() {
        for change in 0..3 {
            let mut cache = CacheState::default();
            install(&mut cache, 310, &[1]);
            let (_, _, epoch) = cache.begin(310, 310).unwrap();
            cache
                .finish(310, 310, epoch, DocumentStep::untracked(vec![9]))
                .unwrap();
            cache.set_face_context(310, 310, 1, Some(0)).unwrap();
            let context = cache.face_context(310, 310, 1, 0).unwrap();
            match change {
                0 => {
                    cache.invalidate(311, Some(310)).unwrap();
                }
                1 => {
                    cache.resolve_duplicate(310, false).unwrap_err();
                }
                _ => {
                    cache
                        .finish(310, 310, epoch, DocumentStep::untracked(vec![9]))
                        .unwrap();
                }
            }
            assert!(!cache.current_face_context(&context));
            assert!(cache.face_context(310, 310, 1, 0).is_err());
        }
    }

    #[test]
    fn profile_failure_preserves_validated_step_and_invalid_context_cannot_export() {
        let mut cache = CacheState::default();
        install(&mut cache, 320, &[1]);
        let (_, _, epoch) = cache.begin(320, 320).unwrap();
        cache
            .finish(320, 320, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        cache.set_face_context(320, 320, 1, Some(0)).unwrap();
        let context = cache.face_context(320, 320, 1, 0).unwrap();
        assert!(context.step.planar_face_profile(0).is_err());
        assert!(cache.current_face_context(&context));
        assert!(context.profile.is_none());
        assert_eq!(cache.result.as_ref().unwrap().step.bytes(), &[9]);
        assert!(cache.set_face_context(319, 320, 2, Some(0)).is_err());
        assert!(!cache.current_face_context(&context));
        assert!(cache.face_profile.is_none());
        assert_eq!(cache.result.as_ref().unwrap().step.bytes(), &[9]);
    }
}
