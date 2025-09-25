use serde::{Serialize, de::DeserializeOwned};

pub mod v1;

pub trait Request: Serialize + DeserializeOwned {
	/// JSON payload if the request was successful (code 200)
	type Response: Serialize + DeserializeOwned;
	/// JSON payload of the request failed (status code - error)
	type Error: Serialize + DeserializeOwned;

	/// Request route path
	const PATH: &'static str;
}
