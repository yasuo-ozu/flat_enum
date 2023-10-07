use flat_enum::ToBeFlatten;

#[derive(ToBeFlatten)]
pub enum Enum6<'a, const N: usize, A> {
    E8(&'a [A; N]),
}
