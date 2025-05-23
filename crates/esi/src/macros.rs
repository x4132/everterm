pub const ESI_URL: &'static str = "https://esi.evetech.net/latest";

/// Prepend the ESI base URL to a `format!`‐style string.
///
/// # Examples
///
/// ```rust
/// # #![allow(unused_must_use)]
/// let character_id = 90000001;
/// let url = esi!("/characters/{}/", character_id);
/// assert_eq!(url, "https://esi.evetech.net/latest/characters/90000001/");
/// ```
#[macro_export]
macro_rules! esi_url {
    // With one or more positional formatting args
    ($fmt:literal, $($args:expr),+ $(,)?) => {
        format!(
            concat!(ESI_URL, $fmt),
            $($args),+
        )
    };
    // No formatting args
    ($fmt:literal $(,)?) => {
        format!(concat!(ESI_URL, $fmt))
    };
}