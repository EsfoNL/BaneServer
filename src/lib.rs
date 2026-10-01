use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use argon2::PasswordHasher;

pub mod strenum;
/// return the error or a hashing error
/// # Errors
/// propegates argon2 errors
pub fn hash_password(password: &str, salt: &[u8]) -> Result<String, argon2::password_hash::Error> {
    Ok(argon2::Argon2::default()
        .hash_password_with_salt(password.as_bytes(), salt)?
        .to_string())
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
