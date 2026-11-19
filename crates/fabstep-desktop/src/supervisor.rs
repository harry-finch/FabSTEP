//! One isolated large-STL job. No native object, client path or STEP enters the
//! desktop cache until the trusted adjacent worker has exited and committed it.
use super::{io_error, lock_error, progress, stale, EngineState, Response, MAX_SOURCE_BYTES};
use fabstep_app::{ConversionCheckpoint, ConversionPhase, DocumentStep, InputFormat};
use fabstep_core::{FabError, Options};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::State;

const REQUEST_LIMIT: usize = 16 * 1024;
const REPORT_LIMIT: usize = 128 * 1024 * 1024;
const STEP_LIMIT: usize = 128 * 1024 * 1024;
const PROGRESS_LIMIT: usize = 1024;
const COMPLETION_LIMIT: usize = 8 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30 * 60);
const POLL: Duration = Duration::from_millis(50);
const MEMORY_LIMIT: u64 = 4 * 1024 * 1024 * 1024;

fn problem(code: &str, message: &str) -> FabError {
    FabError::new(format!("worker.{code}"), message)
}
fn protocol() -> FabError {
    problem(
        "protocol",
        "La réponse du moteur isolé est absente ou incohérente.",
    )
}
fn cancelled() -> FabError {
    problem("cancelled", "La conversion isolée a été annulée.")
}

#[derive(Clone, Copy)]
struct ProcessMemory {
    resident: u64,
    footprint: u64,
}

