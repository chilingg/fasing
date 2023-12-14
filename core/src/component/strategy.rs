#[derive(
    Clone, Copy, Hash, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum PlaceMain {
    Equal,
    Contain,
    Acute,
    Zero,
    Both,
}
