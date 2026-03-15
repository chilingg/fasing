#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct AssignVal {
    pub base: f32,
    pub excess: f32,
}

impl AssignVal {
    pub fn new(base: f32, excess: f32) -> Self {
        Self { base, excess }
    }

    pub fn total(&self) -> f32 {
        self.base + self.excess
    }

    pub fn check_set(&mut self, val: f32) {
        self.excess = (val - self.base).max(0.0);
    }
}