fn enforce_memory(memory: ProcessMemory) -> Result<(), FabError> {
    if memory.resident.max(memory.footprint) >= MEMORY_LIMIT {
        return Err(problem(
            "memory_limit",
            "Le moteur isolé a atteint la limite de 4 Gio de mémoire.",
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn process_memory(pid: u32) -> Result<ProcessMemory, FabError> {
    // SDK sys/resource.h: RUSAGE_INFO_V0, UUID followed by ten u64 fields.
    // libproc.h: proc_pid_rusage returns 0 on success, -1 on failure.
    #[repr(C)]
    #[derive(Default)]
    struct UsageInfo {
        _uuid: [u8; 16],
        _times_and_counters: [u64; 6],
        resident_size: u64,
        physical_footprint: u64,
        _lifetime: [u64; 2],
    }
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut *mut std::ffi::c_void) -> i32;
    }
    const _: [(); 96] = [(); std::mem::size_of::<UsageInfo>()];
    const _: [(); 8] = [(); std::mem::align_of::<UsageInfo>()];
    const _: [(); 64] = [(); std::mem::offset_of!(UsageInfo, resident_size)];
    const _: [(); 72] = [(); std::mem::offset_of!(UsageInfo, physical_footprint)];
    let unavailable = || {
        problem(
            "monitor_failed",
            "Le suivi mémoire du moteur isolé est indisponible.",
        )
    };
    let pid = i32::try_from(pid).map_err(|_| unavailable())?;
    let mut info = UsageInfo::default();
    // Despite the opaque pointer-to-pointer typedef, the API writes directly
    // into the 96-byte struct, not into a separately allocated pointer slot.
    let status = unsafe { proc_pid_rusage(pid, 0, std::ptr::from_mut(&mut info).cast()) };
    if status != 0 || info.resident_size == 0 || info.physical_footprint == 0 {
        return Err(unavailable());
    }
    Ok(ProcessMemory {
        resident: info.resident_size,
        footprint: info.physical_footprint,
    })
}

#[cfg(not(target_os = "macos"))]
fn process_memory(_pid: u32) -> Result<ProcessMemory, FabError> {
    Err(problem(
        "monitor_failed",
        "Le moteur isolé avec suivi mémoire est qualifié uniquement sur macOS.",
    ))
}

#[derive(Default)]
pub(super) struct WorkerRegistry {
    inner: Arc<RegistryInner>,
}
#[derive(Default)]
struct RegistryInner {
    job: Mutex<Option<Arc<Job>>>,
    shutting_down: AtomicBool,
}
struct Job {
    request_id: u64,
    source_id: u64,
    epoch: u64,
    cancelled: AtomicBool,
    child: Mutex<Option<Child>>,
    done: Mutex<bool>,
    completed: Condvar,
}
struct Lease {
    registry: Arc<RegistryInner>,
    job: Arc<Job>,
}
struct ReapBeforeDirectory(Arc<Job>);
impl Drop for ReapBeforeDirectory {
    fn drop(&mut self) {
        self.0.kill_and_reap();
    }
}

impl WorkerRegistry {
    // Called while holding the cache lock, in the same order as cancellation.
    fn reserve(&self, request_id: u64, source_id: u64, epoch: u64) -> Result<Lease, FabError> {
        let mut slot = self.inner.job.lock().map_err(lock_error)?;
        if self.inner.shutting_down.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        if slot.is_some() {
            return Err(problem("busy", "Un moteur isolé est encore actif."));
        }
        let job = Arc::new(Job {
            request_id,
            source_id,
            epoch,
            cancelled: AtomicBool::new(false),
            child: Mutex::new(None),
            done: Mutex::new(false),
            completed: Condvar::new(),
        });
        *slot = Some(Arc::clone(&job));
        Ok(Lease {
            registry: Arc::clone(&self.inner),
            job,
        })
    }

    fn matching(&self, request_id: u64, source_id: u64) -> Result<Option<Arc<Job>>, FabError> {
        Ok(self
            .inner
            .job
            .lock()
            .map_err(lock_error)?
            .as_ref()
            .filter(|job| job.request_id == request_id && job.source_id == source_id)
            .cloned())
    }

    pub(super) fn shutdown(&self) {
        self.inner.shutting_down.store(true, Ordering::Release);
        let job = self
            .inner
            .job
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(job) = job {
            job.cancel_and_wait();
        }
    }
}

impl Job {
    fn kill_and_reap(&self) {
        let mut slot = self.child.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn cancel_and_wait(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.kill_and_reap();
        let mut done = self.done.lock().unwrap_or_else(|e| e.into_inner());
        while !*done {
            done = self.completed.wait(done).unwrap_or_else(|e| e.into_inner());
        }
    }

    fn check(&self, state: &EngineState) -> Result<(), FabError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        if !state.inner.lock().map_err(lock_error)?.current(
            self.request_id,
            self.source_id,
            self.epoch,
        ) {
            return Err(stale());
        }
        Ok(())
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.job.kill_and_reap();
        let mut slot = self.registry.job.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_some_and(|job| Arc::ptr_eq(job, &self.job)) {
            *slot = None;
        }
        drop(slot);
        *self.job.done.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.job.completed.notify_all();
    }
}

fn validate_binary(bytes: &[u8], format: InputFormat) -> Result<usize, FabError> {
    if format != InputFormat::Stl || bytes.len() < 84 || bytes.len() > MAX_SOURCE_BYTES {
        return Err(problem(
            "input_invalid",
            "Ce parcours attend un STL binaire de 100 001 à 600 000 triangles, limité à 64 Mio.",
        ));
    }
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    if !(100_001..=600_000).contains(&count) || 84_u64 + 50 * count as u64 != bytes.len() as u64 {
        return Err(problem("input_invalid", "Le nombre de triangles ou la longueur du STL binaire ne convient pas au parcours isolé."));
    }
    Ok(count)
}

fn helper_path() -> Result<PathBuf, FabError> {
    let name = if cfg!(windows) {
        "fabstep-worker.exe"
    } else {
        "fabstep-worker"
    };
    let path = std::env::current_exe()
        .map_err(io_error)?
        .with_file_name(name);
    if !path.is_file() {
        return Err(problem("unavailable", "Le moteur isolé fabstep-worker est absent du dossier de l’application. Reconstruisez le paquet de développement."));
    }
    Ok(path)
}

#[derive(Serialize)]
struct Request<'a> {
    version: u32,
    request_id: u64,
    source_id: u64,
    options: &'a Options,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    version: u32,
    request_id: u64,
    source_id: u64,
    status: String,
    error: Option<WorkerError>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerError {
    code: String,
    message: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Analysis,
    SourceSeparation,
    Body,
    Recognition,
    SourceValidation,
    Build,
    AuditTessellation,
    AuditComparison,
    Combine,
    ResultInspection,
    Finish,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerProgress {
    phase: Phase,
    body_index: Option<usize>,
}
impl Phase {
    fn native(self) -> ConversionPhase {
        match self {
            Self::Analysis => ConversionPhase::Analysis,
            Self::SourceSeparation => ConversionPhase::SourceSeparation,
            Self::Body => ConversionPhase::Body,
            Self::Recognition => ConversionPhase::Recognition,
            Self::SourceValidation => ConversionPhase::SourceValidation,
            Self::Build => ConversionPhase::Build,
            Self::AuditTessellation => ConversionPhase::AuditTessellation,
            Self::AuditComparison => ConversionPhase::AuditComparison,
            Self::Combine => ConversionPhase::Combine,
            Self::ResultInspection => ConversionPhase::ResultInspection,
            Self::Finish => ConversionPhase::Finish,
        }
    }
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, FabError> {
    if !fs::symlink_metadata(path)
        .map_err(|_| protocol())?
        .is_file()
    {
        return Err(protocol());
    }
    let file = File::open(path).map_err(io_error)?;
    // Atomic progress replacement may change the path between stat and open.
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err(protocol());
    }
    if metadata.len() > limit as u64 {
        return Err(problem(
            "output_too_large",
            "La réponse du moteur isolé dépasse sa limite.",
        ));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(metadata.len() as usize)
        .map_err(|_| {
            problem(
                "failed",
                "Mémoire insuffisante pour lire la réponse isolée.",
            )
        })?;
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(problem(
            "output_too_large",
            "La réponse du moteur isolé dépasse sa limite.",
        ));
    }
    if bytes.len() as u64 != metadata.len() {
        return Err(protocol());
    }
    Ok(bytes)
}

fn write_input(directory: &Path, bytes: &[u8], request: &Request<'_>) -> Result<(), FabError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
    }
    let encoded = serde_json::to_vec(request).map_err(|_| protocol())?;
    if encoded.len() > REQUEST_LIMIT {
        return Err(protocol());
    }
    for (name, data) in [("source.stl", bytes), ("request.json", encoded.as_slice())] {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))
            .map_err(io_error)?;
        file.write_all(data).map_err(io_error)?;
    }
    Ok(())
}

