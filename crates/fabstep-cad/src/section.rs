//! Exact closed sections of immutable validated STEP documents.
use super::{native_error, NativeProfileSegment, MAX_STEP_BYTES};
use fabstep_core::{
    validate_profile_frame, validate_section_profile, validate_section_request, FabError,
    ProfileFrame, ProfileSegment, SectionBodyProfile, SectionLoop, SectionProfile, SectionRequest,
};
use std::ffi::c_char;
use std::slice;

#[repr(C)]
struct NativeSectionLoop {
    solid_id: usize,
    component_id: usize,
    start: usize,
    count: usize,
    is_outer: usize,
}

#[repr(C)]
#[derive(Default)]
struct NativeSection {
    loops: *mut NativeSectionLoop,
    loop_count: usize,
    segments: *mut NativeProfileSegment,
    segment_count: usize,
}

impl Drop for NativeSection {
    fn drop(&mut self) {
        unsafe { fabstep_free_section_profile(self.loops, self.segments) };
    }
}

unsafe extern "C" {
    fn fabstep_planar_face_frame(
        bytes: *const u8,
        size: usize,
        face_id: usize,
        frame: *mut f64,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn fabstep_section_profile(
        bytes: *const u8,
        size: usize,
        frame: *const f64,
        solid_ids: *const usize,
        solid_count: usize,
        output: *mut NativeSection,
        error: *mut c_char,
        capacity: usize,
    ) -> i32;
    fn fabstep_free_section_profile(
        loops: *mut NativeSectionLoop,
        segments: *mut NativeProfileSegment,
    );
}

fn check_bytes(bytes: &[u8]) -> Result<(), FabError> {
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err(FabError::new(
            "cad.read_failed",
            "Le STEP doit être non vide et ne pas dépasser 128 Mio.",
        ));
    }
    Ok(())
}

/// Returns the native oriented plane without requiring exportable boundary curves.
pub fn planar_face_frame(bytes: &[u8], face_id: usize) -> Result<ProfileFrame, FabError> {
    check_bytes(bytes)?;
    let mut frame = [0.; 12];
    let mut error = [0; 512];
    let status = unsafe {
        fabstep_planar_face_frame(
            bytes.as_ptr(),
            bytes.len(),
            face_id,
            frame.as_mut_ptr(),
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    let result = ProfileFrame {
        origin_mm: [frame[0], frame[1], frame[2]],
        x_axis: [frame[3], frame[4], frame[5]],
        y_axis: [frame[6], frame[7], frame[8]],
        normal: [frame[9], frame[10], frame[11]],
    };
    validate_profile_frame(&result)?;
    Ok(result)
}

/// Sections every explicitly selected body. Empty bodies remain in the result;
/// any ambiguous/unsupported/tangent/open body rejects the entire operation.
pub fn section_profile(bytes: &[u8], request: &SectionRequest) -> Result<SectionProfile, FabError> {
    check_bytes(bytes)?;
    validate_section_request(request)?;
    let frame: Vec<_> = request
        .frame
        .origin_mm
        .into_iter()
        .chain(request.frame.x_axis)
        .chain(request.frame.y_axis)
        .chain(request.frame.normal)
        .collect();
    let mut native = NativeSection::default();
    let mut error = [0; 512];
    let status = unsafe {
        fabstep_section_profile(
            bytes.as_ptr(),
            bytes.len(),
            frame.as_ptr(),
            request.solid_ids.as_ptr(),
            request.solid_ids.len(),
            &mut native,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(native_error(status, &error));
    }
    let invalid = || {
        FabError::new(
            "dxf.invalid_profile",
            "Le noyau a renvoyé une section incohérente.",
        )
    };
    if native.loop_count > 4096
        || native.segment_count > 100_000
        || (native.loop_count > 0 && native.loops.is_null())
        || (native.segment_count > 0 && native.segments.is_null())
    {
        return Err(invalid());
    }
    // Empty native arrays are never passed to from_raw_parts, even for length 0.
    let loops = if native.loop_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(native.loops, native.loop_count) }
    };
    let segments = if native.segment_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(native.segments, native.segment_count) }
    };
    let mut bodies: Vec<_> = request
        .solid_ids
        .iter()
        .map(|&solid_id| SectionBodyProfile {
            solid_id,
            loops: vec![],
        })
        .collect();
    let mut end = 0;
    for row in loops {
        if row.start != end
            || row.count == 0
            || row.count > native.segment_count - end
            || row.is_outer > 1
        {
            return Err(invalid());
        }
        let body_index = request
            .solid_ids
            .binary_search(&row.solid_id)
            .map_err(|_| invalid())?;
        end += row.count;
        let mut boundary = Vec::with_capacity(row.count);
        for segment in &segments[row.start..end] {
            boundary.push(match segment.kind {
                0 => ProfileSegment::Line {
                    start_mm: segment.start,
                    end_mm: segment.end,
                },
                1 => ProfileSegment::Arc {
                    center_mm: segment.center,
                    radius_mm: segment.radius,
                    start_angle_rad: segment.start_angle,
                    sweep_angle_rad: segment.sweep_angle,
                },
                2 => ProfileSegment::Circle {
                    center_mm: segment.center,
                    radius_mm: segment.radius,
                },
                3 => ProfileSegment::Ellipse {
                    center_mm: segment.center,
                    major_axis_mm: segment.major_axis,
                    ratio: segment.ratio,
                    start_param_rad: segment.start_angle,
                    sweep_param_rad: segment.sweep_angle,
                },
                _ => return Err(invalid()),
            });
        }
        bodies[body_index].loops.push(SectionLoop {
            component_id: row.component_id,
            is_outer: row.is_outer == 1,
            segments: boundary,
        });
    }
    if end != native.segment_count {
        return Err(invalid());
    }
    let result = SectionProfile {
        frame: request.frame.clone(),
        bodies,
    };
    validate_section_profile(&result)?;
    Ok(result)
}
