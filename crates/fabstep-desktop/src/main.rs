//! Coque Tauri et accès aux fichiers. Le calcul ne bloque jamais le fil de l'interface.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use fabstep_app::{DocumentPreview, DocumentReport, InputFormat};
use fabstep_core::{FabError, Options};
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
}
struct CachedSource {
    source_id: u64,
    format: InputFormat,
    bytes: Arc<Vec<u8>>,
}
struct CachedStep {
    request_id: u64,
    source_id: u64,
    bytes: Vec<u8>,
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
    fn finish(
        &mut self,
        request_id: u64,
        source_id: u64,
        epoch: u64,
        bytes: Vec<u8>,
    ) -> Result<(), FabError> {
        if !self.current(request_id, source_id, epoch) {
            return Err(stale());
        }
        self.result = Some(CachedStep {
            request_id,
            source_id,
            bytes,
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
    let result = tauri::async_runtime::spawn_blocking(move || {
        fabstep_app::convert_document(&bytes, format, &options)
    })
    .await
    .map_err(|e| FabError::new("engine.task", e.to_string()))??;
    state.inner.lock().map_err(lock_error)?.finish(
        request_id,
        source_id,
        epoch,
        result.step_bytes,
    )?;
    Ok(Response {
        request_id,
        report: result.report,
    })
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
        file.write_all(&result.bytes).map_err(io_error)?;
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
            upload_source,
            preview_document,
            convert_document,
            invalidate_result,
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
        upload_bytes, validate_source_size, CacheState, CachedSource, EngineState, InputFormat,
        MAX_SOURCE_BYTES,
    };
    use std::sync::Arc;

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
        assert!(c.finish(10, 10, epoch, vec![1]).is_err());
        let (snapshot, _, next) = c.begin(11, 10).unwrap();
        c.finish(11, 10, next, vec![2]).unwrap();
        c.invalidate(10, Some(10)).unwrap(); // late invalidation is ignored
        assert_eq!(c.result.as_ref().unwrap().bytes, vec![2]);
        c.invalidate(12, Some(12)).unwrap(); // different file
        assert!(c.source.is_none());
        assert_eq!(snapshot.as_slice(), &[1, 2]); // running task owns old bounded snapshot
        assert!(c.expect_upload(10).is_err());
        assert!(c.begin(12, 10).is_err());
        assert!(c.finish(11, 10, next, vec![3]).is_err());
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
        c.finish(30, 30, epoch, vec![4]).unwrap();
        c.resolve_duplicate(30, true).unwrap();
        assert!(c.result.is_some());
        assert_eq!(
            c.resolve_duplicate(30, false).unwrap_err().code,
            "engine.source_conflict"
        );
        assert!(c.result.is_none());
        assert!(c.finish(30, 30, epoch, vec![5]).is_err());
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
    fn raw_upload_is_idempotent_but_rejects_different_bytes_or_format() {
        let state = EngineState::default();
        upload_bytes(&state, 40, InputFormat::Stl, &[1, 2, 3]).unwrap();
        upload_bytes(&state, 40, InputFormat::Stl, &[1, 2, 3]).unwrap();
        let epoch = {
            let mut cache = state.inner.lock().unwrap();
            let (_, _, epoch) = cache.begin(40, 40).unwrap();
            cache.finish(40, 40, epoch, vec![9]).unwrap();
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
            assert!(cache.finish(40, 40, epoch, vec![9]).is_err());
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
}