fn progress_file(directory: &Path, observer: &progress::ProgressObserver) -> Result<(), FabError> {
    let path = directory.join("progress.json");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(protocol()),
        Ok(_) => {}
    }
    let update: WorkerProgress =
        serde_json::from_slice(&read_limited(&path, PROGRESS_LIMIT)?).map_err(|_| protocol())?;
    if update.body_index.is_some_and(|i| i >= 32) {
        return Err(protocol());
    }
    observer.checkpoint(ConversionCheckpoint {
        phase: update.phase.native(),
        body_index: update.body_index,
    })
}

fn wait_for_child(
    lease: &Lease,
    state: &EngineState,
    observer: &progress::ProgressObserver,
    directory: &Path,
    timeout: Duration,
) -> Result<ExitStatus, FabError> {
    wait_for_child_with_monitor(lease, state, observer, directory, timeout, process_memory)
}

fn wait_for_child_with_monitor(
    lease: &Lease,
    state: &EngineState,
    observer: &progress::ProgressObserver,
    directory: &Path,
    timeout: Duration,
    mut monitor: impl FnMut(u32) -> Result<ProcessMemory, FabError>,
) -> Result<ExitStatus, FabError> {
    let started = Instant::now();
    let mut memory_checked = Instant::now();
    loop {
        lease.job.check(state)?;
        if started.elapsed() >= timeout {
            return Err(problem(
                "timeout",
                "La conversion isolée dépasse la limite de trente minutes.",
            ));
        }
        let (status, pid) = {
            let mut slot = lease.job.child.lock().map_err(lock_error)?;
            let child = slot.as_mut().ok_or_else(cancelled)?;
            (child.try_wait().map_err(io_error)?, child.id())
        };
        if let Some(status) = status {
            return Ok(status);
        }
        if memory_checked.elapsed() >= Duration::from_secs(1) {
            match monitor(pid) {
                Ok(memory) => enforce_memory(memory)?,
                Err(error) => {
                    lease.job.check(state)?;
                    // A process can exit between try_wait and the memory query.
                    let mut slot = lease.job.child.lock().map_err(lock_error)?;
                    if let Some(status) = slot
                        .as_mut()
                        .ok_or_else(cancelled)?
                        .try_wait()
                        .map_err(io_error)?
                    {
                        return Ok(status);
                    }
                    return Err(error);
                }
            }
            memory_checked = Instant::now();
        }
        progress_file(directory, observer)?;
        thread::sleep(POLL);
    }
}

