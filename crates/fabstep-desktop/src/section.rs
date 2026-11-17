//! Read-only section context, bound to immutable STEP/source geometry and a plane epoch.
use super::*;
use fabstep_core::{
    export_section_dxf as encode_section_dxf, validate_section_request, ProfileFrame,
    SectionProfile, SectionRequest,
};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SectionBasis {
    Xy,
    Xz,
    Yz,
    Face,
    Custom,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SectionInput {
    #[serde(default)]
    target: SectionTarget,
    #[serde(default)]
    preparation_id: Option<u64>,
    basis: SectionBasis,
    face_id: Option<usize>,
    #[serde(default)]
    custom_frame: Option<ProfileFrame>,
    offset_mm: f64,
    solid_ids: Vec<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SectionTarget {
    #[default]
    Step,
    Source,
}

#[derive(Clone)]
enum SectionDocument {
    Step(Arc<DocumentStep>),
    Source {
        preparation_id: u64,
        prepared: Arc<fabstep_app::PreparedSourceSection>,
    },
}
impl SectionDocument {
    fn target(&self) -> SectionTarget {
        match self {
            Self::Step(_) => SectionTarget::Step,
            Self::Source { .. } => SectionTarget::Source,
        }
    }
    fn preparation_id(&self) -> Option<u64> {
        match self {
            Self::Step(_) => None,
            Self::Source { preparation_id, .. } => Some(*preparation_id),
        }
    }
    fn step(&self) -> Option<&DocumentStep> {
        match self {
            Self::Step(step) => Some(step),
            Self::Source { .. } => None,
        }
    }
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Step(a), Self::Step(b)) => Arc::ptr_eq(a, b),
            (
                Self::Source {
                    preparation_id: a,
                    prepared: ap,
                },
                Self::Source {
                    preparation_id: b,
                    prepared: bp,
                },
            ) => a == b && Arc::ptr_eq(ap, bp),
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(super) struct SectionContext {
    request_id: u64,
    source_id: u64,
    section_id: u64,
    epoch: u64,
    document: SectionDocument,
    request: Option<SectionRequest>,
    profile: Option<Arc<SectionProfile>>,
}
impl SectionContext {
    pub(super) fn is_source(&self) -> bool {
        self.document.target() == SectionTarget::Source
    }
}

fn invalid_plane() -> FabError {
    FabError::new(
        "dxf.section_invalid_plane",
        "Le plan et son décalage doivent être valides (±1 000 000 000 mm maximum).",
    )
}

fn resolve_request(
    step: Option<&DocumentStep>,
    input: &SectionInput,
) -> Result<SectionRequest, FabError> {
    if !input.offset_mm.is_finite() || input.offset_mm.abs() > 1e9 {
        return Err(invalid_plane());
    }
    if input.solid_ids.len() > 32 {
        return Err(FabError::new(
            "dxf.too_complex",
            "La coupe est limitée à 32 corps.",
        ));
    }
    if !matches!(input.basis, SectionBasis::Custom) && input.custom_frame.is_some() {
        return Err(invalid_plane());
    }
    let mut frame = match input.basis {
        SectionBasis::Custom => {
            if input.face_id.is_some() {
                return Err(invalid_plane());
            }
            input.custom_frame.clone().ok_or_else(invalid_plane)?
        }
        SectionBasis::Face => step
            .ok_or_else(invalid_plane)?
            .planar_face_frame(input.face_id.ok_or_else(invalid_plane)?)?,
        basis => {
            if input.face_id.is_some() {
                return Err(invalid_plane());
            }
            let (x_axis, y_axis, normal) = match basis {
                SectionBasis::Xy => ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]),
                SectionBasis::Xz => ([1., 0., 0.], [0., 0., 1.], [0., -1., 0.]),
                SectionBasis::Yz => ([0., 1., 0.], [0., 0., 1.], [1., 0., 0.]),
                SectionBasis::Face | SectionBasis::Custom => unreachable!(),
            };
            ProfileFrame {
                origin_mm: [0.; 3],
                x_axis,
                y_axis,
                normal,
            }
        }
    };
    if frame
        .origin_mm
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1e9)
    {
        return Err(invalid_plane());
    }
    for i in 0..3 {
        frame.origin_mm[i] += input.offset_mm * frame.normal[i];
    }
    if frame
        .origin_mm
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1e9)
    {
        return Err(invalid_plane());
    }
    let request = SectionRequest {
        frame,
        solid_ids: input.solid_ids.clone(),
    };
    validate_section_request(&request)?;
    Ok(request)
}

