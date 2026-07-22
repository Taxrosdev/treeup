use std::{ffi::OsString, path::PathBuf};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
pub enum StringLike {
    Str(String),
    OsStr(OsString),
}

impl From<String> for StringLike {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

impl From<OsString> for StringLike {
    fn from(value: OsString) -> Self {
        match value.into_string() {
            Ok(str) => Self::Str(str),
            Err(os_str) => Self::OsStr(os_str),
        }
    }
}

impl StringLike {
    pub fn to_path_buf(&self) -> PathBuf {
        match self {
            StringLike::Str(str) => PathBuf::from(str),
            StringLike::OsStr(os_str) => PathBuf::from(os_str),
        }
    }

    pub fn to_os_string(&self) -> OsString {
        match self {
            StringLike::Str(str) => OsString::from(str),
            StringLike::OsStr(os_str) => os_str.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osstr_to_str() {
        let test_str = OsString::from("test");
        let test = StringLike::from(test_str);

        assert_eq!(test, StringLike::Str("test".to_string()))
    }

    #[test]
    fn to_path_buf() {
        let test = StringLike::Str("test".to_string()).to_path_buf();
        assert_eq!(test, PathBuf::from("test"))
    }

    #[test]
    fn to_os_string() {
        let test = StringLike::Str("test".to_string()).to_os_string();
        assert_eq!(test, OsString::from("test"))
    }
}
