/// Prepend the ESI base URL to a `format!`‐style string.
///
/// # Examples
///
/// ```rust
/// # #![allow(unused_must_use)]
/// use universe::esi;
/// let character_id = 90000001;
/// let url = esi!("/characters/{}/", character_id);
/// assert_eq!(url, "https://esi.evetech.net/latest/characters/90000001/");
/// ```
#[macro_export]
macro_rules! esi {
    // With one or more positional formatting args
    ($fmt:literal, $($args:expr),+ $(,)?) => {
        format!(
            concat!("https://esi.evetech.net/latest", $fmt),
            $($args),+
        )
    };
    // No formatting args
    ($fmt:literal $(,)?) => {
        format!(concat!("https://esi.evetech.net/latest", $fmt))
    };
}