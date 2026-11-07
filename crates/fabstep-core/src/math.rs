pub(crate) type V = [f64; 3];
pub(crate) fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(crate) fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub(crate) fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn norm(a: V) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub(crate) fn scale(a: V, s: f64) -> V {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub(crate) fn bounds(points: &[V]) -> [V; 2] {
    let mut result = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in points {
        for (i, x) in p.iter().enumerate() {
            result[0][i] = result[0][i].min(*x);
            result[1][i] = result[1][i].max(*x);
        }
    }
    result
}
#[derive(Default)]
pub(crate) struct Sum {
    value: f64,
    correction: f64,
}
impl Sum {
    pub fn add(&mut self, x: f64) {
        let t = self.value + x;
        self.correction += if self.value.abs() >= x.abs() {
            (self.value - t) + x
        } else {
            (x - t) + self.value
        };
        self.value = t;
    }
    pub fn get(&self) -> f64 {
        self.value + self.correction
    }
}
