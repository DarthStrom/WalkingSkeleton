use std::{
    cmp::Ordering,
    f32::{self, consts::PI},
    ffi::OsStr,
    iter::once,
    mem::*,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
};
use winapi::{
    ctypes::c_void,
    shared::{minwindef::LRESULT, mmreg::*, windef::*, winerror::ERROR_SUCCESS},
    um::{
        dsound::*, libloaderapi::GetModuleHandleW, memoryapi::*, profileapi::*, wingdi::*,
        winnt::*, winuser::*, xinput::*,
    },
};

const WINDOW_NAME: &str = "Walking Skeleton";
const WINDOW_CLASS_NAME: &str = "WalkingSkeletonWindowClass";

const VK_W: i32 = 'W' as i32;
const VK_A: i32 = 'A' as i32;
const VK_S: i32 = 'S' as i32;
const VK_D: i32 = 'D' as i32;
const VK_Q: i32 = 'Q' as i32;
const VK_E: i32 = 'E' as i32;

static mut GLOBAL_RUNNING: bool = true;
static mut GLOBAL_SOUND_BUFFER: LPDIRECTSOUNDBUFFER = null_mut();
static mut GLOBAL_PERFORMANCE_COUNT_FREQUENCY: i64 = 0;
static mut GLOBAL_BACK_BUFFER: OffscreenBuffer = OffscreenBuffer {
    info: BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 0,
            biWidth: 0,
            biHeight: 0,
            biPlanes: 0,
            biBitCount: 0,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }; 1],
    },
    memory: null_mut(),
    width: 0,
    height: 0,
    pitch: 0,
    bytes_per_pixel: 4,
};

struct OffscreenBuffer {
    info: BITMAPINFO,
    memory: *mut c_void,
    width: i32,
    height: i32,
    pitch: i32,
    bytes_per_pixel: i32,
}

struct WindowDimension {
    width: i32,
    height: i32,
}

fn get_window_dimension(window: HWND) -> WindowDimension {
    let mut client_rect = RECT::default();
    unsafe {
        GetClientRect(window, &mut client_rect);
    }
    WindowDimension {
        width: client_rect.right - client_rect.left,
        height: client_rect.bottom - client_rect.top,
    }
}

struct SoundOutput {
    tone_volume: f32,
    tone_hz: u32,
    running_sample_index: u32,
    wave_period: u32,
    bytes_per_sample: u32,
    sound_buffer_size: u32,
    samples_per_second: u32,
    latency_sample_count: u32,
}

unsafe fn fill_sound_buffer(
    sound_output: &mut SoundOutput,
    byte_to_lock: u32,
    bytes_to_write: u32,
) {
    let mut region1 = zeroed();
    let mut region1_size = zeroed();
    let mut region2 = zeroed();
    let mut region2_size = zeroed();
    if (*GLOBAL_SOUND_BUFFER).Lock(
        byte_to_lock,
        bytes_to_write,
        &mut region1,
        &mut region1_size,
        &mut region2,
        &mut region2_size,
        0,
    ) == DS_OK
    {
        let mut dest_sample = region1 as *mut i16;
        let region1_sample_count = region1_size / sound_output.bytes_per_sample;
        for _ in 0..region1_sample_count {
            let t = 2.0 * PI * sound_output.running_sample_index as f32
                / sound_output.wave_period as f32;
            let sine_value = (t).sin();
            let sample_value = (sine_value * sound_output.tone_volume) as i16;

            *dest_sample = sample_value;
            dest_sample = dest_sample.offset(1);
            *dest_sample = sample_value;
            dest_sample = dest_sample.offset(1);
            sound_output.running_sample_index += 1;
        }
        dest_sample = region2 as *mut i16;
        let region2_sample_count = region2_size / sound_output.bytes_per_sample;
        for _ in 0..region2_sample_count {
            let t = 2.0 * PI * sound_output.running_sample_index as f32
                / sound_output.wave_period as f32;
            let sine_value = (t).sin();
            let sample_value = (sine_value * sound_output.tone_volume) as i16;

            *dest_sample = sample_value;
            dest_sample = dest_sample.offset(1);
            *dest_sample = sample_value;
            dest_sample = dest_sample.offset(1);
            sound_output.running_sample_index += 1;
        }

        (*GLOBAL_SOUND_BUFFER).Unlock(region1, region1_size, region2, region2_size);
    }
}

