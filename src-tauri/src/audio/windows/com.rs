use crate::errors::IpcError;

pub(super) struct ComApartment;

impl ComApartment {
    pub(super) fn initialize() -> Result<Self, IpcError> {
        wasapi::initialize_mta().ok().map_err(|error| {
            IpcError::new(
                "audio_runtime_unavailable",
                format!("Could not initialize the Windows audio apartment: {error}"),
                true,
            )
        })?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}
