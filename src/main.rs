extern crate winapi;
extern crate kernel32;
extern crate user32;
extern crate gdi32;

use std::mem::{size_of, zeroed};
use std::os::raw::{c_void};

use winapi::{UINT, WPARAM, LPARAM, LRESULT, LPVOID, LPCSTR, LPCWSTR};
use winapi::{HWND, HDC, HMENU, HICON, HCURSOR, HBRUSH, RECT};
use winapi::{WNDCLASSEXW, CS_OWNDC, CS_VREDRAW, CS_HREDRAW, COLOR_WINDOWFRAME};
use winapi::{WS_OVERLAPPEDWINDOW, WS_VISIBLE, CW_USEDEFAULT};
use winapi::{WM_DESTROY, WM_PAINT, WM_SIZE, WM_CLOSE, WM_QUIT, WM_ACTIVATEAPP};
use kernel32::{GetModuleHandleA};
use user32::{RegisterClassExW, CreateWindowExW, MessageBoxA};
use user32::{PeekMessageW, TranslateMessage, DispatchMessageW};
use user32::{DefWindowProcW, PostQuitMessage, BeginPaint, EndPaint};

static SZ_CLASS: &'static [u8] = b"H\0a\0n\0d\0m\0a\0d\0e\0H\0e\0r\0o\0";
static SZ_TITLE: &'static [u8] = b"t\0i\0t\0l\0e\0\0\0";
static mut running: bool = false;
static mut bitmap_width: i32 = 0;
static mut bitmap_height: i32 = 0;
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

unsafe
fn render_weird_gradient(x_offset: i32, y_offset: i32) {
    let bytes_per_pixel = 4;
    let pitch = bitmap_width * bytes_per_pixel;
    println!("drawing {0} rows at {1} bytes each", bitmap_height, pitch);
    let mut row = bitmap_memory;
    for y in 0..bitmap_height {
        let mut pixel = row as *mut [u8; 4];
        for x in 0..bitmap_width {
            *pixel = [(x + x_offset) as u8, (y + y_offset) as u8, 0, 0];
            pixel = pixel.offset(1);
        }
        row = row.offset(pitch as isize);
    }
}

unsafe
fn win32_resize_dib_section(width: i32, height: i32){
    if !bitmap_memory.is_null(){
        kernel32::VirtualFree(bitmap_memory, 0, winapi::winnt::MEM_RELEASE);
    }
    bitmap_width = width;
    bitmap_height = height;
    bitmap_info.bmiHeader = winapi::BITMAPINFOHEADER{
        biSize: size_of::<winapi::BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: winapi::BI_RGB,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    bitmap_info.bmiHeader.biSize = size_of::<winapi::BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = width;
    bitmap_info.bmiHeader.biHeight = -height;
    let bytes_per_pixel = 4 as i32;
    let bitmap_memory_size = (width as u64 * height as u64) * bytes_per_pixel as u64;
    bitmap_memory = kernel32::VirtualAlloc(
        0 as *mut c_void,
        bitmap_memory_size,
        winapi::winnt::MEM_COMMIT,
        winapi::winnt::PAGE_READWRITE
    );

    render_weird_gradient(128,0);
}

unsafe
fn win32_update_window( device_context: HDC, window_rect: RECT){
    let window_width = window_rect.right - window_rect.left;
    let window_height = window_rect.bottom - window_rect.top;
    gdi32::StretchDIBits(
        device_context,
        0, 0, bitmap_width, bitmap_height,
        0, 0, window_width, window_height,
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
            let mut rect = zeroed::<RECT>();
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
            0
        },
        WM_ACTIVATEAPP => {
            println!("WM_ACTIVATEAPP");
            0
        }
        WM_PAINT => {
            let mut rect = zeroed::<RECT>();
            user32::GetClientRect(window, &mut rect);
            let mut paint = zeroed::<winapi::PAINTSTRUCT>();
            let device_context = BeginPaint(window, &mut paint);
            win32_update_window(device_context, rect);
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
                    running = true;
                    let mut x_offset = 0;
                    let y_offset = 0;
                    while running {
                        while PeekMessageW(&mut msg, 0 as HWND, 0, 0, winapi::PM_REMOVE) != 0 {
                            if msg.message == WM_QUIT {
                                running = false;
                            }
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                        let mut rect = zeroed::<RECT>();
                        user32::GetClientRect(window, &mut rect);
                        let device_context = user32::GetDC(window);
                        render_weird_gradient(x_offset, y_offset);
                        win32_update_window(device_context, rect);
                        user32::ReleaseDC(window, device_context);
                        x_offset += 1;
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
