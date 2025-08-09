use protocol::tonic::{self, Request, Status};

pub fn version_check(req: Request<()>) -> tonic::Result<Request<()>> {
	let version = match req.metadata().get("version") {
		Some(x) => x,
		None => {
			return Err(Status::invalid_argument(
				"Request must contain the `version` metadata",
			));
		}
	};
	let version = version
		.to_str()
		.map_err(|_| Status::invalid_argument("`version` not valid str"))?;
	let version = semver::Version::parse(version)
		.map_err(|_| Status::invalid_argument("`version` not valid semver version"))?;

	// client must be compatible (care ^) with our server version
	let requirement = semver::Comparator {
		op: semver::Op::Caret,
		major: protocol::VERSION.major,
		minor: Some(protocol::VERSION.minor),
		patch: Some(protocol::VERSION.patch),
		pre: semver::Prerelease::EMPTY,
	};

	if !requirement.matches(&version) {
		return Err(Status::permission_denied(format!(
			"Incompatible version. Requirement: {requirement}"
		)));
	}

	Ok(req)
}
