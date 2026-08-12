use argon2::{
    password_hash::{rand_core::OsRng, Salt, SaltString},
    PasswordHasher,
};

pub fn hash_password(password: &str, salt: Salt) -> String {
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

pub fn salt() -> SaltString {
    SaltString::generate(OsRng::default())
}
