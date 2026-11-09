//! Window capture for the documentation build (`--shot`, `--shot-live`, `--shot-settings`).
//!
//! WebView2 paints through DirectComposition, so `PrintWindow` with `PW_RENDERFULLCONTENT`
//! is used instead of an ordinary blit: it asks DWM for the composed window, chrome and
//! hotel surface together, exactly like the C# client's `CaptureWindow`.

#[cfg(windows)]
pub fn capture_window(window: &tao::window::Window, path: &std::path::Path) -> Result<(), String> {
    use tao::platform::windows::WindowExtWindows;
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HGDIOBJ, ReleaseDC, SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    const PW_RENDERFULLCONTENT: u32 = 0x0000_0002;

    // `PrintWindow` is not part of the windows-sys user32 bindings, so it is declared here.
    unsafe extern "system" {
        fn PrintWindow(hwnd: HWND, hdc: windows_sys::Win32::Graphics::Gdi::HDC, flags: u32) -> i32;
    }

    unsafe {
        let hwnd = window.hwnd() as HWND;
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err("could not measure the window".into());
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err("the window has no drawable size".into());
        }

        let screen = GetDC(std::ptr::null_mut());
        if screen.is_null() {
            return Err("could not reach the display".into());
        }
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        if memory.is_null() || bitmap.is_null() {
            if !memory.is_null() {
                DeleteDC(memory);
            }
            ReleaseDC(std::ptr::null_mut(), screen);
            return Err("could not create the capture surface".into());
        }

        let previous = SelectObject(memory, bitmap as HGDIOBJ);
        let _ = PrintWindow(hwnd, memory, PW_RENDERFULLCONTENT);

        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        // A negative height asks for a top-down buffer, so the rows are already in order.
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;

        let mut buffer = vec![0u8; (width as usize) * (height as usize) * 4];
        let lines = GetDIBits(
            memory,
            bitmap,
            0,
            height as u32,
            buffer.as_mut_ptr() as *mut _,
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(memory, previous);
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);

        if lines == 0 {
            return Err("the window could not be rendered".into());
        }

        for pixel in buffer.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        let image = image::RgbaImage::from_raw(width as u32, height as u32, buffer)
            .ok_or_else(|| "unexpected capture buffer size".to_string())?;
        image.save(path).map_err(|error| error.to_string())
    }
}