impl CacheState {
    #[cfg(test)]
    fn begin_section(
        &mut self,
        request_id: u64,
        source_id: u64,
        section_id: u64,
        active: bool,
    ) -> Result<Option<SectionContext>, FabError> {
        self.begin_section_target(
            request_id,
            source_id,
            section_id,
            active.then_some((SectionTarget::Step, None)),
        )
    }

    fn begin_section_target(
        &mut self,
        request_id: u64,
        source_id: u64,
        section_id: u64,
        target: Option<(SectionTarget, Option<u64>)>,
    ) -> Result<Option<SectionContext>, FabError> {
        if section_id == 0 || section_id <= self.section_generation {
            return Err(stale());
        }
        self.section_generation = section_id;
        self.section = None;
        let Some((target, preparation_id)) = target else {
            return Ok(None);
        };
        let (document, epoch) = match target {
            SectionTarget::Step => {
                if preparation_id.is_some() {
                    return Err(invalid_plane());
                }
                let (step, epoch) = self.result_snapshot(request_id, source_id)?;
                (SectionDocument::Step(step), epoch)
            }
            SectionTarget::Source => {
                let preparation_id = preparation_id.ok_or_else(stale)?;
                let (prepared, epoch) =
                    self.source_section_snapshot(request_id, source_id, preparation_id)?;
                (
                    SectionDocument::Source {
                        preparation_id,
                        prepared,
                    },
                    epoch,
                )
            }
        };
        let context = SectionContext {
            request_id,
            source_id,
            section_id,
            epoch,
            document,
            request: None,
            profile: None,
        };
        self.section = Some(context.clone());
        Ok(Some(context))
    }
    fn current_section(&self, context: &SectionContext) -> bool {
        self.section_generation == context.section_id
            && self.section.as_ref().is_some_and(|current| {
                current.section_id == context.section_id && current.document.same(&context.document)
            })
            && match &context.document {
                SectionDocument::Step(step) => {
                    self.current_result(context.request_id, context.source_id, context.epoch, step)
                }
                SectionDocument::Source {
                    preparation_id,
                    prepared,
                } => self.current_source_section(
                    context.request_id,
                    context.source_id,
                    context.epoch,
                    *preparation_id,
                    prepared,
                ),
            }
    }
    fn section_context(
        &self,
        request_id: u64,
        source_id: u64,
        section_id: u64,
    ) -> Result<SectionContext, FabError> {
        self.section
            .as_ref()
            .filter(|context| {
                context.request_id == request_id
                    && context.source_id == source_id
                    && context.section_id == section_id
                    && self.current_section(context)
            })
            .cloned()
            .ok_or_else(stale)
    }
}

#[derive(Serialize)]
pub(crate) struct SectionContextResponse {
    request_id: u64,
    source_id: u64,
    section_id: u64,
    target: SectionTarget,
    preparation_id: Option<u64>,
    frame: Option<ProfileFrame>,
}
#[derive(Serialize)]
pub(crate) struct SectionResponse {
    request_id: u64,
    source_id: u64,
    section_id: u64,
    target: SectionTarget,
    preparation_id: Option<u64>,
    profile: SectionProfile,
}

#[tauri::command]
pub(crate) async fn set_section_context(
    request_id: u64,
    source_id: u64,
    section_id: u64,
    input: Option<SectionInput>,
    state: State<'_, Arc<EngineState>>,
) -> Result<SectionContextResponse, FabError> {
    let state = Arc::clone(state.inner());
    let context = state
        .inner
        .lock()
        .map_err(lock_error)?
        .begin_section_target(
            request_id,
            source_id,
            section_id,
            input
                .as_ref()
                .map(|input| (input.target, input.preparation_id)),
        )?;
    let Some(context) = context else {
        return Ok(SectionContextResponse {
            request_id,
            source_id,
            section_id,
            target: SectionTarget::Step,
            preparation_id: None,
            frame: None,
        });
    };
    let input = input.ok_or_else(invalid_plane)?;
    let worker_context = context.clone();
    let worker_state = Arc::clone(&state);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        if !worker_state
            .inner
            .lock()
            .map_err(lock_error)?
            .current_section(&worker_context)
        {
            return Err(stale());
        }
        resolve_request(worker_context.document.step(), &input)
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current_section(&context) {
        return Err(stale());
    }
    let request = computed.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    let frame = request.frame.clone();
    cache.section.as_mut().ok_or_else(stale)?.request = Some(request);
    Ok(SectionContextResponse {
        request_id,
        source_id,
        section_id,
        target: context.document.target(),
        preparation_id: context.document.preparation_id(),
        frame: Some(frame),
    })
}

