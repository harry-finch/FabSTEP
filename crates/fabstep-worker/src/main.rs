//! Private one-job process: the ordinary validated application pipeline runs
//! here with large-STL limits, never inside the desktop process.
use fabstep_app::isolated_report::{
    CompactWorkerReport, COMPACT_REPORT_LIMIT, COMPACT_REPORT_VERSION,
};
use fabstep_app::{
    convert_document_controlled, ConversionCheckpoint, ConversionPhase, InputFormat,
};
use fabstep_core::{FabError, Options};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const MIB: usize = 1024 * 1024;
const SOURCE_LIMIT: usize = 64 * MIB;
const REQUEST_LIMIT: usize = 16 * 1024;
const LEGACY_REPORT_LIMIT: usize = 128 * MIB;
const STEP_LIMIT: usize = 128 * MIB;
const PROGRESS_LIMIT: usize = 1024;
const COMPLETION_LIMIT: usize = 8 * 1024;

#[derive(Clone, Copy, Default)]
#[repr(u32)]
enum ProtocolVersion {
    #[default]
    Legacy = 1,
    Compact = COMPACT_REPORT_VERSION,
}

#[derive(Clone, Copy, Default)]
struct Identity {
    protocol: ProtocolVersion,
    request_id: u64,
    source_id: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    request_id: u64,
    source_id: u64,
    options: Options,
}

#[derive(Serialize)]
struct Progress {
    phase: &'static str,
    body_index: Option<usize>,
}

#[derive(Serialize)]
struct Completion {
    version: u32,
    request_id: u64,
    source_id: u64,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<CompletionError>,
}

#[derive(Serialize)]
struct CompletionError {
    code: String,
    message: String,
}

