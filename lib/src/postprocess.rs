#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct PostprocessConfig {
    /// Convenience field for [g_code::emit::FormatOptions] field
    #[cfg_attr(feature = "serde", serde(default))]
    pub checksums: bool,
    /// Convenience field for [g_code::emit::FormatOptions] field
    #[cfg_attr(feature = "serde", serde(default))]
    pub line_numbers: bool,
    /// Convenience field for [g_code::emit::FormatOptions] field
    #[cfg_attr(feature = "serde", serde(default))]
    pub newline_before_comment: bool,
}

impl From<&PostprocessConfig> for g_code::emit::FormatOptions {
    fn from(value: &PostprocessConfig) -> Self {
        Self {
            checksums: value.checksums,
            line_numbers: value.line_numbers,
            newline_before_comment: value.newline_before_comment,
            ..Default::default()
        }
    }
}