// TODO: investigate XAudio2
// waiting on https://github.com/retep998/winapi-rs/pull/602
unsafe fn init_direct_sound(window: HWND, samples_per_second: u32, buffer_size: u32) {
    let mut direct_sound_ptr: LPDIRECTSOUND = zeroed();
    if DirectSoundCreate(null_mut(), &mut direct_sound_ptr, null_mut()) == DS_OK {
        let direct_sound = &*direct_sound_ptr;

        let bits_per_sample = 16;
        let channels = 2;
        let block_alignment = channels * bits_per_sample / 8;
        let mut wave_format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM,
            nChannels: channels,
            nSamplesPerSec: samples_per_second,
            nAvgBytesPerSec: samples_per_second * block_alignment as u32,
            nBlockAlign: block_alignment,
            wBitsPerSample: bits_per_sample,
            cbSize: 0,
        };

        if direct_sound.SetCooperativeLevel(window, DSSCL_PRIORITY) == DS_OK {
            let mut buffer_description: DSBUFFERDESC = zeroed();
            buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
            buffer_description.dwFlags = DSBCAPS_PRIMARYBUFFER;
            let mut primary_buffer: LPDIRECTSOUNDBUFFER = zeroed();

            if direct_sound.CreateSoundBuffer(&buffer_description, &mut primary_buffer, null_mut())
                == DS_OK
            {
                if (*primary_buffer).SetFormat(&wave_format) != DS_OK {
                    // TODO: diagnostic - couldn't set the format
                }
            } else {
                // TODO: diagnostic - couldn't create primary sound buffer
            }
        } else {
            // TODO: diagnostic - couldn't set the cooperative level
        }

        let mut buffer_description: DSBUFFERDESC = zeroed();
        buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
        buffer_description.dwFlags = DSBCAPS_GETCURRENTPOSITION2;
        buffer_description.dwBufferBytes = buffer_size;
        buffer_description.lpwfxFormat = &mut wave_format;

        match direct_sound.CreateSoundBuffer(
            &buffer_description,
            &mut GLOBAL_SOUND_BUFFER,
            null_mut(),
        ) {
            DS_OK => {
                // println!("secondary buffer created successfully!");
            }
            e => println!("error: {}", e),
        }
    } else {
        // TODO: diagnostic - couldn't create direct sound object
    }
}

unsafe fn render_weird_gradient(buffer: &OffscreenBuffer, x_offset: i32, y_offset: i32) {
    for y in 0..buffer.height {
        let row = (buffer.memory as *mut u8).offset((y * buffer.pitch) as isize);
        for x in 0..buffer.width {
            let pixel = row.offset((x * buffer.bytes_per_pixel) as isize);
            let blue = pixel;
            let green = pixel.offset(1);
            let red = pixel.offset(2);
            *red = 0;
            *green = (y + y_offset) as u8;
            *blue = (x + x_offset) as u8;
        }
    }
}

unsafe fn resize_dib_section(buffer: &mut OffscreenBuffer, width: i32, height: i32) {
    if !buffer.memory.is_null() {
        VirtualFree(buffer.memory, 0, MEM_RELEASE);
    }

    buffer.width = width;
    buffer.height = height;

    buffer.info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as _;
    buffer.info.bmiHeader.biWidth = buffer.width;
    buffer.info.bmiHeader.biHeight = -buffer.height;
    buffer.info.bmiHeader.biPlanes = 1;
    buffer.info.bmiHeader.biBitCount = 32;
    buffer.info.bmiHeader.biCompression = BI_RGB;

    let bitmap_memory_size = buffer.bytes_per_pixel * buffer.width * buffer.height;
    buffer.memory = VirtualAlloc(
        null_mut(),
        bitmap_memory_size as usize,
        MEM_RESERVE | MEM_COMMIT,
        PAGE_READWRITE,
    );

    buffer.pitch = buffer.width * buffer.bytes_per_pixel;
}

