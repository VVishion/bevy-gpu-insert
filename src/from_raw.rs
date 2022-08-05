pub trait FromRaw {
    fn from_raw(raw: &[u8]) -> Self;
}
