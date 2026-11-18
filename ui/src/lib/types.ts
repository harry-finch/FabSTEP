export type Options = { input_unit: 'mm' | 'inch'; weld_tolerance_mm: number; plane_tolerance_mm: number; reconstruction: 'auto' | 'planar' | 'cylinders' | 'primitives' | 'surfaces'; cylinder_fit_tolerance_mm: number; max_deviation_mm: number };
export type ProfileLoop = { kind: 'polygon'; points: number[][] } | { kind: 'circle'; center: number[]; radius_mm: number };
export type Diagnostic = { code: string; severity: 'info' | 'warning'; message: string; count: number };
export type RecognitionReport = { cylinders: number; cones: number; spheres: number; tori: number; toroidal_fillets: number; max_radial_residual_mm: number; max_chord_error_mm: number; max_wall_residual_mm: number; max_deviation_bound_mm: number; expected_volume_mm3: number; diagnostics: Diagnostic[] };
export type MixedSegment = { kind: 'line'; vertices: [number, number] } | { kind: 'arc'; arc: number; reversed: boolean };
export type MixedLoop = { kind: 'polygon'; vertices: number[] } | { kind: 'circle'; ring: number } | { kind: 'segments'; segments: MixedSegment[] };
export type MixedModel = { vertices: number[][]; rings: { center: number[]; axis: number[]; radius_mm: number }[]; planes: { normal: number[]; loops: MixedLoop[] }[]; cylinders: { rings: [number, number]; inner: boolean }[]; arcs: { center: number[]; axis: number[]; radius_mm: number; sweep_rad: number; vertices: [number, number] }[]; partial_cylinders: { arcs: [number, number]; inner: boolean }[] };
export type RevolvedModel = { mixed: MixedModel; cones: { rings: [number, number]; inner: boolean }[] };
export type RoundedModel = { octants?: { center: number[]; radius_mm: number; directions: number[][]; arc_uses: [number, boolean][]; inner: boolean }[]; revolved: RevolvedModel; partial_cones?: { arcs: [number, number]; inner: boolean }[]; spheres: { rings: [number, number] | null; center: number[]; radius_mm: number; inner: boolean }[]; tori: { rings: [number, number] | null; axis?: number[]; center: number[]; major_radius_mm: number; minor_radius_mm: number; meridian_rad: [number, number]; inner: boolean }[] };
export type RoundedBoxFace = { kind:'plane'; axis:number; positive:boolean } | { kind:'cylinder'; axis:number; positive:[boolean, boolean] } | { kind:'sphere'; positive:[boolean, boolean, boolean] };
export type RoundedBoxRecognition = { model:{ vertices:number[][]; center:number[]; axes:number[][]; core_half_extents:number[]; radius_mm:number }; report:RecognitionReport; triangle_faces:RoundedBoxFace[] };
export type Fidelity = { source_to_result_mm: number; result_to_source_mm: number; source_samples: number; result_samples: number; tessellation_deflection_mm: number };
export type Mesh = { vertices: number[][]; triangles: number[][] };
export type Source = { format: 'stl' | '3mf'; unit: string; unit_origin: 'selected' | 'declared' | 'default' };
export type BodyProvenance = { generated_name?: boolean; model_path?: string | null; object_id?: number | null; build_item?: number | null; instance_path?: unknown; component_index?: number };
export type DocumentPreview = { request_id: number; source: Source; bodies: { id: string; name: string; provenance?: BodyProvenance; mesh: Mesh }[]; diagnostics: Diagnostic[] };
export type BodyResult = { id: string; name: string; provenance: BodyProvenance; analysis: { mesh: { vertices: number[][]; triangles: number[][] }; model: { vertices: number[][]; faces: { loops: number[][]; normal: number[] }[] }; report: { input_triangles: number; vertices: number; planar_faces: number; welded_vertices: number; volume_mm3: number; bounds_mm: number[][]; diagnostics: Diagnostic[] } }; cad: { solids: number; faces: number; planar_faces: number; cylindrical_faces: number; conical_faces: number; spherical_faces: number; toroidal_faces: number; volume_mm3: number; bounds_mm: number[][]; kernel: string }; recognition: { extrusion: { origin: number[]; axis: number[]; x_axis: number[]; height_mm: number; outer: ProfileLoop; holes: ProfileLoop[] }; report: RecognitionReport } | null; rounded_box_recognition?: RoundedBoxRecognition | null; rounded_recognition: { model: RoundedModel; report: RecognitionReport; triangle_faces: { kind: 'plane' | 'cylinder' | 'partial_cylinder' | 'partial_cone' | 'cone' | 'sphere' | 'spherical_octant' | 'torus'; index: number }[] } | null; revolved_recognition: { model: RevolvedModel; report: RecognitionReport; triangle_faces: { kind: 'plane' | 'cylinder' | 'cone'; index: number }[] } | null; mixed_recognition?: { model: MixedModel; report: RecognitionReport; triangle_faces: { kind: 'plane' | 'cylinder' | 'partial_cylinder'; index: number }[] } | null; fidelity?: Fidelity | null };

