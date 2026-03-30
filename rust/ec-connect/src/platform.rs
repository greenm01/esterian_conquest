#[cfg(windows)]
pub fn setup_console() {
    use winapi::shared::minwindef::TRUE;
    use winapi::shared::ntdef::NULL;
    use winapi::shared::windef::RECT;
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::wincon::{
        GetConsoleWindow, SetConsoleScreenBufferSize, SetConsoleWindowInfo, COORD, SMALL_RECT,
    };
    use winapi::um::winuser::{
        GetSystemMetrics, GetWindowRect, MoveWindow, SM_CXSCREEN, SM_CYSCREEN,
    };
    use winapi::um::processenv::GetStdHandle;

    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout == NULL as _ {
            return;
        }

        let cols: i16 = 120;
        let rows: i16 = 40;

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

        // Center the console window on screen
        let hwnd = GetConsoleWindow();
        if hwnd.is_null() {
            return;
        }

        let mut rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut rect);
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        MoveWindow(hwnd, (sw - w) / 2, (sh - h) / 2, w, h, TRUE);
    }
}

#[cfg(not(windows))]
pub fn setup_console() {}
