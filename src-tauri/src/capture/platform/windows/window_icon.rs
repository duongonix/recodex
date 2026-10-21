use std::{ffi::c_void, io::Cursor, path::Path};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat, RgbaImage};
use windows_sys::Win32::{
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, HGDIOBJ, SelectObject,
    },
    UI::{
        Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGetFileInfoW},
        WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx, HICON},
    },
};

const ICON_EDGE: i32 = 32;

pub(super) fn application_icon(path: &Path) -> Option<String> {
    let path = path.to_str()?;
    let path_wide = path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut file_info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let result = unsafe {
        SHGetFileInfoW(
            path_wide.as_ptr(),
            0,
            &mut file_info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        )
    };
    if result == 0 || file_info.hIcon.is_null() {
        return None;
    }

    let icon = file_info.hIcon;
    let encoded = render_icon(icon);
    unsafe { DestroyIcon(icon) };
    encoded
}

fn render_icon(icon: HICON) -> Option<String> {
    let dc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
    if dc.is_null() {
        return None;
    }

    let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: ICON_EDGE,
        biHeight: -ICON_EDGE,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { std::mem::zeroed() }
    };
    let mut pixels: *mut c_void = std::ptr::null_mut();
    let bitmap = unsafe {
        CreateDIBSection(
            dc,
            &info,
            DIB_RGB_COLORS,
            &mut pixels,
            std::ptr::null_mut(),
            0,
        )
    };
    if bitmap.is_null() || pixels.is_null() {
        unsafe { DeleteDC(dc) };
        if !bitmap.is_null() {
            unsafe { DeleteObject(bitmap as HGDIOBJ) };
        }
        return None;
    }

    let previous = unsafe { SelectObject(dc, bitmap as HGDIOBJ) };
    let drawn = unsafe {
        DrawIconEx(
            dc,
            0,
            0,
            icon,
            ICON_EDGE,
            ICON_EDGE,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        )
    };
    let encoded = (|| {
        if drawn == 0 {
            return None;
        }
        let pixel_count = usize::try_from(ICON_EDGE * ICON_EDGE).ok()?;
        let bytes = unsafe { std::slice::from_raw_parts(pixels.cast::<u8>(), pixel_count * 4) };
        let mut rgba = bytes.to_vec();
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            if pixel[3] == 0 && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0) {
                pixel[3] = 255;
            }
        }
        let image = RgbaImage::from_raw(ICON_EDGE as u32, ICON_EDGE as u32, rgba)?;
        let mut output = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut output, ImageFormat::Png)
            .ok()?;
        Some(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(output.into_inner())
        ))
    })();

    unsafe {
        SelectObject(dc, previous);
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(dc);
    }
    encoded
}
