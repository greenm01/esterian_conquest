pub const WINDOWS_CONSOLE_COLS: i16 = 100;
pub const WINDOWS_CONSOLE_ROWS: i16 = 29;
pub const WINDOWS_CONSOLE_FONT_HEIGHT_CAP: i16 = 16;

pub fn preferred_console_size() -> (i16, i16) {
    (WINDOWS_CONSOLE_COLS, WINDOWS_CONSOLE_ROWS)
}

pub fn preferred_console_font_height(current_height: i16) -> i16 {
    if current_height > WINDOWS_CONSOLE_FONT_HEIGHT_CAP {
        WINDOWS_CONSOLE_FONT_HEIGHT_CAP
    } else {
        current_height
    }
}

pub fn centered_start(container_start: i32, container_size: i32, item_size: i32) -> i32 {
    let centered = container_start + (container_size - item_size) / 2;
    centered.max(container_start)
}

#[cfg(windows)]
pub fn setup_console() {
    use std::mem::{size_of, zeroed};

    use winapi::shared::minwindef::{FALSE, TRUE};
    use winapi::shared::ntdef::NULL;
    use winapi::shared::windef::RECT;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::wincon::{
        CONSOLE_FONT_INFOEX, COORD, GetConsoleWindow, GetCurrentConsoleFontEx,
        SMALL_RECT, SetConsoleScreenBufferSize, SetConsoleWindowInfo, SetCurrentConsoleFontEx,
    };
    use winapi::um::winuser::{
        GetWindowRect, MoveWindow, SystemParametersInfoW, SPI_GETWORKAREA,
    };
    use winapi::um::processenv::GetStdHandle;

    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout == NULL as _ {
            return;
        }

        let (cols, rows) = preferred_console_size();
        normalize_console_font(stdout);

        // Shrink window before buffer to avoid size conflict
        let small = SMALL_RECT {
            Left: 0,
            Top: 0,
            Right: cols - 1,
            Bottom: rows - 1,
        };
        SetConsoleWindowInfo(stdout, TRUE, &small);

        let buf = COORD { X: cols, Y: rows };
        SetConsoleScreenBufferSize(stdout, buf);
        SetConsoleWindowInfo(stdout, TRUE, &small);

        // Center the console window within the usable work area (excludes taskbar)
        let hwnd = GetConsoleWindow();
        if hwnd.is_null() {
            return;
        }

        let mut work: RECT = zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
        let aw = work.right - work.left;
        let ah = work.bottom - work.top;

        let mut rect: RECT = zeroed();
        GetWindowRect(hwnd, &mut rect);
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        let x = centered_start(work.left, aw, w);
        let y = centered_start(work.top, ah, h);
        MoveWindow(hwnd, x, y, w, h, TRUE);
    }

    unsafe fn normalize_console_font(stdout: *mut core::ffi::c_void) {
        let mut font: CONSOLE_FONT_INFOEX = zeroed();
        font.cbSize = size_of::<CONSOLE_FONT_INFOEX>() as u32;
        if GetCurrentConsoleFontEx(stdout, FALSE, &mut font) == 0 {
            return;
        }

        let desired_height = preferred_console_font_height(font.dwFontSize.Y);
        if desired_height <= 0 || desired_height == font.dwFontSize.Y {
            return;
        }

        font.dwFontSize.Y = desired_height;
        let _ = SetCurrentConsoleFontEx(stdout, FALSE, &font);
    }
}

#[cfg(not(windows))]
pub fn setup_console() {}