fn io_error(error: impl std::fmt::Display) -> FabError {
    FabError::new(
        "worker.io",
        format!("Échange avec le moteur isolé impossible : {error}"),
    )
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

/// Metadata and streaming reads both enforce the ceiling. A private input is
/// immutable once the parent launches this process; growth/shrinkage is refused.
fn read_limited(path: &Path, limit: usize, too_large: &'static str) -> Result<Vec<u8>, FabError> {
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if !metadata.is_file() {
        return Err(FabError::new(
            "worker.invalid_input",
            "L'entrée doit être un fichier ordinaire.",
        ));
    }
    if metadata.len() > limit as u64 {
        return Err(FabError::new(
            too_large,
            "Le fichier dépasse la limite du moteur isolé.",
        ));
    }
    let expected = metadata.len() as usize;
    let mut file = File::open(path).map_err(io_error)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(expected).map_err(|_| {
        FabError::new(
            "worker.allocation",
            "Allocation du fichier d'entrée impossible.",
        )
    })?;
    let mut buffer = [0_u8; 8192];
    loop {
        let available = expected
            .saturating_sub(bytes.len())
            .saturating_add(1)
            .min(buffer.len());
        let n = file.read(&mut buffer[..available]).map_err(io_error)?;
        if n == 0 {
            break;
        }
        if n > limit.saturating_sub(bytes.len()) {
            return Err(FabError::new(
                too_large,
                "Le fichier dépasse la limite du moteur isolé.",
            ));
        }
        if n > expected.saturating_sub(bytes.len()) {
            return Err(FabError::new(
                "worker.input_changed",
                "Le fichier d'entrée a changé pendant sa lecture.",
            ));
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
    if bytes.len() != expected {
        return Err(FabError::new(
            "worker.input_changed",
            "Le fichier d'entrée a changé pendant sa lecture.",
        ));
    }
    Ok(bytes)
}

struct LimitedWriter {
    file: File,
    remaining: usize,
    exceeded: bool,
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            self.exceeded = true;
            return Err(io::Error::other("worker output byte limit"));
        }
        let n = self.file.write(bytes)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Durable outputs are synced before publication. Progress lives only for the job.
/// Renames stay in the private job directory, on the same filesystem.
fn atomic_publish(
    directory: &Path,
    name: &str,
    limit: usize,
    write: impl FnOnce(&mut LimitedWriter) -> Result<(), FabError>,
) -> Result<(), FabError> {
    let temporary_path = directory.join(format!(".{name}.tmp-{}", std::process::id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(io_error)?;
    let temporary = Temporary(temporary_path);
    let mut writer = LimitedWriter {
        file,
        remaining: limit,
        exceeded: false,
    };
    let result = write(&mut writer);
    if writer.exceeded {
        return Err(FabError::new(
            "worker.output_too_large",
            format!("La sortie {name} dépasse sa limite."),
        ));
    }
    result?;
    if name != "progress.json" {
        writer.file.sync_all().map_err(io_error)?;
    }
    drop(writer);
    fs::rename(&temporary.0, directory.join(name)).map_err(io_error)
}

fn atomic_json<T: Serialize>(
    directory: &Path,
    name: &str,
    limit: usize,
    value: &T,
) -> Result<(), FabError> {
    atomic_publish(directory, name, limit, |writer| {
        let mut buffered = BufWriter::with_capacity(limit.min(64 * 1024), writer);
        serde_json::to_writer(&mut buffered, value)
            .map_err(|error| FabError::new("worker.serialization", error.to_string()))?;
        buffered.flush().map_err(io_error)
    })
}

fn atomic_bytes(directory: &Path, name: &str, limit: usize, bytes: &[u8]) -> Result<(), FabError> {
    atomic_publish(directory, name, limit, |writer| {
        writer.write_all(bytes).map_err(io_error)
    })
}

struct Job {
    directory: PathBuf,
    _lock: File,
}
impl Job {
    fn acquire(directory: &Path) -> Result<Self, FabError> {
        let metadata = fs::symlink_metadata(directory).map_err(io_error)?;
        if !metadata.is_dir() {
            return Err(FabError::new(
                "worker.invalid_job",
                "Le dossier privé du moteur est invalide.",
            ));
        }
        let lock_path = directory.join(".worker.lock");
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
            .map_err(io_error)?;
        let job = Self {
            directory: directory.to_owned(),
            _lock: lock,
        };
        for name in [
            "progress.json",
            "report.json",
            "result.step",
            "completion.json",
        ] {
            match fs::symlink_metadata(directory.join(name)) {
                Ok(_) => {
                    return Err(FabError::new(
                        "worker.job_not_fresh",
                        "Le dossier privé contient déjà une sortie.",
                    ))
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(io_error(error)),
            }
        }
        Ok(job)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.directory.join(".worker.lock"));
    }
}

fn read_request(directory: &Path, identity: &mut Identity) -> Result<Request, FabError> {
    let bytes = read_limited(
        &directory.join("request.json"),
        REQUEST_LIMIT,
        "worker.request_too_large",
    )?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
        FabError::new(
            "worker.request_invalid",
            "La requête privée n'est pas un document JSON valide.",
        )
    })?;
    identity.request_id = value
        .get("request_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    identity.source_id = value.get("source_id").and_then(|v| v.as_u64()).unwrap_or(0);
    if value.get("version").and_then(|v| v.as_u64()) == Some(u64::from(COMPACT_REPORT_VERSION)) {
        identity.protocol = ProtocolVersion::Compact;
    }
    let request: Request = serde_json::from_value(value).map_err(|_| {
        FabError::new(
            "worker.request_invalid",
            "La requête privée du moteur est invalide.",
        )
    })?;
    if request.version != identity.protocol as u32
        || request.request_id == 0
        || request.source_id == 0
        || request.source_id > request.request_id
    {
        return Err(FabError::new(
            "worker.request_invalid",
            "Version ou identité de requête invalide.",
        ));
    }
    Ok(request)
}

fn completion(identity: Identity, error: Option<&FabError>) -> Completion {
    fn prefix(value: &str, maximum: usize) -> String {
        let mut end = value.len().min(maximum);
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value[..end].to_owned()
    }
    Completion {
        version: identity.protocol as u32,
        request_id: identity.request_id,
        source_id: identity.source_id,
        status: if error.is_some() { "error" } else { "success" },
        error: error.map(|error| CompletionError {
            code: if error.code.len() <= 128 {
                error.code.clone()
            } else {
                "worker.error".into()
            },
            // Even maximum JSON escaping (six bytes per character) remains
            // below the 8 KiB completion ceiling; no giant error is copied.
            message: prefix(&error.message, 1000),
        }),
    }
}

struct Payloads<'a> {
    directory: &'a Path,
    report: bool,
    step: bool,
    committed: bool,
}
impl Drop for Payloads<'_> {
    fn drop(&mut self) {
        if !self.committed {
            if self.report {
                let _ = fs::remove_file(self.directory.join("report.json"));
            }
            if self.step {
                let _ = fs::remove_file(self.directory.join("result.step"));
            }
        }
    }
}

fn publish_success<T: Serialize>(
    directory: &Path,
    identity: Identity,
    report: &T,
    step: &[u8],
) -> Result<(), FabError> {
    let mut payloads = Payloads {
        directory,
        report: false,
        step: false,
        committed: false,
    };
    let report_limit = match identity.protocol {
        ProtocolVersion::Legacy => LEGACY_REPORT_LIMIT,
        ProtocolVersion::Compact => COMPACT_REPORT_LIMIT,
    };
    atomic_json(directory, "report.json", report_limit, report)?;
    payloads.report = true;
    atomic_bytes(directory, "result.step", STEP_LIMIT, step)?;
    payloads.step = true;
    // The parent may accept payloads only after this final atomic publication.
    atomic_json(
        directory,
        "completion.json",
        COMPLETION_LIMIT,
        &completion(identity, None),
    )?;
    payloads.committed = true;
    Ok(())
}

fn execute_job(directory: &Path) -> Result<bool, FabError> {
    let _job = Job::acquire(directory)?;
    let mut identity = Identity::default();
    let result = (|| {
        let mut request = read_request(directory, &mut identity)?;
        // The isolated large-STL job has a fixed time and memory budget,
        // including when it contains several individually small bodies.
        request.options.auto_tune = false;
        let source = read_limited(
            &directory.join("source.stl"),
            SOURCE_LIMIT,
            "worker.source_too_large",
        )?;
        let converted = convert_document_controlled(
            &source,
            InputFormat::Stl,
            &request.options,
            &mut |checkpoint: ConversionCheckpoint| {
                atomic_json(
                    directory,
                    "progress.json",
                    PROGRESS_LIMIT,
                    &Progress {
                        phase: phase_name(checkpoint.phase),
                        body_index: checkpoint.body_index,
                    },
                )
            },
        )?;
        drop(source);
        let step = &converted.step_bytes;
        match identity.protocol {
            ProtocolVersion::Legacy => {
                publish_success(directory, identity, &converted.report, step)
            }
            ProtocolVersion::Compact => {
                let report = CompactWorkerReport::from(&converted.report);
                drop(converted.report);
                publish_success(directory, identity, &report, step)
            }
        }
    })();
    match result {
        Ok(()) => Ok(true),
        Err(error) => {
            atomic_json(
                directory,
                "completion.json",
                COMPLETION_LIMIT,
                &completion(identity, Some(&error)),
            )?;
            Ok(false)
        }
    }
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let directory = match (arguments.next(), arguments.next(), arguments.next()) {
        (Some(flag), Some(path), None) if flag == "--job-dir" => PathBuf::from(path),
        _ => {
            eprintln!("Usage: fabstep-worker --job-dir PATH");
            std::process::exit(2);
        }
    };
    // SAFETY: this executable handles one job synchronously. No OCCT work has started.
    unsafe { fabstep_app::prepare_isolated_atlas_checks() };
    match execute_job(&directory) {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct TestJob(PathBuf);
    impl TestJob {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "fabstep-worker-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }
        fn request(&self) {
            self.request_with_options(1, &Options::default());
        }
        fn request_with_options(&self, version: u32, options: &Options) {
            let request = serde_json::json!({
                "version": version,
                "request_id": 7,
                "source_id": 3,
                "options": options,
            });
            fs::write(
                self.0.join("request.json"),
                serde_json::to_vec(&request).unwrap(),
            )
            .unwrap();
        }
        fn finished(&self) -> serde_json::Value {
            serde_json::from_slice(&fs::read(self.0.join("completion.json")).unwrap()).unwrap()
        }
    }
    impl Drop for TestJob {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn malformed_request_publishes_only_a_bounded_error() {
        let job = TestJob::new();
        fs::write(job.0.join("request.json"), b"{").unwrap();
        assert!(!execute_job(&job.0).unwrap());
        let result = job.finished();
        assert_eq!(result["status"], "error");
        assert_eq!(result["request_id"], 0);
        assert_eq!(result["error"]["code"], "worker.request_invalid");
        assert!(!job.0.join("result.step").exists());
        assert!(!job.0.join("report.json").exists());
    }

    #[test]
    fn malformed_options_preserve_the_request_identity() {
        for version in [1, COMPACT_REPORT_VERSION] {
            let job = TestJob::new();
            let request = serde_json::json!({
                "version": version,
                "request_id": 7,
                "source_id": 3,
                "options": {"input_unit": "unknown"},
            });
            fs::write(
                job.0.join("request.json"),
                serde_json::to_vec(&request).unwrap(),
            )
            .unwrap();
            assert!(!execute_job(&job.0).unwrap());
            let result = job.finished();
            assert_eq!(result["version"], version);
            assert_eq!(result["request_id"], 7);
            assert_eq!(result["source_id"], 3);
            assert_eq!(result["error"]["code"], "worker.request_invalid");
        }
    }

    #[test]
    fn an_unknown_protocol_returns_a_structured_error_before_conversion() {
        let job = TestJob::new();
        job.request_with_options(99, &Options::default());
        assert!(!execute_job(&job.0).unwrap());
        let result = job.finished();
        assert_eq!(result["version"], 1);
        assert_eq!(result["request_id"], 7);
        assert_eq!(result["source_id"], 3);
        assert_eq!(result["status"], "error");
        assert_eq!(result["error"]["code"], "worker.request_invalid");
        for file in ["progress.json", "report.json", "result.step"] {
            assert!(!job.0.join(file).exists());
        }
    }

    #[test]
    fn oversize_request_and_source_are_refused_before_conversion() {
        let job = TestJob::new();
        File::create(job.0.join("request.json"))
            .unwrap()
            .set_len(REQUEST_LIMIT as u64 + 1)
            .unwrap();
        assert!(!execute_job(&job.0).unwrap());
        assert_eq!(job.finished()["error"]["code"], "worker.request_too_large");
        let job = TestJob::new();
        job.request();
        File::create(job.0.join("source.stl"))
            .unwrap()
            .set_len(SOURCE_LIMIT as u64 + 1)
            .unwrap();
        assert!(!execute_job(&job.0).unwrap());
        let result = job.finished();
        assert_eq!(result["error"]["code"], "worker.source_too_large");
        assert_eq!(result["request_id"], 7);
        assert!(!job.0.join("progress.json").exists());
    }

    #[test]
    fn failed_atomic_write_preserves_the_previous_snapshot_and_removes_its_temp() {
        let job = TestJob::new();
        fs::write(job.0.join("progress.json"), b"previous").unwrap();
        assert_eq!(
            atomic_bytes(&job.0, "progress.json", 3, b"too large")
                .unwrap_err()
                .code,
            "worker.output_too_large"
        );
        assert_eq!(fs::read(job.0.join("progress.json")).unwrap(), b"previous");
        atomic_bytes(&job.0, "progress.json", 4, b"next").unwrap();
        assert_eq!(fs::read(job.0.join("progress.json")).unwrap(), b"next");
        assert_eq!(fs::read_dir(&job.0).unwrap().count(), 1);
    }

    #[test]
    fn failed_completion_removes_both_payloads() {
        for protocol in [ProtocolVersion::Legacy, ProtocolVersion::Compact] {
            let job = TestJob::new();
            fs::create_dir(job.0.join("completion.json")).unwrap();
            assert!(publish_success(
                &job.0,
                Identity {
                    protocol,
                    request_id: 7,
                    source_id: 3
                },
                &serde_json::json!({"ok":true}),
                b"STEP"
            )
            .is_err());
            assert!(!job.0.join("result.step").exists());
            assert!(!job.0.join("report.json").exists());
            assert_eq!(fs::read_dir(&job.0).unwrap().count(), 1);
        }
    }

    #[test]
    fn compact_report_limit_refuses_publication_without_payloads() {
        let job = TestJob::new();
        let report = "x".repeat(COMPACT_REPORT_LIMIT);
        let error = publish_success(
            &job.0,
            Identity {
                protocol: ProtocolVersion::Compact,
                request_id: 7,
                source_id: 3,
            },
            &report,
            b"STEP",
        )
        .unwrap_err();
        assert_eq!(error.code, "worker.output_too_large");
        assert_eq!(fs::read_dir(&job.0).unwrap().count(), 0);
    }

    #[test]
    fn completion_error_is_bounded_even_for_escaped_unicode_messages() {
        let job = TestJob::new();
        let error = FabError::new("source.invalid", "\0\n🛠".repeat(10000));
        atomic_json(
            &job.0,
            "completion.json",
            COMPLETION_LIMIT,
            &completion(Identity::default(), Some(&error)),
        )
        .unwrap();
        assert!(
            fs::metadata(job.0.join("completion.json")).unwrap().len() <= COMPLETION_LIMIT as u64
        );
        assert_eq!(job.finished()["error"]["code"], "source.invalid");
    }

    #[test]
    fn a_job_cannot_reuse_old_outputs_or_run_twice() {
        let job = TestJob::new();
        let first = Job::acquire(&job.0).unwrap();
        assert!(Job::acquire(&job.0).is_err());
        drop(first);
        fs::write(job.0.join("result.step"), b"old").unwrap();
        assert_eq!(
            Job::acquire(&job.0).err().unwrap().code,
            "worker.job_not_fresh"
        );
        assert_eq!(fs::read(job.0.join("result.step")).unwrap(), b"old");
    }

    #[test]
    fn a_small_cube_uses_the_standard_validated_conversion_and_completion() {
        let job = TestJob::new();
        job.request();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cube.stl"),
            job.0.join("source.stl"),
        )
        .unwrap();
        assert!(execute_job(&job.0).unwrap());
        let done = job.finished();
        assert_eq!(done["version"], 1);
        assert_eq!(done["status"], "success");
        assert!(done.get("error").is_none());
        assert_eq!(done["request_id"], 7);
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(job.0.join("report.json")).unwrap()).unwrap();
        assert_eq!(report["cad"]["solids"], 1);
        assert_eq!(report["cad"]["faces"], 6);
        assert!(report["bodies"][0]["analysis"]["mesh"]["triangles"].is_array());
        assert!(report["bodies"][0]["analysis"]["model"]["faces"].is_array());
        assert!(fs::metadata(job.0.join("result.step")).unwrap().len() > 0);
        let progress: serde_json::Value =
            serde_json::from_slice(&fs::read(job.0.join("progress.json")).unwrap()).unwrap();
        assert_eq!(progress["phase"], "finish");
    }

    #[test]
    fn compact_report_keeps_validated_measurements_and_coverage_without_geometry() {
        let options = Options {
            reconstruction: fabstep_core::ReconstructionMode::Cylinders,
            ..Options::default()
        };
        let [legacy, compact] = [1, COMPACT_REPORT_VERSION].map(|version| {
            let job = TestJob::new();
            job.request_with_options(version, &options);
            fs::write(
                job.0.join("source.stl"),
                include_bytes!("../../../fixtures/m2c_open_arc.stl"),
            )
            .unwrap();
            assert!(execute_job(&job.0).unwrap());
            let done = job.finished();
            assert_eq!(done["version"], version);
            assert_eq!(done["status"], "success");
            assert_eq!(done["request_id"], 7);
            assert_eq!(done["source_id"], 3);
            assert!(fs::metadata(job.0.join("result.step")).unwrap().len() > 0);
            let bytes = fs::read(job.0.join("report.json")).unwrap();
            if version == COMPACT_REPORT_VERSION {
                let encoded: CompactWorkerReport = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(encoded.version, version);
            }
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        });
        let compact = &compact["report"];
        for key in ["source", "settings", "cad", "diagnostics", "guidance_hints"] {
            assert_eq!(compact[key], legacy[key]);
        }
        let old_body = &legacy["bodies"][0];
        let new_body = &compact["bodies"][0];
        for key in ["id", "name", "provenance", "settings", "cad", "fidelity"] {
            assert_eq!(new_body[key], old_body[key], "{key}");
        }
        assert_eq!(
            new_body["analysis"],
            serde_json::json!({"report": old_body["analysis"]["report"]})
        );
        assert_eq!(
            new_body["mixed_recognition"],
            serde_json::json!({
                "report": old_body["mixed_recognition"]["report"],
                "planar_faces": old_body["mixed_recognition"]["model"]["planes"].as_array().unwrap().len(),
            })
        );
    }
}
