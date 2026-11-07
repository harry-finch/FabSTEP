use crate::{
    error, xml::*, BodySource, ImportedBody, ImportedDocument, UnitOrigin, MAX_BODIES,
    MAX_TRIANGLES,
};
use fabstep_core::{Diagnostic, FabError, Mesh, Severity};
use roxmltree::Node;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
struct Transform([[f64; 3]; 4]);
impl Transform {
    fn identity() -> Self {
        Self([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0., 0., 0.]])
    }
    fn parse(node: Node<'_, '_>) -> Result<Self, FabError> {
        let Some(text) = node.attribute("transform") else {
            return Ok(Self::identity());
        };
        let values: Vec<f64> = text
            .split_whitespace()
            .take(13)
            .map(number)
            .collect::<Result<_, _>>()?;
        if values.len() != 12 {
            return Err(error(
                "invalid_transform",
                "Une transformation 3MF exige 12 coefficients.",
            ));
        }
        let t = Self(std::array::from_fn(|i| {
            std::array::from_fn(|j| values[3 * i + j])
        }));
        t.validate()?;
        Ok(t)
    }
    fn determinant(&self) -> f64 {
        let m = &self.0;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }
    fn validate(&self) -> Result<(), FabError> {
        let scale: f64 = self.0[..3]
            .iter()
            .map(|r| r[0].hypot(r[1]).hypot(r[2]))
            .product();
        let det = self.determinant();
        if self.0.iter().flatten().any(|v| !v.is_finite())
            || !scale.is_finite()
            || !det.is_finite()
            || det.abs() <= scale * 1e-12
        {
            return Err(error(
                "invalid_transform",
                "Transformation singulière, trop mal conditionnée ou hors plage numérique.",
            ));
        }
        Ok(())
    }
    // Row-vector convention: local first, then parent. Translation is row 3.
    fn then(self, parent: Self) -> Result<Self, FabError> {
        let mut m = [[0.; 3]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| self.0[i][k] * parent.0[k][j]).sum::<f64>();
                if i == 3 {
                    *v += parent.0[3][j];
                }
            }
        }
        let t = Self(m);
        t.validate()?;
        Ok(t)
    }
    fn point(self, p: [f64; 3], unit: f64) -> Result<[f64; 3], FabError> {
        let q = std::array::from_fn(|j| {
            ((0..3).map(|i| p[i] * self.0[i][j]).sum::<f64>() + self.0[3][j]) * unit
        });
        if q.iter().any(|v| !v.is_finite()) {
            return Err(error(
                "non_finite",
                "Débordement lors de la transformation du maillage.",
            ));
        }
        Ok(q)
    }
}
struct Reference {
    id: u32,
    transform: Transform,
}
enum Geometry {
    Mesh(Mesh),
    Components(Vec<Reference>),
}
struct Object {
    name: Option<String>,
    kind: String,
    geometry: Geometry,
}
struct Budget {
    triangles: usize,
    vertices: usize,
    references: usize,
}
fn budget_error() -> FabError {
    error("complexity_limit","Limite 3MF dépassée (32 occurrences, 100 000 triangles, 300 000 sommets ou 4 096 références).")
}
fn reference(node: Node<'_, '_>, ignored: &mut usize) -> Result<Reference, FabError> {
    attributes(node, &["objectid", "transform", "partnumber"], ignored)?;
    Ok(Reference {
        id: resource_id(required(node, "objectid")?)?,
        transform: Transform::parse(node)?,
    })
}
fn mesh(node: Node<'_, '_>, budget: &mut Budget, ignored: &mut usize) -> Result<Mesh, FabError> {
    attributes(node, &[], ignored)?;
    let parts: Vec<_> = children(node)
        .filter(|n| n.tag_name().namespace() == Some(CORE))
        .collect();
    if parts.len() != 2 || !is(parts[0], CORE, "vertices") || !is(parts[1], CORE, "triangles") {
        return Err(invalid());
    }
    *ignored += children(node)
        .filter(|n| n.tag_name().namespace() != Some(CORE))
        .count();
    let mut vertices = Vec::new();
    for v in children(parts[0]) {
        if !is(v, CORE, "vertex") {
            return Err(invalid());
        }
        attributes(v, &["x", "y", "z"], ignored)?;
        if children(v).next().is_some() {
            return Err(invalid());
        }
        budget.vertices += 1;
        if budget.vertices > 300_000 {
            return Err(budget_error());
        }
        vertices.push([
            number(required(v, "x")?)?,
            number(required(v, "y")?)?,
            number(required(v, "z")?)?,
        ]);
    }
    let mut triangles = Vec::new();
    for t in children(parts[1]) {
        if !is(t, CORE, "triangle") {
            return Err(invalid());
        }
        attributes(t, &["v1", "v2", "v3", "pid", "p1", "p2", "p3"], ignored)?;
        if children(t).next().is_some() {
            return Err(invalid());
        }
        if t.attribute("pid").is_some() || t.attribute("p1").is_some() {
            *ignored += 1;
        }
        let mut indices = [0; 3];
        for (i, key) in ["v1", "v2", "v3"].into_iter().enumerate() {
            indices[i] = required(t, key)?.parse::<usize>().map_err(|_| invalid())?;
            if indices[i] >= vertices.len() {
                return Err(error(
                    "invalid_index",
                    "Indice de triangle 3MF hors du tableau des sommets.",
                ));
            }
        }
        budget.triangles += 1;
        if budget.triangles > MAX_TRIANGLES {
            return Err(budget_error());
        }
        triangles.push(indices);
    }
    if vertices.len() < 3 || triangles.is_empty() {
        return Err(error("empty_model", "Maillage 3MF vide."));
    }
    Ok(Mesh {
        vertices,
        triangles,
    })
}

