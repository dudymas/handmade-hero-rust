extern crate winapi;
extern crate kernel32;
extern crate user32;
extern crate gdi32;

use std::mem::{size_of, zeroed};
use std::os::raw::{c_void};

use winapi::{UINT, WPARAM, LPARAM, LRESULT, LPVOID, LPCSTR, LPCWSTR};
use winapi::{HWND, HDC, HMENU, HICON, HCURSOR, HBRUSH};
use winapi::{WNDCLASSEXW, CS_OWNDC, CS_VREDRAW, CS_HREDRAW, COLOR_WINDOWFRAME};
use winapi::{WS_OVERLAPPEDWINDOW, WS_VISIBLE, CW_USEDEFAULT};
use winapi::{WM_DESTROY, WM_PAINT, WM_SIZE, WM_CLOSE, WM_ACTIVATEAPP};
use kernel32::{GetModuleHandleA};
use user32::{RegisterClassExW, CreateWindowExW, MessageBoxA};
use user32::{GetMessageW, TranslateMessage, DispatchMessageW};
use user32::{DefWindowProcW, PostQuitMessage, BeginPaint, EndPaint};
use gdi32::{TextOutA};

static SZ_CLASS: &'static [u8] = b"L\0n\0d\0C\0r\0a\0f\0t\0";
static SZ_TITLE: &'static [u8] = b"t\0i\0t\0l\0e\0\0\0";
static SZ_TEXT: &'static [u8] = b"Hello, world!";
static mut bitmap_memory: *mut c_void = 0 as *mut c_void;
static mut bitmap_info: winapi::BITMAPINFO = winapi::BITMAPINFO{
    bmiHeader: winapi::BITMAPINFOHEADER{
        biSize: 0,
        biWidth: 0,
        biHeight: 0,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: winapi::BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    },
    bmiColors: []
};

macro_rules! c_str {
    ($s:expr) => { {
        concat!($s, "\0").as_ptr() as *const i8
    } }
}

unsafe
fn win32_resize_dib_section(width: i32, height: i32){
    if !bitmap_memory.is_null(){
        kernel32::VirtualFree(bitmap_memory, 0, winapi::winnt::MEM_RELEASE);
    }
    bitmap_info.bmiHeader.biSize = size_of::<winapi::BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = width;
    bitmap_info.bmiHeader.biHeight = height;
    let bytes_per_pixel = 4;
    let bitmap_memory_size = (width as u64 * height as u64)*bytes_per_pixel;
    bitmap_memory = kernel32::VirtualAlloc(
        0 as *mut c_void,
        bitmap_memory_size,
        winapi::winnt::MEM_COMMIT,
        winapi::winnt::PAGE_READWRITE
    );
}

unsafe
fn win32_update_window( device_context: HDC, x: i32, y: i32, width: i32, height: i32){
    gdi32::StretchDIBits(
        device_context,
        x, y, width, height,
        x, y, width, height,
        bitmap_memory,
        &bitmap_info,
        winapi::DIB_RGB_COLORS, winapi::SRCCOPY
    );
}

unsafe extern "system"
fn wnd_proc(window: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        },
        WM_SIZE => {
            let mut rect = zeroed::<winapi::RECT>();
            user32::GetClientRect(window, &mut rect);
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            println!("WM_SIZE : {0} {1}", width, height);
            win32_resize_dib_section(width, height);
            0
        },
        WM_CLOSE => {
            println!("WM_CLOSE");
            PostQuitMessage(0);
            DefWindowProcW(window, message, wparam, lparam)
        },
        WM_ACTIVATEAPP => {
            println!("WM_ACTIVATEAPP");
            0
        }
        WM_PAINT => {
            let mut paint = zeroed();
            let device_context = BeginPaint(window, &mut paint);
            let x = paint.rcPaint.left;
            let y = paint.rcPaint.top;
            let width = paint.rcPaint.right - paint.rcPaint.left;
            let height = paint.rcPaint.bottom - paint.rcPaint.top;
            win32_update_window(device_context, x, y, width, height);
            gdi32::PatBlt(device_context, x, y, width, height, winapi::BLACKNESS);
            TextOutA(device_context, 5, 5,
                SZ_TEXT.as_ptr() as *const i8,
                SZ_TEXT.len() as i32
            );
            EndPaint(window, &mut paint);
            0
        },
        _ => {
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}

fn main() {
    unsafe {
        let h_instance = GetModuleHandleA(0 as LPCSTR);
        let window_class = WNDCLASSEXW {
            style: CS_OWNDC|CS_VREDRAW|CS_HREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hIcon: 0 as HICON,
            hCursor: 0 as HCURSOR,
            lpszMenuName: 0 as LPCWSTR,
            lpszClassName: SZ_CLASS.as_ptr() as *const u16,
            hInstance: h_instance,
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hbrBackground: (COLOR_WINDOWFRAME) as HBRUSH,
            hIconSm: 0 as HICON,
        };
        match RegisterClassExW(&window_class) {
            0 => {
                MessageBoxA(
                    0 as HWND,
                    b"Call to RegisterClassEx failed!\0".as_ptr() as *const i8,
                    b"Win32 Guided Tour\0".as_ptr() as *const i8,
                    0 as UINT
                );
            },
            _atom => {
                let window = CreateWindowExW(
                    0,
                    SZ_CLASS.as_ptr() as *const u16,
                    SZ_TITLE.as_ptr() as *const u16,
                    WS_OVERLAPPEDWINDOW|WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    0 as HWND, 0 as HMENU,
                    h_instance,
                    0 as LPVOID
                );
                if window.is_null() {
                    MessageBoxA(
                        0 as HWND,
                        b"Call to CreateWindow failed!\0".as_ptr() as *const i8,
                        b"Win32 Guided Tour\0".as_ptr() as *const i8,
                        0 as UINT
                    );
                } else {
                    let mut msg = zeroed();
                    while GetMessageW(&mut msg, 0 as HWND, 0, 0) != 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                };
            }
        };
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
