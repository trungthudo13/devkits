use rand::{seq::SliceRandom, Rng};
#[derive(Clone, Debug)]
pub struct PasswordOptions {
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    pub exclude_ambiguous: bool,
}
impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 24,
            lower: true,
            upper: true,
            digits: true,
            symbols: true,
            exclude_ambiguous: true,
        }
    }
}
/// Generate a bounded batch using the same constraints for every password.
pub fn passwords(options: &PasswordOptions, count: usize) -> Result<Vec<String>, String> {
    if !(1..=1000).contains(&count) {
        return Err("Password quantity must be 1–1000.".into());
    }
    (0..count).map(|_| password(options)).collect()
}

pub fn password(options: &PasswordOptions) -> Result<String, String> {
    if !(8..=128).contains(&options.length) {
        return Err("Password length must be 8–128.".into());
    }
    let groups: Vec<Vec<u8>> = [
        (options.lower, "abcdefghijklmnopqrstuvwxyz"),
        (options.upper, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        (options.digits, "0123456789"),
        (options.symbols, "!@#$%^&*()-_=+[]{}:,.?"),
    ]
    .into_iter()
    .filter(|(enabled, _)| *enabled)
    .map(|(_, chars)| {
        chars
            .bytes()
            .filter(|c| !options.exclude_ambiguous || !b"Il1O0o".contains(c))
            .collect()
    })
    .collect();
    if groups.is_empty() {
        return Err("Select at least one character group.".into());
    }
    let pool: Vec<u8> = groups.iter().flatten().copied().collect();
    let mut rng = crate::adapter::random::secure_rng();
    let mut bytes: Vec<u8> = groups
        .iter()
        .map(|g| g[rng.gen_range(0..g.len())])
        .collect();
    while bytes.len() < options.length {
        bytes.push(pool[rng.gen_range(0..pool.len())]);
    }
    bytes.shuffle(&mut rng);
    Ok(String::from_utf8(bytes).expect("ASCII password alphabet"))
}
