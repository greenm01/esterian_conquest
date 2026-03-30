pub const WINDOWS_CONSOLE_COLS: i16 = 100;
pub const WINDOWS_CONSOLE_ROWS: i16 = 30;

pub fn preferred_console_size() -> (i16, i16) {
    (WINDOWS_CONSOLE_COLS, WINDOWS_CONSOLE_ROWS)
}

pub fn centered_start(container_start: i32, container_size: i32, item_size: i32) -> i32 {
    let centered = container_start + (container_size - item_size) / 2;
    centered.max(container_start)
}

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
        GetWindowRect, MoveWindow, SystemParametersInfoW, SPI_GETWORKAREA,
    };
    use winapi::um::processenv::GetStdHandle;

    unsafe {
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout == NULL as _ {
            return;
        }

        let (cols, rows) = preferred_console_size();

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

        let mut work: RECT = std::mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
        let aw = work.right - work.left;
        let ah = work.bottom - work.top;

        let mut rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut rect);
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        let x = centered_start(work.left, aw, w);
        let y = centered_start(work.top, ah, h);
        MoveWindow(hwnd, x, y, w, h, TRUE);
    }
}

#[cfg(not(windows))]
pub fn setup_console() {}
