//! OS entropy boundary for password generation.
pub fn secure_rng() -> rand::rngs::OsRng {
    rand::rngs::OsRng
}