unsafe fn display_buffer_in_window(
    device_context: HDC,
    window_width: i32,
    window_height: i32,
    buffer: &OffscreenBuffer,
) {
    StretchDIBits(
        device_context,
        0,
        0,
        window_width,
        window_height,
        0,
        0,
        buffer.width,
        buffer.height,
        buffer.memory,
        &buffer.info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

unsafe extern "system" fn main_window_callback(
    window: HWND,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> LRESULT {
    let mut result = 0;
    let dimension = get_window_dimension(window);

    match message {
        WM_ACTIVATEAPP => {}
        WM_CLOSE | WM_DESTROY => GLOBAL_RUNNING = false,
        WM_KEYUP | WM_KEYDOWN => {
            let vk_code = w_param as i32;
            let was_down = l_param & (1 << 30) != 0;
            let is_down = l_param & (1 << 31) == 0;

            if was_down != is_down {
                match vk_code {
                    VK_W => println!("W"),
                    VK_A => println!("A"),
                    VK_S => println!("S"),
                    VK_D => println!("D"),
                    VK_Q => println!("Q"),
                    VK_E => println!("E"),
                    VK_UP => println!("up"),
                    VK_LEFT => println!("left"),
                    VK_DOWN => println!("down"),
                    VK_RIGHT => println!("right"),
                    VK_ESCAPE => println!("escape"),
                    VK_SPACE => println!("space"),
                    _ => {}
                };
            }
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let device_context = BeginPaint(window, &mut paint);
            display_buffer_in_window(
                device_context,
                dimension.width,
                dimension.height,
                &GLOBAL_BACK_BUFFER,
            );
            EndPaint(window, &paint);
        }
        WM_SIZE => {}
        _ => {
            result = DefWindowProcW(window, message, w_param, l_param);
        }
    }

    result
}

fn win32_string(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

pub fn main() {
    unsafe {
        let mut performance_count_frequency = zeroed();
        QueryPerformanceFrequency(&mut performance_count_frequency);
        GLOBAL_PERFORMANCE_COUNT_FREQUENCY = *performance_count_frequency.QuadPart();

        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(main_window_callback),
            hInstance: GetModuleHandleW(null_mut()),
            lpszClassName: win32_string(WINDOW_CLASS_NAME).as_ptr(),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
        };

        resize_dib_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);

        if RegisterClassW(&window_class) > 0 {
            let window = CreateWindowExW(
                0,
                window_class.lpszClassName,
                win32_string(WINDOW_NAME).as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                null_mut(),
                null_mut(),
                window_class.hInstance,
                null_mut(),
            );

            if !window.is_null() {
                // graphics test
                let mut x_offset = 0;
                let mut y_offset = 0;

                // sound test
                let samples_per_second = 48_000;
                let tone_hz = 256;
                let bytes_per_sample = (size_of::<i16>() * 2) as u32;
                let sound_buffer_size = samples_per_second * bytes_per_sample as u32;
                let latency_sample_count = samples_per_second / 10;
                let mut sound_output = SoundOutput {
                    tone_volume: 3_000.0,
                    tone_hz,
                    running_sample_index: 0,
                    wave_period: samples_per_second / tone_hz,
                    bytes_per_sample,
                    sound_buffer_size,
                    samples_per_second,
                    latency_sample_count,
                };

                init_direct_sound(window, samples_per_second, sound_buffer_size);
                fill_sound_buffer(
                    &mut sound_output,
                    0,
                    latency_sample_count * bytes_per_sample,
                );
                (*GLOBAL_SOUND_BUFFER).Play(0, 0, DSBPLAY_LOOPING);

                let mut last_counter: LARGE_INTEGER = zeroed();
                QueryPerformanceCounter(&mut last_counter);

                while GLOBAL_RUNNING {

                    let mut message = zeroed();

                    while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                        if message.message == WM_QUIT {
                            GLOBAL_RUNNING = false;
                        }

                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }

                    // TODO: should we poll this more frequently?
                    for controller_index in 0..XUSER_MAX_COUNT {
                        let mut controller_state: XINPUT_STATE = zeroed();
                        if XInputGetState(controller_index, &mut controller_state) == ERROR_SUCCESS
                        {
                            // controller is plugged in
                            let pad = controller_state.Gamepad;

                            let _up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP > 0;
                            let _down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN > 0;
                            let _left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT > 0;
                            let _right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT > 0;
                            let _start = pad.wButtons & XINPUT_GAMEPAD_START > 0;
                            let _back = pad.wButtons & XINPUT_GAMEPAD_BACK > 0;
                            let _left_shoulder = pad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER > 0;
                            let _right_shoulder = pad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER > 0;
                            let _a_button = pad.wButtons & XINPUT_GAMEPAD_A > 0;
                            let _b_button = pad.wButtons & XINPUT_GAMEPAD_B > 0;
                            let _x_button = pad.wButtons & XINPUT_GAMEPAD_X > 0;
                            let _y_button = pad.wButtons & XINPUT_GAMEPAD_Y > 0;

                            let stick_x = pad.sThumbLX;
                            let stick_y = pad.sThumbLY;

                            x_offset += stick_x / 4096;
                            y_offset += stick_y / 4096;

                            sound_output.tone_hz =
                                (512.0 + (256.0 * (stick_y as f32 / 30_000.0))) as u32;
                            sound_output.wave_period =
                                sound_output.samples_per_second / sound_output.tone_hz;
                        } else {
                            // controller is not available
                        }
                    }

                    render_weird_gradient(&GLOBAL_BACK_BUFFER, x_offset as i32, y_offset as i32);

                    // direct sound output test
                    let mut play_cursor = 0;
                    let mut write_cursor = 0;
                    if (*GLOBAL_SOUND_BUFFER)
                        .GetCurrentPosition(&mut play_cursor, &mut write_cursor)
                        == DS_OK
                    {
                        let byte_to_lock = (sound_output.running_sample_index
                            * sound_output.bytes_per_sample)
                            % sound_output.sound_buffer_size;

                        let target_cursor = (play_cursor
                            + (sound_output.latency_sample_count * sound_output.bytes_per_sample))
                            % sound_output.sound_buffer_size;
                        let bytes_to_write = match byte_to_lock.cmp(&target_cursor) {
                            Ordering::Equal => 0,
                            Ordering::Greater => {
                                (sound_output.sound_buffer_size - byte_to_lock) + target_cursor
                            }
                            Ordering::Less => target_cursor - byte_to_lock,
                        };

                        fill_sound_buffer(&mut sound_output, byte_to_lock, bytes_to_write);
                    }

                    let device_context = GetDC(window);

                    let dimension = get_window_dimension(window);
                    display_buffer_in_window(
                        device_context,
                        dimension.width,
                        dimension.height,
                        &GLOBAL_BACK_BUFFER,
                    );
                    ReleaseDC(window, device_context);

                    let mut end_counter: LARGE_INTEGER = zeroed();
                    QueryPerformanceCounter(&mut end_counter);

                    let counter_elapsed = end_counter.QuadPart() - last_counter.QuadPart();
                    println!("{}ms/f / {}f/s", 1000 * counter_elapsed / GLOBAL_PERFORMANCE_COUNT_FREQUENCY, GLOBAL_PERFORMANCE_COUNT_FREQUENCY / counter_elapsed);

                    last_counter = end_counter;
                }
            } else {
                // TODO: logging
            }
        } else {
            // TODO: logging
        }
    }
}