export type GuidedFamily = 'auto'|'plane'|'cylinder'|'cone'|'torus';
export type GuidedReport = { hint_id:number; family:GuidedFamily; selected_face_ids:number[]; preview_body_id:string; analysis_body_id:string; selected_source_triangle_ids:number[]; reconstructed_source_triangle_ids:number[] };
export type Result = { guidance_hints:GuidedReport[]; guidance?:GuidedReport|null; settings: Omit<Options, 'input_unit'>; request_id: number; source: Source; bodies: BodyResult[]; cad: BodyResult['cad']; diagnostics: Diagnostic[] };

/** Provenance topologique exacte ; chaque voisin est un indice du maillage source placé. */
export type SelectionTopology = { request_id:number; source_id:number; components:{ preview_body_id:string; analysis_body_id:string; analysis_to_source_triangles:number[]; source_triangle_neighbors:number[][] }[] };

/** Identités natives valables uniquement pour cette révision du STEP réimporté. */
export type StepFamily = 'plane'|'cylinder'|'cone'|'sphere'|'torus'|'other';
export type StepSourceCorrespondence = { status:'exact'; faces:{ face_id:number; preview_body_id:string; analysis_body_id:string; source_triangle_ids:number[] }[] } | { status:'unavailable'; reason:string };
export type ResultFaces = { request_id:number; source_id:number; mesh:Mesh; faces:{ id:number; solid_id:number; family:StepFamily; triangle_start:number; triangle_count:number; adjacent_face_ids:number[] }[]; deflection_mm:number; source_correspondence:StepSourceCorrespondence };

/** Contours natifs d’une seule face plane, dans un repère orthonormé en millimètres. */
export type FaceProfileSegment = { kind:'line'; start_mm:[number,number]; end_mm:[number,number] } | { kind:'arc'; center_mm:[number,number]; radius_mm:number; start_angle_rad:number; sweep_angle_rad:number } | { kind:'circle'; center_mm:[number,number]; radius_mm:number } | {kind:'ellipse';center_mm:[number,number];major_axis_mm:[number,number];ratio:number;start_param_rad:number;sweep_param_rad:number};
export type ProfileFrame = { origin_mm:[number,number,number]; x_axis:[number,number,number]; y_axis:[number,number,number]; normal:[number,number,number] };
export type NativeProfileLoop = { is_outer:boolean; segments:FaceProfileSegment[] };
export type PlanarFaceProfile = { face_id:number; solid_id:number; frame:ProfileFrame; loops:NativeProfileLoop[] };
export type FaceProfileResponse = { request_id:number; source_id:number; selection_id:number; profile:PlanarFaceProfile };
export type SectionInput = { basis:'xy'|'xz'|'yz'|'face'|'custom'; face_id:number|null; offset_mm:number; solid_ids:number[]; custom_frame?:ProfileFrame|null;target?:'step'|'source';preparation_id?:number|null };
export type SectionProfile = { frame:ProfileFrame; bodies:{ solid_id:number; loops:(NativeProfileLoop & {component_id:number})[] }[] };
export type SectionContextResponse = { request_id:number; source_id:number; section_id:number; frame:ProfileFrame|null;target:'step'|'source';preparation_id:number|null };
export type SectionResponse = { request_id:number; source_id:number; section_id:number; profile:SectionProfile;target:'step'|'source';preparation_id:number|null };
export type SourceSectionMetadata={source:Source;settings:Omit<Options,'input_unit'>;bounds_mm:number[][];bodies:{solid_id:number;analysis_body_id:string;name:string;provenance:BodyProvenance;bounds_mm:number[][];triangles:number;input_triangles:number;welded_vertices:number;volume_mm3:number;diagnostics:Diagnostic[]}[];diagnostics:Diagnostic[]};
export type SourceSectionPreparationResponse={request_id:number;source_id:number;preparation_id:number;metadata:SourceSectionMetadata|null};