fn completion(
    directory: &Path,
    request_id: u64,
    source_id: u64,
    success: bool,
) -> Result<(), FabError> {
    let response: Completion = serde_json::from_slice(&read_limited(
        &directory.join("completion.json"),
        COMPLETION_LIMIT,
    )?)
    .map_err(|_| protocol())?;
    if response.version != 1 || response.request_id != request_id || response.source_id != source_id
    {
        return Err(protocol());
    }
    match (response.status.as_str(), response.error) {
        ("success", None) if success => Ok(()),
        ("error", Some(error)) if !error.code.is_empty() && error.code.len() <= 128 => {
            Err(FabError::new(error.code, error.message))
        }
        _ => Err(protocol()),
    }
}

fn supervise(
    lease: Lease,
    state: Arc<EngineState>,
    observer: progress::ProgressObserver,
    helper: PathBuf,
    bytes: Arc<Vec<u8>>,
    options: Options,
    count: usize,
) -> Result<(Value, Vec<u8>), FabError> {
    let directory = tempfile::tempdir().map_err(io_error)?;
    // Locals drop before the Lease parameter. This extra reaper is declared
    // after TempDir so errors and unwinding stop the process before deletion;
    // Lease marks completion only after both resources have been cleaned up.
    let _reaper = ReapBeforeDirectory(Arc::clone(&lease.job));
    lease.job.check(&state)?;
    write_input(
        directory.path(),
        &bytes,
        &Request {
            version: 1,
            request_id: lease.job.request_id,
            source_id: lease.job.source_id,
            options: &options,
        },
    )?;
    {
        // Cancellation shares this short lock with spawn, closing the window
        // where a cancelled job could register a new child after cancel returns.
        let mut slot = lease.job.child.lock().map_err(lock_error)?;
        lease.job.check(&state)?;
        *slot = Some(
            Command::new(helper)
                .arg("--job-dir")
                .arg(directory.path())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|_| problem("unavailable", "Le moteur isolé ne peut pas être lancé."))?,
        );
    }
    let status = wait_for_child(&lease, &state, &observer, directory.path(), TIMEOUT)?;
    lease.job.check(&state)?;
    completion(
        directory.path(),
        lease.job.request_id,
        lease.job.source_id,
        status.success(),
    )?;
    let encoded = read_limited(&directory.path().join("report.json"), REPORT_LIMIT)?;
    let report: Value = serde_json::from_slice(&encoded).map_err(|_| protocol())?;
    drop(encoded);
    let bodies = report
        .get("bodies")
        .and_then(Value::as_array)
        .ok_or_else(protocol)?;
    let total: Option<u64> = bodies.iter().try_fold(0_u64, |sum, body| {
        sum.checked_add(body.pointer("/analysis/report/input_triangles")?.as_u64()?)
    });
    if bodies.is_empty()
        || bodies.len() > 32
        || total != Some(count as u64)
        || report.pointer("/source/format").and_then(Value::as_str) != Some("stl")
        || !report.get("settings").is_some_and(Value::is_object)
        || !report
            .pointer("/cad/faces")
            .and_then(Value::as_u64)
            .is_some_and(|n| (1..=100_000).contains(&n))
        || report.pointer("/cad/solids").and_then(Value::as_u64) != Some(bodies.len() as u64)
        || !report.get("diagnostics").is_some_and(Value::is_array)
        || !report.get("guidance_hints").is_some_and(Value::is_array)
    {
        return Err(protocol());
    }
    lease.job.check(&state)?;
    let step = read_limited(&directory.path().join("result.step"), STEP_LIMIT)?;
    if !step.starts_with(b"ISO-10303-21;") {
        return Err(protocol());
    }
    lease.job.check(&state)?;
    Ok((report, step))
}

