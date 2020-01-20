use crate::game;
use std::{ffi::OsStr, iter::once, mem::*, os::windows::ffi::OsStrExt, ptr::null_mut};
use winapi::{
    ctypes::c_void,
    shared::{minwindef::LRESULT, minwindef::*, mmreg::*, windef::*, winerror::ERROR_SUCCESS},
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

struct SoundOutput {
    running_sample_index: u32,
    bytes_per_sample: u32,
    sound_buffer_size: u32,
    samples_per_second: u32,
    latency_sample_count: u32,
}

static mut GLOBAL_RUNNING: bool = true;
static mut GLOBAL_SOUND_BUFFER: LPDIRECTSOUNDBUFFER = null_mut();
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

fn win32_string(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

fn kilobytes(bytes: usize) -> usize {
    bytes * 1024
}

fn megabytes(bytes: usize) -> usize {
    1024 * kilobytes(bytes)
}

fn gigabytes(bytes: usize) -> usize {
    1024 * megabytes(bytes)
}

fn terabytes(bytes: usize) -> usize {
    1024 * gigabytes(bytes)
}

// TODO: investigate XAudio2
// waiting on https://github.com/retep998/winapi-rs/pull/602
unsafe fn init_direct_sound(window: HWND, samples_per_second: u32, buffer_size: u32) {
    let mut direct_sound_ptr: LPDIRECTSOUND = zeroed();
    match DirectSoundCreate(null_mut(), &mut direct_sound_ptr, null_mut()) {
        DS_OK => {
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

            match direct_sound.SetCooperativeLevel(window, DSSCL_PRIORITY) {
                DS_OK => {
                    let mut buffer_description: DSBUFFERDESC = zeroed();
                    buffer_description.dwSize = size_of::<DSBUFFERDESC>() as u32;
                    buffer_description.dwFlags = DSBCAPS_PRIMARYBUFFER;
                    let mut primary_buffer: LPDIRECTSOUNDBUFFER = zeroed();

                    match direct_sound.CreateSoundBuffer(
                        &buffer_description,
                        &mut primary_buffer,
                        null_mut(),
                    ) {
                        DS_OK => match (*primary_buffer).SetFormat(&wave_format) {
                            DS_OK => info!("Successfully set the wave format"),
                            e => error!("Couldn't set the wave format: {:x}", e),
                        },
                        e => error!("Couldn't create the primary sound buffer: {:x}", e),
                    }
                }
                e => error!("Couldn't set the cooperative level: {:x}", e),
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
                DS_OK => info!("Secondary buffer created successfully"),
                e => error!("Couldn't create the secondary sound buffer: {:x}", e),
            }
        }
        e => {
            error!("Couldn't create direct sound object: {:x}", e);
        }
    }
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
    buffer: &OffscreenBuffer,
    device_context: HDC,
    window_width: i32,
    window_height: i32,
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
        WM_ACTIVATEAPP => trace!("WM_ACTIVATEAPP"),
        WM_CLOSE | WM_DESTROY => GLOBAL_RUNNING = false,
        WM_KEYUP | WM_KEYDOWN => {
            let vk_code = w_param as i32;
            let was_down = l_param & (1 << 30) != 0;
            let is_down = l_param & (1 << 31) == 0;

            if was_down != is_down {
                match vk_code {
                    VK_W => trace!("W"),
                    VK_A => trace!("A"),
                    VK_S => trace!("S"),
                    VK_D => trace!("D"),
                    VK_Q => trace!("Q"),
                    VK_E => trace!("E"),
                    VK_UP => trace!("up"),
                    VK_LEFT => trace!("LEFT"),
                    VK_DOWN => trace!("DOWN"),
                    VK_RIGHT => trace!("RIGHT"),
                    VK_ESCAPE => {
                        trace!(
                            "ESCAPE: {}{}",
                            if is_down { " IsDown" } else { "" },
                            if was_down { " WasDown" } else { "" }
                        );
                    }
                    VK_SPACE => trace!("SPACE"),
                    _ => {}
                };
            }
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let device_context = BeginPaint(window, &mut paint);
            display_buffer_in_window(
                &GLOBAL_BACK_BUFFER,
                device_context,
                dimension.width,
                dimension.height,
            );
            EndPaint(window, &paint);
        }
        WM_SIZE => {}
        _ => {
            trace!("default");
            result = DefWindowProcW(window, message, w_param, l_param);
        }
    }

    result
}

unsafe fn clear_sound_buffer(sound_output: &SoundOutput) {
    let mut region1 = zeroed();
    let mut region1_size = zeroed();
    let mut region2 = zeroed();
    let mut region2_size = zeroed();
    match (*GLOBAL_SOUND_BUFFER).Lock(
        0,
        sound_output.sound_buffer_size,
        &mut region1,
        &mut region1_size,
        &mut region2,
        &mut region2_size,
        0,
    ) {
        DS_OK => {
            let mut dest_sample = region1 as *mut u8;
            for _ in 0..region1_size {
                *dest_sample = 0;
                dest_sample = dest_sample.wrapping_offset(1);
            }
            dest_sample = region2 as *mut u8;
            for _ in 0..region2_size {
                *dest_sample = 0;
                dest_sample = dest_sample.wrapping_offset(1);
            }
            (*GLOBAL_SOUND_BUFFER).Unlock(region1, region1_size, region2, region2_size);
        }
        e => error!("Could not lock the sound buffer to clear it: {:x}", e),
    }
}

unsafe fn fill_sound_buffer(
    sound_output: &mut SoundOutput,
    byte_to_lock: u32,
    bytes_to_write: u32,
    source_buffer: &mut game::SoundOutputBuffer,
) {
    let mut region1 = zeroed();
    let mut region1_size = zeroed();
    let mut region2 = zeroed();
    let mut region2_size = zeroed();
    match (*GLOBAL_SOUND_BUFFER).Lock(
        byte_to_lock,
        bytes_to_write,
        &mut region1,
        &mut region1_size,
        &mut region2,
        &mut region2_size,
        0,
    ) {
        DS_OK => {
            let region1_sample_count = region1_size / sound_output.bytes_per_sample;
            let mut source_sample = source_buffer.samples;
            let mut dest_sample = region1 as *mut i16;
            for _ in 0..region1_sample_count {
                *dest_sample = *source_sample;
                dest_sample = dest_sample.offset(1);
                source_sample = source_sample.offset(1);
                *dest_sample = *source_sample;
                dest_sample = dest_sample.offset(1);
                source_sample = source_sample.offset(1);
                sound_output.running_sample_index += 1;
            }
            let region2_sample_count = region2_size / sound_output.bytes_per_sample;
            dest_sample = region2 as *mut i16;
            for _ in 0..region2_sample_count {
                *dest_sample = *source_sample;
                dest_sample = dest_sample.offset(1);
                source_sample = source_sample.offset(1);
                *dest_sample = *source_sample;
                dest_sample = dest_sample.offset(1);
                source_sample = source_sample.offset(1);
                sound_output.running_sample_index += 1;
            }
            (*GLOBAL_SOUND_BUFFER).Unlock(region1, region1_size, region2, region2_size);
        }
        e => error!("Could not lock sound buffer for filling: {:x}", e),
    }
}

fn process_xinput_digital_button(
    xinput_button_state: WORD,
    old_state: &game::ButtonState,
    button_bit: WORD,
    new_state: &mut game::ButtonState,
) {
    new_state.ended_down = (xinput_button_state & button_bit) == button_bit;
    new_state.half_transition_count = if old_state.ended_down != new_state.ended_down {
        1
    } else {
        0
    };
}

pub fn main() {
    unsafe {
        let mut performance_count_frequency_result = zeroed();
        QueryPerformanceFrequency(&mut performance_count_frequency_result);
        let performance_count_frequency = *performance_count_frequency_result.QuadPart();

        let window_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
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
                let device_context = GetDC(window);

                // sound test
                let samples_per_second = 48_000;
                let bytes_per_sample = (size_of::<i16>() * 2) as u32;
                let sound_buffer_size = samples_per_second * bytes_per_sample as u32;
                let latency_sample_count = samples_per_second / 10;
                let mut sound_output = SoundOutput {
                    running_sample_index: 0,
                    bytes_per_sample,
                    sound_buffer_size,
                    samples_per_second,
                    latency_sample_count,
                };
                init_direct_sound(
                    window,
                    sound_output.samples_per_second,
                    sound_output.sound_buffer_size,
                );
                clear_sound_buffer(&sound_output);
                // Shelving sound output for now - revisiting on day 19/20
                // (*GLOBAL_SOUND_BUFFER).Play(0, 0, DSBPLAY_LOOPING);

                GLOBAL_RUNNING = true;

                // TODO: Pool with bitmap virtualalloc
                // remove max_possible_overrun?
                let max_possible_overrun = 2 * 8 * size_of::<u16>() as u32;
                let samples = VirtualAlloc(
                    null_mut(),
                    (sound_output.sound_buffer_size + max_possible_overrun) as usize,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                ) as *mut i16;

                let base_address: LPVOID = if cfg!(debug_assertions) {
                    null_mut::<VOID>().wrapping_add(terabytes(2))
                } else {
                    null_mut::<VOID>()
                };
                let mut game_memory: game::Memory = zeroed();
                game_memory.permanent_storage_size = megabytes(64);
                game_memory.transient_storage_size = gigabytes(4);

                let total_size =
                    game_memory.permanent_storage_size + game_memory.transient_storage_size;
                game_memory.permanent_storage = VirtualAlloc(
                    base_address,
                    total_size,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                ) as *mut u8;
                game_memory.transient_storage = game_memory
                    .permanent_storage
                    .wrapping_add(game_memory.permanent_storage_size);

                if !samples.is_null()
                    && !game_memory.permanent_storage.is_null()
                    && !game_memory.transient_storage.is_null()
                {
                    let mut new_input: game::Input = zeroed();
                    let mut old_input: game::Input = zeroed();

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
                        let mut max_controller_count = XUSER_MAX_COUNT as usize;
                        if max_controller_count > new_input.controllers.len() {
                            max_controller_count = new_input.controllers.len()
                        }

                        for controller_index in 0..max_controller_count {
                            let old_controller = &old_input.controllers[controller_index];
                            let mut new_controller = &mut new_input.controllers[controller_index];

                            let mut controller_state: XINPUT_STATE = zeroed();
                            if XInputGetState(controller_index as u32, &mut controller_state)
                                == ERROR_SUCCESS
                            {
                                // controller is plugged in
                                let pad = controller_state.Gamepad;

                                // TODO: DPad
                                let _up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP > 0;
                                let _down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN > 0;
                                let _left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT > 0;
                                let _right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT > 0;

                                new_controller.is_analog = true;
                                new_controller.start_x = old_controller.end_x;
                                new_controller.start_y = old_controller.end_y;

                                // TODO: Dead zone processing
                                // XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE
                                // XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE

                                // TODO: Min/Max macros
                                // TODO: Collapse to single function
                                let x = if pad.sThumbLX < 0 {
                                    pad.sThumbLX as f32 / 32768.0
                                } else {
                                    pad.sThumbLX as f32 / 32767.0
                                };
                                new_controller.min_x = x;
                                new_controller.max_x = x;
                                new_controller.end_x = x;

                                let y = if pad.sThumbLY < 0 {
                                    pad.sThumbLY as f32 / 32768.0
                                } else {
                                    pad.sThumbLY as f32 / 32767.0
                                };
                                new_controller.min_y = y;
                                new_controller.max_y = y;
                                new_controller.end_y = y;

                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &old_controller.right,
                                    XINPUT_GAMEPAD_A,
                                    &mut new_controller.right,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &old_controller.down,
                                    XINPUT_GAMEPAD_B,
                                    &mut new_controller.down,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &old_controller.up,
                                    XINPUT_GAMEPAD_X,
                                    &mut new_controller.up,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &old_controller.left,
                                    XINPUT_GAMEPAD_Y,
                                    &mut new_controller.left,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &old_controller.left_shoulder,
                                    XINPUT_GAMEPAD_LEFT_SHOULDER,
                                    &mut new_controller.left_shoulder,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &old_controller.right_shoulder,
                                    XINPUT_GAMEPAD_RIGHT_SHOULDER,
                                    &mut new_controller.right_shoulder,
                                );
                            } else {
                                trace!("controller is not available");
                            }
                        }

                        let mut byte_to_lock = 0;
                        let mut bytes_to_write = 0;
                        let mut play_cursor = 0;
                        let mut write_cursor = 0;
                        let sound_is_valid = match (*GLOBAL_SOUND_BUFFER)
                            .GetCurrentPosition(&mut play_cursor, &mut write_cursor)
                        {
                            DS_OK => {
                                byte_to_lock = (sound_output.running_sample_index
                                    * sound_output.bytes_per_sample)
                                    % sound_output.sound_buffer_size;

                                let target_cursor = (play_cursor
                                    + (sound_output.latency_sample_count
                                        * sound_output.bytes_per_sample))
                                    % sound_output.sound_buffer_size;
                                bytes_to_write = if byte_to_lock > target_cursor {
                                    (sound_output.sound_buffer_size - byte_to_lock) + target_cursor
                                } else {
                                    target_cursor - byte_to_lock
                                };

                                true
                            }
                            e => {
                                error!("Could not get sound cursor position: {:x}", e);
                                false
                            }
                        };

                        let mut sound_buffer = game::SoundOutputBuffer {
                            samples_per_second: sound_output.samples_per_second,
                            sample_count: bytes_to_write / sound_output.bytes_per_sample,
                            samples,
                        };

                        let buffer = game::OffscreenBuffer {
                            memory: GLOBAL_BACK_BUFFER.memory,
                            width: GLOBAL_BACK_BUFFER.width,
                            height: GLOBAL_BACK_BUFFER.height,
                            pitch: GLOBAL_BACK_BUFFER.pitch,
                            bytes_per_pixel: GLOBAL_BACK_BUFFER.bytes_per_pixel,
                        };
                        game::update_and_render(
                            &mut game_memory,
                            &new_input,
                            &buffer,
                            &sound_buffer,
                        );

                        // direct sound output test
                        if sound_is_valid && bytes_to_write > 0 {
                            fill_sound_buffer(
                                &mut sound_output,
                                byte_to_lock,
                                bytes_to_write,
                                &mut sound_buffer,
                            );
                        }

                        let dimension = get_window_dimension(window);
                        display_buffer_in_window(
                            &GLOBAL_BACK_BUFFER,
                            device_context,
                            dimension.width,
                            dimension.height,
                        );

                        let mut end_counter: LARGE_INTEGER = zeroed();
                        QueryPerformanceCounter(&mut end_counter);

                        let counter_elapsed = end_counter.QuadPart() - last_counter.QuadPart();
                        trace!(
                            "{}ms/f, {}f/s",
                            1000 * counter_elapsed / performance_count_frequency,
                            performance_count_frequency / counter_elapsed
                        );

                        last_counter = end_counter;

                        let temp = new_input;
                        new_input = old_input;
                        old_input = temp;
                    }
                } else {
                    error!(
                        "Could not allocate game memory {:?} or samples {:?}",
                        game_memory, samples
                    );
                }
            } else {
                error!("Window wasn't created");
            }
        } else {
            error!("Couldn't register window class");
        }
    }
}
