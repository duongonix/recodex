use std::{os::windows::ffi::OsStrExt, path::Path};

use windows::{
    Win32::Media::MediaFoundation::{
        IMFSourceReader, MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, MF_SOURCE_READERF_ENDOFSTREAM,
        MFCreateAttributes, MFCreateSourceReaderFromURL,
    },
    Win32::System::Com::StructuredStorage::PROPVARIANT,
    core::{GUID, PCWSTR},
};

use crate::{errors::IpcError, recording::encoding::windows_media::mf_error};

pub(crate) struct TimedBytes {
    pub(crate) timestamp_100ns: i64,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) struct SampleReader {
    reader: IMFSourceReader,
    stream_index: u32,
}

impl SampleReader {
    pub(crate) fn new(reader: IMFSourceReader, stream_index: u32) -> Self {
        Self {
            reader,
            stream_index,
        }
    }

    pub(crate) fn next(&self) -> Result<Option<TimedBytes>, IpcError> {
        loop {
            let mut actual_stream = 0;
            let mut flags = 0;
            let mut timestamp_100ns = 0;
            let mut sample = None;
            unsafe {
                self.reader
                    .ReadSample(
                        self.stream_index,
                        0,
                        Some(&mut actual_stream),
                        Some(&mut flags),
                        Some(&mut timestamp_100ns),
                        Some(&mut sample),
                    )
                    .map_err(|error| {
                        mf_error("Windows could not decode the selected recording", error)
                    })?;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return Ok(None);
            }
            if let Some(sample) = sample {
                let buffer = unsafe {
                    sample.ConvertToContiguousBuffer().map_err(|error| {
                        mf_error("Windows could not read a decoded media sample", error)
                    })?
                };
                let length = unsafe {
                    buffer.GetCurrentLength().map_err(|error| {
                        mf_error("Windows could not inspect a decoded media sample", error)
                    })?
                } as usize;
                let mut data = std::ptr::null_mut();
                let mut maximum_length = 0;
                unsafe {
                    buffer
                        .Lock(&mut data, Some(&mut maximum_length), None)
                        .map_err(|error| {
                            mf_error("Windows could not access a decoded media sample", error)
                        })?;
                }
                if data.is_null() || maximum_length < length as u32 {
                    let _ = unsafe { buffer.Unlock() };
                    return Err(IpcError::new(
                        "recording_sample_invalid",
                        "Windows returned an incomplete decoded media sample.",
                        true,
                    ));
                }
                let bytes = unsafe { std::slice::from_raw_parts(data, length).to_vec() };
                unsafe {
                    buffer.Unlock().map_err(|error| {
                        mf_error("Windows could not release a decoded media sample", error)
                    })?;
                }
                return Ok(Some(TimedBytes {
                    timestamp_100ns,
                    bytes,
                }));
            }
        }
    }

    pub(crate) fn seek_to(&self, timestamp_100ns: i64) -> Result<(), IpcError> {
        let position = PROPVARIANT::from(timestamp_100ns);
        let time_format = GUID::zeroed();
        unsafe {
            self.reader
                .SetCurrentPosition(&time_format, &position)
                .map_err(|error| {
                    mf_error(
                        "Windows could not seek to a video thumbnail position",
                        error,
                    )
                })
        }
    }
}

pub(crate) fn open_reader(input_path: &Path) -> Result<IMFSourceReader, IpcError> {
    if !input_path.is_file() {
        return Err(IpcError::new(
            "recording_source_missing",
            "The recording file is no longer available for native editing.",
            true,
        ));
    }
    let wide_path = source_reader_path(input_path);
    let mut attributes = None;
    unsafe {
        MFCreateAttributes(&mut attributes, 1)
            .map_err(|error| mf_error("Could not configure the native recording decoder", error))?;
    }
    let attributes = attributes
        .ok_or_else(|| IpcError::internal("Windows did not create decoder attributes."))?;
    unsafe {
        attributes
            .SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)
            .map_err(|error| mf_error("Could not enable native video format conversion", error))?;
        MFCreateSourceReaderFromURL(PCWSTR(wide_path.as_ptr()), Some(&attributes))
            .map_err(|error| mf_error("Could not open the recording for native editing", error))
    }
}

fn source_reader_path(input_path: &Path) -> Vec<u16> {
    let path: Vec<u16> = input_path.as_os_str().encode_wide().collect();
    let verbatim_prefix: Vec<u16> = r"\\?\".encode_utf16().collect();
    let verbatim_unc_prefix: Vec<u16> = r"\\?\UNC\".encode_utf16().collect();
    let normalized = if path.starts_with(&verbatim_unc_prefix) {
        let mut unc_path = vec![b'\\' as u16, b'\\' as u16];
        unc_path.extend_from_slice(&path[verbatim_unc_prefix.len()..]);
        unc_path
    } else {
        let drive_index = verbatim_prefix.len();
        let has_drive_prefix = path.starts_with(&verbatim_prefix)
            && path.get(drive_index).is_some_and(|unit| {
                (b'A' as u16..=b'Z' as u16).contains(unit)
                    || (b'a' as u16..=b'z' as u16).contains(unit)
            })
            && path.get(drive_index + 1) == Some(&(b':' as u16))
            && path
                .get(drive_index + 2)
                .is_some_and(|unit| *unit == b'\\' as u16 || *unit == b'/' as u16);
        if has_drive_prefix {
            path[verbatim_prefix.len()..].to_vec()
        } else {
            path
        }
    };
    normalized.into_iter().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::source_reader_path;
    use std::path::Path;

    fn decoded_path(path: &Path) -> String {
        let mut wide = source_reader_path(path);
        assert_eq!(wide.pop(), Some(0));
        String::from_utf16(&wide).expect("test path should be valid UTF-16")
    }

    #[test]
    fn source_reader_path_strips_verbatim_drive_prefix() {
        assert_eq!(
            decoded_path(Path::new(r"\\?\C:\Users\duong\Videos\capture.mp4")),
            r"C:\Users\duong\Videos\capture.mp4"
        );
    }

    #[test]
    fn source_reader_path_converts_verbatim_unc_prefix() {
        assert_eq!(
            decoded_path(Path::new(r"\\?\UNC\server\share\capture.mp4")),
            r"\\server\share\capture.mp4"
        );
    }

    #[test]
    fn source_reader_path_preserves_unicode_and_regular_paths() {
        assert_eq!(
            decoded_path(Path::new(r"C:\Users\duong\Videos\bản ghi.mp4")),
            r"C:\Users\duong\Videos\bản ghi.mp4"
        );
    }
}
