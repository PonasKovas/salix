pub use tonic;

mod generated {
	tonic::include_proto!("main");
}
pub use generated::*;

/// Version of the protocol (which is just the version of this crate)
///
/// To be sent in the handshake
pub const VERSION: semver::Version = const {
	// this might not be the most "robust" solution to get the package version
	// as a const here, but i dont want to pull in any more dependencies just for this
	// and im sure we can trust cargo to provide CARGO_PKG_VERSION that will be parsed correctly
	let bytes = env!("CARGO_PKG_VERSION").as_bytes();

	let mut result = [0; 3];
	let mut part_index = 0;
	let mut current_num = 0;
	let mut i = 0;
	while i < bytes.len() && current_num < 3 {
		let byte = bytes[i];

		if byte >= b'0' && byte <= b'9' {
			current_num = current_num * 10 + (byte - b'0') as u64;
		} else if byte == b'.' {
			result[part_index] = current_num;
			part_index += 1;
			current_num = 0;
		}

		i += 1;
	}
	result[part_index] = current_num;

	semver::Version::new(result[0], result[1], result[2])
};
