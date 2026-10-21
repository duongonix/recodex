use serde::Serialize;

use crate::errors::IpcError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebcamDevice {
    pub id: String,
    pub name: String,
}

pub fn enumerate() -> Result<Vec<WebcamDevice>, IpcError> {
    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto).map_err(|error| {
        IpcError::new(
            "webcam_enumeration_failed",
            format!("The system could not list camera devices: {error}"),
            true,
        )
    })?;

    Ok(cameras
        .into_iter()
        .map(|camera| WebcamDevice {
            id: camera.index().as_string(),
            name: camera.human_name(),
        })
        .collect())
}