pub(crate) fn read(source: &str, path: &str) -> Result<ImportedDocument, FabError> {
    let doc = parse(source)?;
    let root = doc.root_element();
    if !is(root, CORE, "model") {
        return Err(error(
            "unsupported_version",
            "Espace de noms Core 3MF non pris en charge.",
        ));
    }
    let mut ignored = 0;
    attributes(
        root,
        &["unit", "requiredextensions", "recommendedextensions"],
        &mut ignored,
    )?;
    if root
        .attribute("requiredextensions")
        .is_some_and(|v| !v.trim().is_empty())
    {
        return Err(error(
            "unsupported_extension",
            "Le document exige une extension 3MF non prise en charge (Core seul).",
        ));
    }
    if root
        .attribute("recommendedextensions")
        .is_some_and(|v| !v.trim().is_empty())
    {
        ignored += 1;
    }
    // Never silently accept Production references, even when its required flag
    // was omitted by an invalid producer or is hidden in an ignored subtree.
    if root.descendants().any(|n| {
        n.attributes()
            .any(|a| a.namespace() == Some(PRODUCTION) && a.name() == "path")
    }) {
        return Err(error(
            "unsupported_extension",
            "Les références Production multi-modèles ne sont pas prises en charge.",
        ));
    }
    validate_structure(root, &mut ignored)?;
    let unit = root.attribute("unit").unwrap_or("millimeter");
    let unit_origin = if root.attribute("unit").is_some() {
        UnitOrigin::Declared
    } else {
        UnitOrigin::Default
    };
    let factor = match unit {
        "micron" => 0.001,
        "millimeter" => 1.,
        "centimeter" => 10.,
        "inch" => 25.4,
        "foot" => 304.8,
        "meter" => 1000.,
        _ => return Err(error("invalid_unit", "Unité 3MF inconnue.")),
    };
    let mut resources = None;
    let mut build = None;
    for n in children(root) {
        if is(n, CORE, "resources") && resources.is_none() && build.is_none() {
            resources = Some(n);
        } else if is(n, CORE, "build") && resources.is_some() && build.is_none() {
            build = Some(n);
        } else if (is(n, CORE, "metadata") && resources.is_none())
            || n.tag_name().namespace() != Some(CORE)
        {
            ignored += 1;
        } else {
            return Err(invalid());
        }
    }
    let resources = resources.ok_or_else(invalid)?;
    let build = build.ok_or_else(invalid)?;
    let mut objects = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut budget = Budget {
        vertices: 0,
        triangles: 0,
        references: 0,
    };
    for n in children(resources) {
        if n.tag_name().namespace() != Some(CORE) {
            ignored += 1;
            continue;
        }
        let id = resource_id(required(n, "id")?)?;
        if !ids.insert(id) {
            return Err(error(
                "duplicate_id",
                "Identifiant de ressource 3MF dupliqué.",
            ));
        }
        if ids.len() > 4096 {
            return Err(budget_error());
        }
        if is(n, CORE, "basematerials") {
            ignored += 1;
            continue;
        }
        if !is(n, CORE, "object") {
            return Err(invalid());
        }
        attributes(
            n,
            &[
                "id",
                "type",
                "name",
                "partnumber",
                "thumbnail",
                "pid",
                "pindex",
            ],
            &mut ignored,
        )?;
        if n.attribute("pid").is_some() {
            ignored += 1;
        }
        if n.attribute("name").is_some_and(|s| s.len() > 1024) {
            return Err(error(
                "xml_limit",
                "Nom d'objet limité à 1 024 octets UTF-8.",
            ));
        }
        let name = n.attribute("name").map(str::to_owned);
        let kind = n.attribute("type").unwrap_or("model").to_owned();
        if !["model", "solidsupport", "support", "surface", "other"].contains(&kind.as_str()) {
            return Err(invalid());
        }
        let mut geometry = None;
        for child in children(n) {
            if is(child, CORE, "mesh") && geometry.is_none() {
                geometry = Some(Geometry::Mesh(mesh(child, &mut budget, &mut ignored)?));
            } else if is(child, CORE, "components") && geometry.is_none() {
                let mut refs = Vec::new();
                for c in children(child) {
                    if !is(c, CORE, "component") {
                        return Err(invalid());
                    }
                    let r = reference(c, &mut ignored)?;
                    // Core requires prior definitions, excluding all cycles.
                    if !objects.contains_key(&r.id) {
                        return Err(error(
                            "invalid_reference",
                            "Référence de composant absente, cyclique ou avant sa définition.",
                        ));
                    }
                    if children(c).next().is_some() {
                        return Err(invalid());
                    }
                    budget.references += 1;
                    if budget.references > 4096 {
                        return Err(budget_error());
                    }
                    refs.push(r);
                }
                if refs.is_empty() {
                    return Err(invalid());
                }
                geometry = Some(Geometry::Components(refs));
            } else if is(child, CORE, "metadatagroup") || child.tag_name().namespace() != Some(CORE)
            {
                ignored += 1;
            } else {
                return Err(invalid());
            }
        }
        objects.insert(
            id,
            Object {
                name,
                kind,
                geometry: geometry.ok_or_else(invalid)?,
            },
        );
    }
    let mut result = ImportedDocument {
        bodies: Vec::new(),
        diagnostics: Vec::new(),
        unit: unit.into(),
        unit_origin,
    };
    let mut expanded = Budget {
        vertices: 0,
        triangles: 0,
        references: 0,
    };
    for (build_item, item) in children(build).enumerate() {
        if !is(item, CORE, "item") {
            return Err(invalid());
        }
        let r = reference(item, &mut ignored)?;
        for child in children(item) {
            if !is(child, CORE, "metadatagroup") {
                return Err(invalid());
            }
            ignored += 1;
        }
        expand(
            &objects,
            &r,
            r.transform,
            path,
            build_item,
            &mut Vec::new(),
            factor,
            &mut result,
            &mut expanded,
        )?;
    }
    if result.bodies.is_empty() {
        return Err(error(
            "empty_model",
            "Le build 3MF ne contient aucune occurrence de maillage.",
        ));
    }
    if ignored > 0 {
        result.diagnostics.push(Diagnostic{code:"threemf.metadata_ignored".into(),severity:Severity::Warning,message:"Certaines propriétés, métadonnées ou extensions facultatives ne sont pas conservées dans le STEP géométrique.".into(),count:ignored});
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn expand(
    objects: &BTreeMap<u32, Object>,
    r: &Reference,
    transform: Transform,
    path: &str,
    build_item: usize,
    instance_path: &mut Vec<usize>,
    factor: f64,
    result: &mut ImportedDocument,
    budget: &mut Budget,
) -> Result<(), FabError> {
    budget.references += 1;
    if budget.references > 4096 || instance_path.len() > 64 {
        return Err(budget_error());
    }
    let object = objects
        .get(&r.id)
        .ok_or_else(|| error("invalid_reference", "Objet du build introuvable."))?;
    if object.kind != "model" {
        return Err(error(
            "unsupported_object",
            "Seuls les objets de type model sont convertis en solides CAO.",
        ));
    }
    match &object.geometry {
        Geometry::Mesh(mesh) => {
            if result.bodies.len() >= MAX_BODIES {
                return Err(budget_error());
            }
            budget.triangles += mesh.triangles.len();
            budget.vertices += mesh.vertices.len();
            if budget.triangles > MAX_TRIANGLES || budget.vertices > 300_000 {
                return Err(budget_error());
            }
            let vertices = mesh
                .vertices
                .iter()
                .map(|&p| transform.point(p, factor))
                .collect::<Result<_, _>>()?;
            let mut triangles = mesh.triangles.clone();
            if transform.determinant() < 0. {
                for t in &mut triangles {
                    t.swap(1, 2);
                }
            }
            result.bodies.push(ImportedBody {
                mesh: Mesh {
                    vertices,
                    triangles,
                },
                source: BodySource {
                    model_path: path.into(),
                    object_id: r.id,
                    build_item,
                    instance_path: instance_path.clone(),
                    name: object.name.clone(),
                },
            });
        }
        Geometry::Components(refs) => {
            for (i, child) in refs.iter().enumerate() {
                instance_path.push(i);
                expand(
                    objects,
                    child,
                    child.transform.then(transform)?,
                    path,
                    build_item,
                    instance_path,
                    factor,
                    result,
                    budget,
                )?;
                instance_path.pop();
            }
        }
    }
    Ok(())
}

// Validate element-only content and containers even when their attributes do
// not affect geometry. Silently ignoring `<vertices unit="inch">` would mask
// an invalid producer's attempt to supply a different coordinate unit.
fn validate_structure(root: Node<'_, '_>, ignored: &mut usize) -> Result<(), FabError> {
    for node in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().namespace() == Some(CORE))
    {
        let name = node.tag_name().name();
        if name != "metadata" {
            structural(node)?;
        }
        match name {
            "resources" | "build" | "vertices" | "triangles" | "components" | "metadatagroup" => {
                attributes(node, &[], ignored)?
            }
            "basematerials" => attributes(node, &["id"], ignored)?,
            "base" => attributes(node, &["name", "displaycolor"], ignored)?,
            "metadata" => attributes(node, &["name", "preserve", "type"], ignored)?,
            _ => {}
        }
        if ["vertex", "triangle", "component", "base", "metadata"].contains(&name)
            && children(node).next().is_some()
        {
            return Err(invalid());
        }
        if name == "metadatagroup" && children(node).any(|n| !is(n, CORE, "metadata")) {
            return Err(invalid());
        }
        if name == "basematerials" && children(node).any(|n| !is(n, CORE, "base")) {
            return Err(invalid());
        }
    }
    Ok(())
}
