extern crate winapi;
extern crate kernel32;
extern crate user32;
extern crate gdi32;
extern crate xinput;

use std::mem::{size_of, zeroed};
use std::os::raw::{c_void};

use winapi::{UINT, WPARAM, LPARAM, LRESULT, LPVOID, LPCSTR, LPCWSTR};
use winapi::{HWND, HDC, HMENU, HICON, HCURSOR, HBRUSH, RECT};
use winapi::{WNDCLASSEXW, CS_VREDRAW, CS_HREDRAW, COLOR_WINDOWFRAME};
use winapi::{WS_OVERLAPPEDWINDOW, WS_VISIBLE, CW_USEDEFAULT};
use winapi::{WM_DESTROY, WM_PAINT, WM_SIZE, WM_CLOSE, WM_QUIT, WM_ACTIVATEAPP};
use kernel32::{GetModuleHandleA};
use user32::{RegisterClassExW, CreateWindowExW, MessageBoxA};
use user32::{PeekMessageW, TranslateMessage, DispatchMessageW};
use user32::{DefWindowProcW, PostQuitMessage, BeginPaint, EndPaint};
use xinput::{XInputGetState};

struct OffscreenBuffer {
    width: i32,
    height: i32,
    memory: *mut c_void,
    pitch: i32,
    bytes_per_pixel: i32,
    info: winapi::BITMAPINFO,
}
struct Dimension {
    height: i32,
    width: i32,
}

static mut global_buffer: OffscreenBuffer = OffscreenBuffer {
    width: 0,
    height: 0,
    memory: 0 as *mut c_void,
    pitch: 0,
    bytes_per_pixel: 4,
    info: winapi::BITMAPINFO{
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
    }
};
static mut running: bool = false;

unsafe
fn render_weird_gradient(buffer: &mut OffscreenBuffer, x_offset: i32, y_offset: i32) {
    let mut row = buffer.memory;
    for y in 0..buffer.height {
        let mut pixel = row as *mut [u8; 4];
        for x in 0..buffer.width {
            *pixel = [(x + x_offset) as u8, (y + y_offset) as u8, 0, 0];
            pixel = pixel.offset(1);
        }
        row = row.offset(buffer.pitch as isize);
    }
}

unsafe
fn win32_resize_dib_section(buffer: &mut OffscreenBuffer, width: i32, height: i32){
    if !buffer.memory.is_null(){
        kernel32::VirtualFree(buffer.memory, 0, winapi::winnt::MEM_RELEASE);
    }
    buffer.width = width;
    buffer.height = height;
    buffer.info.bmiHeader.biSize = size_of::<winapi::BITMAPINFOHEADER>() as u32;
    buffer.info.bmiHeader.biWidth = buffer.width;
    buffer.info.bmiHeader.biHeight = -buffer.height;
    buffer.pitch = buffer.width * buffer.bytes_per_pixel;
    let bitmap_memory_size = (width as u64 * height as u64) * buffer.bytes_per_pixel as u64;
    buffer.memory = kernel32::VirtualAlloc(
        0 as *mut c_void,
        bitmap_memory_size,
        winapi::winnt::MEM_COMMIT,
        winapi::winnt::PAGE_READWRITE
    );

    render_weird_gradient(buffer, 128,0);
}

unsafe
fn win32_update_window(device_context: HDC, window_width: i32, window_height: i32,
        buffer: &mut OffscreenBuffer){
    gdi32::StretchDIBits(
        device_context,
        0, 0, window_width, window_height,
        0, 0, buffer.width, buffer.height,
        buffer.memory,
        &buffer.info,
        winapi::DIB_RGB_COLORS, winapi::SRCCOPY
    );
}

unsafe
fn wind32_get_window_dimension(window: HWND) -> Dimension {
    let mut rect = zeroed::<RECT>();
    user32::GetClientRect(window, &mut rect);
    Dimension {
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    }
}

unsafe extern "system"
fn wnd_proc(window: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        },
        WM_SIZE => {
            let dim = wind32_get_window_dimension(window);
            println!("WM_SIZE : {0} {1}", dim.width, dim.height);
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
            let dim = wind32_get_window_dimension(window);
            let mut paint = zeroed::<winapi::PAINTSTRUCT>();
            let device_context = BeginPaint(window, &mut paint);
            win32_update_window(device_context, dim.width, dim.height, &mut global_buffer);
            EndPaint(window, &mut paint);
            0
        },
        _ => {
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}

fn main() {
    let sz_class: &[u8] = b"H\0a\0n\0d\0m\0a\0d\0e\0H\0e\0r\0o\0";
    let sz_title: &[u8] = b"H\0a\0n\0d\0m\0a\0d\0e\0-\0h\0e\0r\0o\0";
    unsafe {
        win32_resize_dib_section(&mut global_buffer, 1280, 720);
        let h_instance = GetModuleHandleA(0 as LPCSTR);
        let window_class = WNDCLASSEXW {
            style: CS_VREDRAW|CS_HREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hIcon: 0 as HICON,
            hCursor: 0 as HCURSOR,
            lpszMenuName: 0 as LPCWSTR,
            lpszClassName: sz_class.as_ptr() as *const u16,
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
                    sz_class.as_ptr() as *const u16,
                    sz_title.as_ptr() as *const u16,
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
                    let mut y_offset = 0;
                    while running {
                        while PeekMessageW(&mut msg, 0 as HWND, 0, 0, winapi::PM_REMOVE) != 0 {
                            if msg.message == WM_QUIT {
                                running = false;
                            }
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }

                        for controller_index in 0..winapi::XUSER_MAX_COUNT {
                            let mut controller_state: winapi::XINPUT_STATE = zeroed();
                            if XInputGetState( controller_index, &mut controller_state) == winapi::ERROR_SUCCESS {
                                let pad: winapi::XINPUT_GAMEPAD = controller_state.Gamepad;
                                let up = pad.wButtons & winapi::XINPUT_GAMEPAD_DPAD_UP;
                                let down = pad.wButtons & winapi::XINPUT_GAMEPAD_DPAD_DOWN;
                                let left = pad.wButtons & winapi::XINPUT_GAMEPAD_DPAD_LEFT;
                                let right = pad.wButtons & winapi::XINPUT_GAMEPAD_DPAD_RIGHT;

                                let back = pad.wButtons & winapi::XINPUT_GAMEPAD_BACK;
                                let left_shoulder = pad.wButtons & winapi::XINPUT_GAMEPAD_LEFT_SHOULDER;
                                let right_shoulder = pad.wButtons & winapi::XINPUT_GAMEPAD_RIGHT_SHOULDER;
                                let a_button = pad.wButtons & winapi::XINPUT_GAMEPAD_A;
                                let b_button = pad.wButtons & winapi::XINPUT_GAMEPAD_B;
                                let x_button = pad.wButtons & winapi::XINPUT_GAMEPAD_X;
                                let y_button = pad.wButtons & winapi::XINPUT_GAMEPAD_Y;

                                let stick_x = pad.sThumbLX;
                                let stick_y = pad.sThumbLY;
                            }
                        }

                        let dim = wind32_get_window_dimension(window);
                        let device_context = user32::GetDC(window);
                        render_weird_gradient(&mut global_buffer, x_offset, y_offset);
                        win32_update_window(device_context, dim.width, dim.height, &mut global_buffer);
                        user32::ReleaseDC(window, device_context);
                        x_offset += 1;
                        y_offset += 1;
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