#[tauri::command]
pub(crate) async fn section_profile(
    request_id: u64,
    source_id: u64,
    section_id: u64,
    state: State<'_, Arc<EngineState>>,
) -> Result<SectionResponse, FabError> {
    let state = Arc::clone(state.inner());
    let context = state
        .inner
        .lock()
        .map_err(lock_error)?
        .section_context(request_id, source_id, section_id)?;
    let request = context
        .request
        .clone()
        .ok_or_else(|| FabError::new("dxf.profile_not_ready", "Attendez la définition du plan."))?;
    let worker_context = context.clone();
    let worker_state = Arc::clone(&state);
    let computed = tauri::async_runtime::spawn_blocking(move || {
        if !worker_state
            .inner
            .lock()
            .map_err(lock_error)?
            .current_section(&worker_context)
        {
            return Err(stale());
        }
        match &worker_context.document {
            SectionDocument::Step(step) => step.section_profile(&request),
            SectionDocument::Source { prepared, .. } => {
                prepared.section_profile_controlled(&request, &mut || {
                    if worker_state
                        .inner
                        .lock()
                        .map_err(lock_error)?
                        .current_section(&worker_context)
                    {
                        Ok(())
                    } else {
                        Err(stale())
                    }
                })
            }
        }
    })
    .await;
    let mut cache = state.inner.lock().map_err(lock_error)?;
    if !cache.current_section(&context) {
        return Err(stale());
    }
    let profile = computed.map_err(|e| FabError::new("engine.task", e.to_string()))??;
    cache.section.as_mut().ok_or_else(stale)?.profile = Some(Arc::new(profile.clone()));
    Ok(SectionResponse {
        request_id,
        source_id,
        section_id,
        target: context.document.target(),
        preparation_id: context.document.preparation_id(),
        profile,
    })
}

