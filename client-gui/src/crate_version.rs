pub fn version() -> &'static str {
	concat!("v", env!("CARGO_PKG_VERSION"), " ", env!("GIT_COMMIT_HASH"))
}