#[tauri::command]
pub(super) async fn convert_large_document(
    request_id: u64,
    source_id: u64,
    options: Options,
    confirmed: bool,
    state: State<'_, Arc<EngineState>>,
) -> Result<Response<Value>, FabError> {
    if !confirmed {
        return Err(problem(
            "confirmation_required",
            "Confirmez explicitement l’essai du STL volumineux.",
        ));
    }
    let state = Arc::clone(state.inner());
    if !cfg!(target_os = "macos") {
        return Err(problem(
            "monitor_failed",
            "Le moteur isolé avec suivi mémoire est qualifié uniquement sur macOS.",
        ));
    }
    let (bytes, epoch, count, lease) = {
        let mut cache = state.inner.lock().map_err(lock_error)?;
        if cache.cancelled_large == Some((request_id, source_id)) {
            return Err(cancelled());
        }
        let (bytes, format, epoch) = cache.begin(request_id, source_id)?;
        let count = validate_binary(&bytes, format)?;
        let lease = state.large_worker.reserve(request_id, source_id, epoch)?;
        (bytes, epoch, count, lease)
    };
    let guard = progress::ConversionGuard::new(Arc::clone(&state), request_id, source_id, epoch)?;
    let helper = helper_path()?;
    let observer = guard.observer();
    let worker_state = Arc::clone(&state);
    let (report, step) = tauri::async_runtime::spawn_blocking(move || {
        supervise(lease, worker_state, observer, helper, bytes, options, count)
    })
    .await
    .map_err(|_| problem("failed", "Le superviseur isolé a échoué."))??;
    state.inner.lock().map_err(lock_error)?.finish(
        request_id,
        source_id,
        epoch,
        DocumentStep::untracked(step),
    )?;
    Ok(Response { request_id, report })
}

fn cancel(state: &EngineState, request_id: u64, source_id: u64) -> Result<(), FabError> {
    if source_id == 0 || source_id > request_id {
        return Err(problem(
            "input_invalid",
            "Identité de conversion isolée invalide.",
        ));
    }
    let job = {
        let mut cache = state.inner.lock().map_err(lock_error)?;
        if request_id >= cache.revision {
            cache.invalidate(request_id, Some(source_id))?;
            cache.cancelled_large = Some((request_id, source_id));
            cache.epoch = cache.epoch.wrapping_add(1);
            cache.result = None;
            cache.guidance_session = None;
            cache.face_profile = None;
            cache.section = None;
            cache.source_section = None;
            cache.progress = None;
        }
        state.large_worker.matching(request_id, source_id)?
    };
    if let Some(job) = job {
        job.cancel_and_wait();
    }
    Ok(())
}