#[tauri::command]
pub(crate) async fn export_section_dxf(
    request_id: u64,
    source_id: u64,
    section_id: u64,
    suggested_name: String,
    app: tauri::AppHandle,
    state: State<'_, Arc<EngineState>>,
) -> Result<Option<String>, FabError> {
    let state = Arc::clone(state.inner());
    let context = state
        .inner
        .lock()
        .map_err(lock_error)?
        .section_context(request_id, source_id, section_id)?;
    let profile = context.profile.as_ref().cloned().ok_or_else(|| {
        FabError::new("dxf.profile_not_ready", "Attendez le calcul de la section.")
    })?;
    let name = std::path::Path::new(&suggested_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("FabSTEP-section.dxf")
        .to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = encode_section_dxf(&profile)?;
        if !state
            .inner
            .lock()
            .map_err(lock_error)?
            .current_section(&context)
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
        let cache = state.inner.lock().map_err(lock_error)?;
        if !cache.current_section(&context) {
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

#[cfg(test)]
mod tests {
    use super::*;
    fn installed() -> CacheState {
        let mut cache = CacheState::default();
        cache.expect_upload(400).unwrap();
        cache.source = Some(CachedSource {
            source_id: 400,
            format: InputFormat::Stl,
            bytes: Arc::new(vec![1]),
        });
        let (_, _, epoch) = cache.begin(400, 400).unwrap();
        cache
            .finish(400, 400, epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        cache
    }
    #[test]
    fn section_epochs_reject_late_plane_body_and_clear_messages() {
        let mut cache = installed();
        let original = cache.begin_section(400, 400, 1, true).unwrap().unwrap();
        assert!(cache.current_section(&original));
        let second = cache.begin_section(400, 400, 3, true).unwrap().unwrap();
        assert!(!cache.current_section(&original));
        assert!(cache.begin_section(400, 400, 2, false).is_err());
        assert!(cache.current_section(&second));
        let third = cache.begin_section(400, 400, 4, true).unwrap().unwrap();
        assert!(!cache.current_section(&second));
        assert!(cache.current_section(&third));
        cache.begin_section(0, 0, 5, false).unwrap();
        assert!(!cache.current_section(&third));
        assert_eq!(cache.result.as_ref().unwrap().step.bytes(), &[9]);
    }
    #[test]
    fn section_result_replacement_and_failed_resolution_do_not_revive_exports() {
        let mut cache = installed();
        let context = cache.begin_section(400, 400, 1, true).unwrap().unwrap();
        assert!(context
            .document
            .step()
            .unwrap()
            .planar_face_frame(0)
            .is_err());
        assert!(cache.current_section(&context));
        assert!(cache
            .section_context(400, 400, 1)
            .unwrap()
            .profile
            .is_none());
        cache
            .finish(400, 400, context.epoch, DocumentStep::untracked(vec![9]))
            .unwrap();
        assert!(!cache.current_section(&context));
        assert!(cache.section_context(400, 400, 1).is_err());
        let newer = cache.begin_section(400, 400, 2, true).unwrap().unwrap();
        cache.invalidate(401, Some(400)).unwrap();
        assert!(!cache.current_section(&newer));
        assert!(cache.section.is_none());
    }
    #[test]
    fn sections_and_face_profiles_have_independent_selection_epochs() {
        let mut cache = installed();
        cache.set_face_context(400, 400, 90, Some(0)).unwrap();
        let face = cache.face_context(400, 400, 90, 0).unwrap();
        let section = cache.begin_section(400, 400, 1, true).unwrap().unwrap();
        assert!(cache.current_face_context(&face));
        cache.set_face_context(400, 400, 91, None).unwrap();
        assert!(cache.current_section(&section));
        cache.resolve_duplicate(400, false).unwrap_err();
        assert!(!cache.current_section(&section));
    }
    #[test]
    fn source_sections_require_prepared_identity_and_do_not_depend_on_step_success() {
        let mut cache = installed();
        cache.source.as_mut().unwrap().bytes =
            Arc::new(include_bytes!("../../../fixtures/cube.stl").to_vec());
        let (preparation, _) = cache
            .begin_source_preparation(400, 400, 1, Some(Options::default()))
            .unwrap()
            .unwrap();
        let prepared = Arc::new(
            fabstep_app::prepare_source_section(
                include_bytes!("../../../fixtures/cube.stl"),
                InputFormat::Stl,
                &Options::default(),
            )
            .unwrap(),
        );
        cache
            .finish_source_preparation(preparation, Arc::clone(&prepared))
            .unwrap();
        cache.result = None; // Source sections remain usable after recognition refusal.
        let context = cache
            .begin_section_target(400, 400, 1, Some((SectionTarget::Source, Some(1))))
            .unwrap()
            .unwrap();
        assert!(cache.current_section(&context));
        let input = SectionInput {
            target: SectionTarget::Source,
            preparation_id: Some(1),
            basis: SectionBasis::Xy,
            face_id: None,
            custom_frame: None,
            offset_mm: 0.5,
            solid_ids: vec![0],
        };
        assert_eq!(
            resolve_request(context.document.step(), &input)
                .unwrap()
                .frame
                .origin_mm,
            [0., 0., 0.5]
        );
        let mut face = input;
        face.basis = SectionBasis::Face;
        face.face_id = Some(0);
        assert_eq!(
            resolve_request(context.document.step(), &face)
                .unwrap_err()
                .code,
            "dxf.section_invalid_plane"
        );
        cache.begin_source_preparation(0, 0, 2, None).unwrap();
        assert!(!cache.current_section(&context));
        assert!(cache.section.is_none());
        assert!(cache
            .begin_section_target(400, 400, 2, Some((SectionTarget::Source, Some(1))))
            .is_err());
    }

    #[test]
    fn cancelling_source_preparation_preserves_current_step_section() {
        let mut cache = installed();
        let step_context = cache.begin_section(400, 400, 1, true).unwrap().unwrap();
        cache.begin_source_preparation(0, 0, 1, None).unwrap();
        assert!(cache.current_section(&step_context));
        assert_eq!(cache.result.as_ref().unwrap().step.bytes(), &[9]);
    }

    #[test]
    fn axis_plane_offsets_are_signed_and_inputs_bounded() {
        let step = DocumentStep::untracked(vec![]);
        for (basis, origin) in [
            (SectionBasis::Xy, [0., 0., 2.]),
            (SectionBasis::Xz, [0., -2., 0.]),
            (SectionBasis::Yz, [2., 0., 0.]),
        ] {
            let input = SectionInput {
                target: SectionTarget::Step,
                preparation_id: None,
                basis,
                face_id: None,
                custom_frame: None,
                offset_mm: 2.,
                solid_ids: vec![0],
            };
            let request = resolve_request(Some(&step), &input).unwrap();
            assert_eq!(request.frame.origin_mm, origin);
            validate_section_request(&request).unwrap();
        }
        for offset_mm in [f64::NAN, f64::INFINITY, 1e9 + 1.] {
            let input = SectionInput {
                target: SectionTarget::Step,
                preparation_id: None,
                basis: SectionBasis::Xy,
                face_id: None,
                custom_frame: None,
                offset_mm,
                solid_ids: vec![0],
            };
            assert_eq!(
                resolve_request(Some(&step), &input).unwrap_err().code,
                "dxf.section_invalid_plane"
            );
        }
        let input = SectionInput {
            target: SectionTarget::Step,
            preparation_id: None,
            basis: SectionBasis::Face,
            face_id: None,
            custom_frame: None,
            offset_mm: 0.,
            solid_ids: vec![0],
        };
        assert!(resolve_request(Some(&step), &input).is_err());
        let input = SectionInput {
            target: SectionTarget::Step,
            preparation_id: None,
            basis: SectionBasis::Xy,
            face_id: Some(0),
            custom_frame: None,
            offset_mm: 0.,
            solid_ids: vec![0],
        };
        assert!(resolve_request(Some(&step), &input).is_err());
    }
    #[test]
    fn custom_plane_preserves_orientation_and_offsets_along_normal() {
        let step = DocumentStep::untracked(vec![]);
        let a = std::f64::consts::FRAC_1_SQRT_2;
        let input = SectionInput {
            target: SectionTarget::Step,
            preparation_id: None,
            basis: SectionBasis::Custom,
            face_id: None,
            custom_frame: Some(ProfileFrame {
                origin_mm: [10., 20., 30.],
                x_axis: [a, 0., -a],
                y_axis: [0., 1., 0.],
                normal: [a, 0., a],
            }),
            offset_mm: -2.,
            solid_ids: vec![0],
        };
        let request = resolve_request(Some(&step), &input).unwrap();
        assert_eq!(request.frame.origin_mm, [10. - 2. * a, 20., 30. - 2. * a]);
        assert_eq!(request.frame.normal, [a, 0., a]);
        assert_eq!(request.frame.x_axis, [a, 0., -a]);
    }

    #[test]
    fn custom_plane_rejects_ambiguous_invalid_and_out_of_bounds_frames() {
        let step = DocumentStep::untracked(vec![]);
        let valid = SectionInput {
            target: SectionTarget::Step,
            preparation_id: None,
            basis: SectionBasis::Custom,
            face_id: None,
            custom_frame: Some(ProfileFrame {
                origin_mm: [0.; 3],
                x_axis: [1., 0., 0.],
                y_axis: [0., 1., 0.],
                normal: [0., 0., 1.],
            }),
            offset_mm: 0.,
            solid_ids: vec![0],
        };
        for case in 0..9 {
            let mut input = valid.clone();
            match case {
                0 => input.custom_frame = None,
                1 => input.face_id = Some(0),
                2 => input.basis = SectionBasis::Xy,
                3 => input.custom_frame.as_mut().unwrap().normal[2] = -1.,
                4 => input.custom_frame.as_mut().unwrap().x_axis[0] = 2.,
                5 => input.custom_frame.as_mut().unwrap().origin_mm[0] = f64::NAN,
                6 => input.custom_frame.as_mut().unwrap().origin_mm[0] = 1e9 + 1.,
                7 => {
                    input.custom_frame.as_mut().unwrap().origin_mm[2] = 1e9;
                    input.offset_mm = 1.;
                }
                _ => input.custom_frame.as_mut().unwrap().y_axis[1] = f64::INFINITY,
            }
            assert!(resolve_request(Some(&step), &input).is_err(), "case {case}");
        }
    }
}
