#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct AssignVal {
    pub base: f32,
    pub excess: f32,
}

impl AssignVal {
    pub fn new(base: f32, excess: f32) -> Self {
        if excess + 0.0001 < 0.0 {
            panic!("excess {} less 0", excess);
        }

        Self {
            base,
            excess: excess.max(0.0),
        }
    }

    pub fn from_base(all: f32, base: f32) -> Self {
        Self::new(base, all - base)
    }

    pub fn total(&self) -> f32 {
        self.base + self.excess
    }

    pub fn uncheck_set(&mut self, val: f32) {
        self.excess = (val - self.base).max(0.0);
    }

    pub fn set(&mut self, val: f32) -> Result<(), f32> {
        let r = if val < self.base {
            Err(self.base - val)
        } else {
            Ok(())
        };
        self.uncheck_set(val);
        r
    }
}

impl std::ops::Add<AssignVal> for AssignVal {
    type Output = AssignVal;
    fn add(self, rhs: AssignVal) -> AssignVal {
        AssignVal::new(self.base + rhs.base, self.excess + rhs.excess)
    }
}

impl std::ops::AddAssign for AssignVal {
    fn add_assign(&mut self, other: Self) {
        self.base += other.base;
        self.excess += other.excess;
    }
}

impl std::iter::Sum for AssignVal {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| AssignVal {
            base: a.base + b.base,
            excess: a.excess + b.excess,
        })
    }
}

impl<'a> std::iter::Sum<&'a AssignVal> for AssignVal {
    fn sum<I: Iterator<Item = &'a AssignVal>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| AssignVal {
            base: a.base + b.base,
            excess: a.excess + b.excess,
        })
    }
}
