pub mod construct;
pub mod fas_file;
pub mod struc;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataHV<T> {
    pub h: T,
    pub v: T,
}

impl<T> DataHV<T> {
    pub fn new(h: T, v: T) -> Self {
        Self { h, v }
    }
}