#[tauri::command]
pub(super) async fn cancel_large_conversion(
    request_id: u64,
    source_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<(), FabError> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || cancel(&state, request_id, source_id))
        .await
        .map_err(|_| problem("failed", "L’arrêt du moteur isolé a échoué."))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CachedSource;

    fn installed() -> Arc<EngineState> {
        let state = Arc::new(EngineState::default());
        {
            let mut cache = state.inner.lock().unwrap();
            cache.expect_upload(3).unwrap();
            cache.source = Some(CachedSource {
                source_id: 3,
                format: InputFormat::Stl,
                bytes: Arc::new(vec![1]),
            });
        }
        state
    }

    fn reservation(state: &Arc<EngineState>) -> (Lease, progress::ConversionGuard) {
        let epoch = state.inner.lock().unwrap().begin(7, 3).unwrap().2;
        let lease = state.large_worker.reserve(7, 3, epoch).unwrap();
        let guard = progress::ConversionGuard::new(Arc::clone(state), 7, 3, epoch).unwrap();
        (lease, guard)
    }

    #[test]
    fn large_binary_admission_requires_exact_length_and_separate_source_range() {
        assert_eq!(fabstep_core::SOURCE_MAX_TRIANGLES, 100_000);
        assert_eq!(fabstep_core::SOURCE_MAX_VERTICES, 300_000);
        for count in [100_000_u32, 600_001] {
            let mut bytes = vec![0; 84];
            bytes[80..84].copy_from_slice(&count.to_le_bytes());
            assert_eq!(
                validate_binary(&bytes, InputFormat::Stl).unwrap_err().code,
                "worker.input_invalid"
            );
        }
        let count = 100_001_u32;
        let mut bytes = vec![0; 84 + count as usize * 50];
        bytes[80..84].copy_from_slice(&count.to_le_bytes());
        assert_eq!(
            validate_binary(&bytes, InputFormat::Stl).unwrap(),
            count as usize
        );
        assert!(validate_binary(&bytes, InputFormat::ThreeMf).is_err());
        bytes.pop();
        assert!(validate_binary(&bytes, InputFormat::Stl).is_err());
    }

    #[test]
    fn cancelling_before_registration_tombstones_identity_without_touching_newer_work() {
        let state = installed();
        cancel(&state, 7, 3).unwrap();
        {
            let cache = state.inner.lock().unwrap();
            assert_eq!(cache.cancelled_large, Some((7, 3)));
            assert!(cache.result.is_none());
        }
        state.inner.lock().unwrap().invalidate(8, Some(3)).unwrap();
        let epoch = state.inner.lock().unwrap().begin(8, 3).unwrap().2;
        let lease = state.large_worker.reserve(8, 3, epoch).unwrap();
        cancel(&state, 7, 3).unwrap();
        assert!(!lease.job.cancelled.load(Ordering::Acquire));
        assert!(state.inner.lock().unwrap().current(8, 3, epoch));
    }

    #[test]
    fn completion_requires_identity_success_and_bounded_regular_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("completion.json");
        fs::write(
            &file,
            br#"{"version":1,"request_id":7,"source_id":3,"status":"success"}"#,
        )
        .unwrap();
        completion(dir.path(), 7, 3, true).unwrap();
        assert_eq!(
            completion(dir.path(), 8, 3, true).unwrap_err().code,
            "worker.protocol"
        );
        assert_eq!(
            completion(dir.path(), 7, 3, false).unwrap_err().code,
            "worker.protocol"
        );
        fs::write(&file, br#"{"version":1,"request_id":7,"source_id":3,"status":"error","error":{"code":"planar.too_complex","message":"refused"}}"#).unwrap();
        assert_eq!(
            completion(dir.path(), 7, 3, false).unwrap_err().code,
            "planar.too_complex"
        );
        fs::write(&file, br#"{"version":1,"request_id":0,"source_id":0,"status":"error","error":{"code":"worker.request_invalid","message":"invalid"}}"#).unwrap();
        assert_eq!(
            completion(dir.path(), 7, 3, false).unwrap_err().code,
            "worker.protocol"
        );
        File::create(&file)
            .unwrap()
            .set_len((COMPLETION_LIMIT + 1) as u64)
            .unwrap();
        assert_eq!(
            read_limited(&file, COMPLETION_LIMIT).unwrap_err().code,
            "worker.output_too_large"
        );
        fs::remove_file(&file).unwrap();
        fs::create_dir(&file).unwrap();
        assert_eq!(
            read_limited(&file, COMPLETION_LIMIT).unwrap_err().code,
            "worker.protocol"
        );
    }

    #[test]
    fn progress_accepts_only_real_phases_and_bounded_body_indices() {
        let state = installed();
        let (_lease, guard) = reservation(&state);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.json");
        for invalid in [
            br#"{"phase":"guessed","body_index":0}"#.as_slice(),
            br#"{"phase":"build","body_index":32}"#.as_slice(),
        ] {
            fs::write(&path, invalid).unwrap();
            assert_eq!(
                progress_file(dir.path(), &guard.observer())
                    .unwrap_err()
                    .code,
                "worker.protocol"
            );
        }
        fs::write(&path, br#"{"phase":"source_validation","body_index":0}"#).unwrap();
        progress_file(dir.path(), &guard.observer()).unwrap();
        assert!(state
            .inner
            .lock()
            .unwrap()
            .conversion_progress(7, 3)
            .is_some());
        File::create(&path)
            .unwrap()
            .set_len((PROGRESS_LIMIT + 1) as u64)
            .unwrap();
        assert_eq!(
            progress_file(dir.path(), &guard.observer())
                .unwrap_err()
                .code,
            "worker.output_too_large"
        );
    }

    #[test]
    fn atomic_progress_replacement_does_not_compare_different_inode_lengths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.json");
        fs::write(&path, b"short").unwrap();
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            for index in 0..500 {
                let tmp = writer_path.with_extension("next");
                fs::write(
                    &tmp,
                    if index % 2 == 0 {
                        b"short".as_slice()
                    } else {
                        b"a much longer complete snapshot".as_slice()
                    },
                )
                .unwrap();
                fs::rename(tmp, &writer_path).unwrap();
            }
        });
        for _ in 0..500 {
            let data = read_limited(&path, PROGRESS_LIMIT).unwrap();
            assert!(data == b"short" || data == b"a much longer complete snapshot");
        }
        writer.join().unwrap();
    }

    #[test]
    fn memory_guard_rejects_the_limit_without_allocating_gigabytes() {
        enforce_memory(ProcessMemory {
            resident: MEMORY_LIMIT - 1,
            footprint: MEMORY_LIMIT - 1,
        })
        .unwrap();
        for memory in [
            ProcessMemory {
                resident: 1,
                footprint: MEMORY_LIMIT,
            },
            ProcessMemory {
                resident: MEMORY_LIMIT,
                footprint: 1,
            },
            ProcessMemory {
                resident: 1,
                footprint: MEMORY_LIMIT + 1,
            },
        ] {
            assert_eq!(
                enforce_memory(memory).unwrap_err().code,
                "worker.memory_limit"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_memory_monitor_reads_its_own_task_and_rejects_invalid_pid() {
        let memory = process_memory(std::process::id()).unwrap();
        assert!(memory.resident > 0 && memory.footprint > 0);
        for pid in [u32::MAX, i32::MAX as u32] {
            assert_eq!(
                process_memory(pid).err().unwrap().code,
                "worker.monitor_failed"
            );
        }
        let state = installed();
        let (lease, _guard) = reservation(&state);
        let pid = sleeping_child(&lease);
        let memory = process_memory(pid).unwrap();
        assert!(memory.resident > 0 && memory.footprint > 0);
        drop(lease);
        assert_reaped(pid);
    }

    #[cfg(unix)]
    fn sleeping_child(lease: &Lease) -> u32 {
        let child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        *lease.job.child.lock().unwrap() = Some(child);
        pid
    }

    #[cfg(unix)]
    fn assert_reaped(pid: u32) {
        assert!(!Command::new("/bin/kill")
            .arg("-0")
            .arg(pid.to_string())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
    }

    #[cfg(unix)]
    #[test]
    fn memory_limit_and_monitor_failure_reap_child_without_publishing() {
        for case in 0..3 {
            let state = installed();
            let (lease, guard) = reservation(&state);
            let pid = sleeping_child(&lease);
            let dir = tempfile::tempdir().unwrap();
            let error = {
                let _reaper = ReapBeforeDirectory(Arc::clone(&lease.job));
                wait_for_child_with_monitor(
                    &lease,
                    &state,
                    &guard.observer(),
                    dir.path(),
                    TIMEOUT,
                    |observed_pid| {
                        assert_eq!(observed_pid, pid);
                        match case {
                            0 => Ok(ProcessMemory {
                                resident: 1,
                                footprint: MEMORY_LIMIT,
                            }),
                            1 => Ok(ProcessMemory {
                                resident: MEMORY_LIMIT,
                                footprint: 1,
                            }),
                            _ => Err(problem("monitor_failed", "test")),
                        }
                    },
                )
                .unwrap_err()
            };
            assert_eq!(
                error.code,
                if case == 2 {
                    "worker.monitor_failed"
                } else {
                    "worker.memory_limit"
                }
            );
            assert_reaped(pid);
            assert!(dir.path().exists());
            drop(lease);
            assert!(state.large_worker.inner.job.lock().unwrap().is_none());
            assert!(state.inner.lock().unwrap().result.is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_reaps_a_real_child_before_return_without_publishing() {
        let state = installed();
        let (lease, guard) = reservation(&state);
        let pid = sleeping_child(&lease);
        let worker_state = Arc::clone(&state);
        let worker = thread::spawn(move || {
            let dir = tempfile::tempdir().unwrap();
            let _reaper = ReapBeforeDirectory(Arc::clone(&lease.job));
            wait_for_child(
                &lease,
                &worker_state,
                &guard.observer(),
                dir.path(),
                TIMEOUT,
            )
        });
        cancel(&state, 7, 3).unwrap();
        assert_eq!(worker.join().unwrap().unwrap_err().code, "worker.cancelled");
        assert_reaped(pid);
        let cache = state.inner.lock().unwrap();
        assert!(cache.result.is_none());
        assert!(cache.conversion_progress(7, 3).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn invalidation_and_timeout_reap_children_and_release_the_slot() {
        for timeout in [false, true] {
            let state = installed();
            let (lease, guard) = reservation(&state);
            let pid = sleeping_child(&lease);
            if !timeout {
                state.inner.lock().unwrap().invalidate(8, Some(3)).unwrap();
            }
            let dir = tempfile::tempdir().unwrap();
            let error = wait_for_child(
                &lease,
                &state,
                &guard.observer(),
                dir.path(),
                if timeout { Duration::ZERO } else { TIMEOUT },
            )
            .unwrap_err();
            assert_eq!(
                error.code,
                if timeout {
                    "worker.timeout"
                } else {
                    "engine.stale"
                }
            );
            drop(lease);
            assert_reaped(pid);
            assert!(state.large_worker.inner.job.lock().unwrap().is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn reaper_runs_before_private_directory_cleanup_even_during_unwind() {
        struct DirectoryCheck {
            directory: tempfile::TempDir,
            job: Arc<Job>,
        }
        impl Drop for DirectoryCheck {
            fn drop(&mut self) {
                assert!(self.directory.path().exists());
                assert!(self.job.child.lock().unwrap().is_none());
            }
        }
        let state = installed();
        let (lease, _guard) = reservation(&state);
        let pid = sleeping_child(&lease);
        let job = Arc::clone(&lease.job);
        let unwind = std::panic::catch_unwind(move || {
            let _directory = DirectoryCheck {
                directory: tempfile::tempdir().unwrap(),
                job: Arc::clone(&job),
            };
            let _reaper = ReapBeforeDirectory(job);
            panic!("simulated supervisor failure");
        });
        assert!(unwind.is_err());
        assert_reaped(pid);
    }

    #[cfg(unix)]
    #[test]
    fn application_shutdown_reaps_the_current_child() {
        let state = installed();
        let (lease, guard) = reservation(&state);
        let pid = sleeping_child(&lease);
        let worker_state = Arc::clone(&state);
        let worker = thread::spawn(move || {
            let dir = tempfile::tempdir().unwrap();
            let _reaper = ReapBeforeDirectory(Arc::clone(&lease.job));
            wait_for_child(
                &lease,
                &worker_state,
                &guard.observer(),
                dir.path(),
                TIMEOUT,
            )
        });
        state.large_worker.shutdown();
        assert_eq!(worker.join().unwrap().unwrap_err().code, "worker.cancelled");
        assert_reaped(pid);
        assert!(state.large_worker.reserve(8, 3, 0).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "set FABSTEP_TEST_WORKER to the absolute path of the built isolated worker"]
    fn real_worker_handoff_publishes_only_after_validated_report_and_step() {
        let helper = PathBuf::from(
            std::env::var_os("FABSTEP_TEST_WORKER")
                .expect("FABSTEP_TEST_WORKER must name the real worker explicitly"),
        );
        assert!(helper.is_absolute() && helper.is_file());
        // The command's >100k admission is tested independently. A small real
        // cube keeps this process/protocol/parent-publication test inexpensive.
        let bytes = Arc::new(include_bytes!("../../../fixtures/cube.stl").to_vec());
        let state = Arc::new(EngineState::default());
        let (epoch, lease) = {
            let mut cache = state.inner.lock().unwrap();
            cache.expect_upload(3).unwrap();
            cache.source = Some(CachedSource {
                source_id: 3,
                format: InputFormat::Stl,
                bytes: Arc::clone(&bytes),
            });
            let epoch = cache.begin(7, 3).unwrap().2;
            (epoch, state.large_worker.reserve(7, 3, epoch).unwrap())
        };
        let job = Arc::clone(&lease.job);
        let guard = progress::ConversionGuard::new(Arc::clone(&state), 7, 3, epoch).unwrap();
        let (report, step) = supervise(
            lease,
            Arc::clone(&state),
            guard.observer(),
            helper,
            Arc::clone(&bytes),
            Options::default(),
            12,
        )
        .unwrap();
        assert_eq!(report["cad"]["solids"], 1);
        assert_eq!(report["cad"]["faces"], 6);
        assert_eq!(report["cad"]["planar_faces"], 6);
        assert!((report["cad"]["volume_mm3"].as_f64().unwrap() - 1.).abs() < 1e-6);
        assert_eq!(
            report["bodies"][0]["analysis"]["report"]["input_triangles"],
            12
        );
        assert!(step.starts_with(b"ISO-10303-21;"));
        assert!(*job.done.lock().unwrap());
        assert!(job.child.lock().unwrap().is_none());
        assert!(state.large_worker.inner.job.lock().unwrap().is_none());
        let cached = {
            let mut cache = state.inner.lock().unwrap();
            assert!(
                cache.result.is_none(),
                "supervision alone cannot publish a STEP"
            );
            cache
                .finish(7, 3, epoch, DocumentStep::untracked(step))
                .unwrap();
            cache.result_snapshot(7, 3).unwrap().0
        };
        drop(guard);
        assert!(state
            .inner
            .lock()
            .unwrap()
            .conversion_progress(7, 3)
            .is_none());
        assert!(!cached.is_guidance_source(&bytes, InputFormat::Stl));
        let inspected = cached.result_faces().unwrap();
        assert_eq!(inspected.result.faces.len(), 6);
        assert_ne!(inspected.source_correspondence.status, "exact");
    }
}
