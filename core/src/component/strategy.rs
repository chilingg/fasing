#[derive(
    Clone, Copy, Hash, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum PlaceMain {
    NoPlane,
    NonLess,
    Equal,
    Acute,
    AlignPlane,
    Contain,
    InContain,
    Both,

    Only,
    Surround,
    BeSurround,
    NoSurround,
    NoBeSurround,
}
