pub mod construct;
pub mod fas_file;
pub mod struc;

#[derive(Default, Clone)]
pub struct DataHV<T> {
    pub h: T,
    pub v: T,
}
