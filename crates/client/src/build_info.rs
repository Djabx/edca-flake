//! Build/version identity.
//!
//! CI sets `EDCAS_BUILD` to the GitLab pipeline number (`$CI_PIPELINE_IID`) at
//! compile time, so the build number *is* the version shown to users. Local and
//! development builds (no env set) report `"dev"`.
pub const BUILD: &str = match option_env!("EDCAS_BUILD") {
    Some(b) => b,
    None => "dev",
};
