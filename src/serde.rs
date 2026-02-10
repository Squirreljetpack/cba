use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(untagged)]
enum Helper<I> {
    Single(I),
    Multiple(Vec<I>),
}

pub fn one_or_many<'de, D, T, I: Deserialize<'de>>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromIterator<I>,
{
    match Helper::deserialize(deserializer)? {
        Helper::Single(s) => Ok(std::iter::once(s).collect()),
        Helper::Multiple(v) => Ok(v.into_iter().collect()),
    }
}
