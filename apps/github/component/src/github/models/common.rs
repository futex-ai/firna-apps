//! Shared GitHub REST response models.

use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug)]
pub(crate) struct RequiredNullable<T>(pub(crate) Option<T>);

pub(crate) fn required_nullable<'de, D, T>(deserializer: D) -> Result<RequiredNullable<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(RequiredNullable)
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ProviderUser {
    pub(crate) id: u64,
    pub(crate) login: String,
    pub(crate) html_url: String,
}
