use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use argon2::{
    password_hash::{rand_core::OsRng, Salt, SaltString},
    PasswordHasher,
};

pub mod strenum;
/// return the error or a hashing error
/// # Errors
/// propegates argon2 errors
pub fn hash_password(password: &str, salt: Salt) -> Result<String, argon2::password_hash::Error> {
    Ok(argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

#[must_use]
pub fn salt() -> SaltString {
    SaltString::generate(OsRng)
}

pub struct DebugIgnore<T>(pub T);
impl<T> Debug for DebugIgnore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DebugIgnore")
            .field(&std::any::type_name::<T>())
            .finish()
    }
}

impl<T> Deref for DebugIgnore<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for DebugIgnore<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
