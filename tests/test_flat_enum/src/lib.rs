use flat_enum::FlatTarget;

#[derive(FlatTarget)]
pub enum Enum6<'a, const N: usize, A> {
    E8(&'a [A; N]),
}
