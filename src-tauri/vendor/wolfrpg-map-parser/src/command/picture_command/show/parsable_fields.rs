pub trait ParsableFields<T> {
    fn parse(bytes: &[u8]) -> (usize, T);
}