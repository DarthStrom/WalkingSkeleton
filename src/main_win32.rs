use crate::game;
use std::{ffi::OsStr, iter::once, mem::*, os::windows::ffi::OsStrExt, ptr::null_mut};
use winapi::{
    ctypes::c_void,
    shared::{minwindef::LRESULT, minwindef::*, mmreg::*, windef::*, winerror::ERROR_SUCCESS},
    um::{
        dsound::*, libloaderapi::GetModuleHandleW, memoryapi::*, mmsystem::*, profileapi::*,
        synchapi::*, timeapi::*, wingdi::*, winnt::*, winuser::*, xinput::*,
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
static mut GLOBAL_PERF_COUNT_FREQUENCY: i64 = 0;
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

pub unsafe fn get_controller(
    input: *mut game::Input,
    controller_index: usize,
) -> *mut game::ControllerInput {
    debug_assert!(controller_index < (*input).controllers.len());
    &mut (*input).controllers[controller_index]
}

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
        WM_KEYUP | WM_KEYDOWN => panic!("Keyboard input came in through a non-dispatch message!"),
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

fn process_keyboard_message(new_state: &mut game::ButtonState, is_down: bool) {
    if new_state.ended_down != is_down {
        new_state.ended_down = is_down;
        new_state.half_transition_count += 1;
    }
    debug_assert!(new_state.ended_down == is_down);
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

fn process_xinput_stick_value(value: SHORT, dead_zone_threshold: SHORT) -> f32 {
    let mut result = 0.0;

    if value < -dead_zone_threshold {
        result = (value + dead_zone_threshold) as f32 / (32768.0 - dead_zone_threshold as f32)
    } else if value > dead_zone_threshold {
        result = (value - dead_zone_threshold) as f32 / (32767.0 - dead_zone_threshold as f32)
    }

    result
}

unsafe fn process_pending_messages(keyboard_controller: &mut game::ControllerInput) {
    let mut message = zeroed();
    while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
        match message.message {
            WM_QUIT => GLOBAL_RUNNING = false,
            WM_KEYDOWN | WM_KEYUP => {
                // letting windows handle WM_SYSKEYUP and WM_SYSKEYDOWN
                // so I don't have to detect Alt-F4 etc.
                let vk_code = message.wParam as i32;
                let was_down = message.lParam & (1 << 30) != 0;
                let is_down = message.lParam & (1 << 31) == 0;

                if was_down != is_down || (!was_down && !is_down) {
                    match vk_code {
                        VK_W => {
                            process_keyboard_message(&mut keyboard_controller.move_up, is_down);
                            debug!("W");
                        }
                        VK_A => {
                            process_keyboard_message(&mut keyboard_controller.move_left, is_down);
                            debug!("A");
                        }
                        VK_S => {
                            process_keyboard_message(&mut keyboard_controller.move_down, is_down);
                            debug!("S");
                        }
                        VK_D => {
                            process_keyboard_message(&mut keyboard_controller.move_right, is_down);
                            debug!("D");
                        }
                        VK_Q => {
                            process_keyboard_message(
                                &mut keyboard_controller.left_shoulder,
                                is_down,
                            );
                            debug!("Q");
                        }
                        VK_E => {
                            process_keyboard_message(
                                &mut keyboard_controller.right_shoulder,
                                is_down,
                            );
                            debug!("E");
                        }
                        VK_UP => {
                            process_keyboard_message(&mut keyboard_controller.action_up, is_down);
                            debug!("UP");
                        }
                        VK_LEFT => {
                            process_keyboard_message(&mut keyboard_controller.action_left, is_down);
                            debug!("LEFT");
                        }
                        VK_DOWN => {
                            process_keyboard_message(&mut keyboard_controller.action_down, is_down);
                            debug!("DOWN: {:?}", keyboard_controller.action_down);
                        }
                        VK_RIGHT => {
                            process_keyboard_message(
                                &mut keyboard_controller.action_right,
                                is_down,
                            );
                            debug!("RIGHT");
                        }
                        VK_ESCAPE => {
                            process_keyboard_message(&mut keyboard_controller.start, is_down);
                            debug!("ESCAPE");
                        }
                        VK_SPACE => {
                            process_keyboard_message(&mut keyboard_controller.select, is_down);
                            debug!("SPACE");
                        }
                        _ => {}
                    };
                }
            }
            _ => {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

fn get_wall_clock() -> LARGE_INTEGER {
    let mut result = LARGE_INTEGER::default();
    unsafe {
        QueryPerformanceCounter(&mut result);
    }
    result
}

fn get_seconds_elapsed(start: LARGE_INTEGER, end: LARGE_INTEGER) -> f32 {
    unsafe { (end.QuadPart() - start.QuadPart()) as f32 / GLOBAL_PERF_COUNT_FREQUENCY as f32 }
}

pub fn main() {
    unsafe {
        let mut performance_count_frequency_result = zeroed();
        QueryPerformanceFrequency(&mut performance_count_frequency_result);
        GLOBAL_PERF_COUNT_FREQUENCY = *performance_count_frequency_result.QuadPart();

        // Set the Windows scheduler granularity to 1ms
        // so that our Sleep() can be more granular
        let desired_scheduler_ms = 1;
        let sleep_is_granular = timeBeginPeriod(desired_scheduler_ms) == TIMERR_NOERROR;

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

        // TODO: How do we reliably query this on Windows?
        let monitor_refresh_hz = 60;
        let game_update_hz = monitor_refresh_hz / 2;
        let target_seconds_per_frame = 1.0 / game_update_hz as f32;

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
                game_memory.transient_storage_size = gigabytes(1);

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

                    let mut last_counter = get_wall_clock();
                    while GLOBAL_RUNNING {
                        let old_keyboard_controller = get_controller(&mut old_input, 0);
                        let new_keyboard_controller = get_controller(&mut new_input, 0);
                        *new_keyboard_controller = zeroed();
                        (*new_keyboard_controller).is_connected = true;

                        (*new_keyboard_controller).move_up.ended_down =
                            (*old_keyboard_controller).move_up.ended_down;
                        (*new_keyboard_controller).move_down.ended_down =
                            (*old_keyboard_controller).move_down.ended_down;
                        (*new_keyboard_controller).move_left.ended_down =
                            (*old_keyboard_controller).move_left.ended_down;
                        (*new_keyboard_controller).move_right.ended_down =
                            (*old_keyboard_controller).move_right.ended_down;

                        (*new_keyboard_controller).action_up.ended_down =
                            (*old_keyboard_controller).action_up.ended_down;
                        (*new_keyboard_controller).action_down.ended_down =
                            (*old_keyboard_controller).action_down.ended_down;
                        (*new_keyboard_controller).action_left.ended_down =
                            (*old_keyboard_controller).action_left.ended_down;
                        (*new_keyboard_controller).action_right.ended_down =
                            (*old_keyboard_controller).action_right.ended_down;

                        (*new_keyboard_controller).left_shoulder.ended_down =
                            (*old_keyboard_controller).left_shoulder.ended_down;
                        (*new_keyboard_controller).right_shoulder.ended_down =
                            (*old_keyboard_controller).right_shoulder.ended_down;
                        (*new_keyboard_controller).select.ended_down =
                            (*old_keyboard_controller).select.ended_down;
                        (*new_keyboard_controller).start.ended_down =
                            (*old_keyboard_controller).start.ended_down;
                        (*new_keyboard_controller).terminator.ended_down =
                            (*old_keyboard_controller).terminator.ended_down;

                        process_pending_messages(new_keyboard_controller.as_mut().unwrap());

                        // TODO: Need to not poll disconnected controllers to avoid
                        // xinput frame rate hit on older libraries...
                        // TODO: should we poll this more frequently?
                        let mut max_controller_count = XUSER_MAX_COUNT as usize;
                        if max_controller_count > (new_input.controllers.len() - 1) {
                            max_controller_count = new_input.controllers.len() - 1
                        }

                        for controller_index in 0..max_controller_count {
                            let our_controller_index = controller_index + 1;
                            let old_controller =
                                get_controller(&mut old_input, our_controller_index);
                            let mut new_controller =
                                get_controller(&mut new_input, our_controller_index);

                            let mut controller_state: XINPUT_STATE = zeroed();
                            if XInputGetState(controller_index as u32, &mut controller_state)
                                == ERROR_SUCCESS
                            {
                                (*new_controller).is_connected = true;

                                // controller is plugged in
                                let pad = controller_state.Gamepad;

                                // TODO: This is a square deadzone, check XInput to
                                // verify that the deadzone is "round" and show how to do
                                // round deadzone processing.
                                (*new_controller).stick_average_x = process_xinput_stick_value(
                                    pad.sThumbLX,
                                    XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE,
                                );
                                (*new_controller).stick_average_y = process_xinput_stick_value(
                                    pad.sThumbLY,
                                    XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE,
                                );
                                if ((*new_controller).stick_average_x != 0.0)
                                    || ((*new_controller).stick_average_y != 0.0)
                                {
                                    (*new_controller).is_analog = true;
                                }

                                if pad.wButtons & XINPUT_GAMEPAD_DPAD_UP != 0 {
                                    (*new_controller).stick_average_y = 1.0;
                                    (*new_controller).is_analog = false;
                                } else if pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0 {
                                    (*new_controller).stick_average_y = -1.0;
                                    (*new_controller).is_analog = false;
                                }

                                if pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0 {
                                    (*new_controller).stick_average_x = -1.0;
                                    (*new_controller).is_analog = false;
                                } else if pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0 {
                                    (*new_controller).stick_average_x = 1.0;
                                    (*new_controller).is_analog = false;
                                }

                                let threshold = 0.5;
                                process_xinput_digital_button(
                                    if (*new_controller).stick_average_x < -threshold {
                                        1
                                    } else {
                                        0
                                    },
                                    &(*old_controller).move_left,
                                    1,
                                    &mut (*new_controller).move_left,
                                );
                                process_xinput_digital_button(
                                    if (*new_controller).stick_average_x > threshold {
                                        1
                                    } else {
                                        0
                                    },
                                    &(*old_controller).move_right,
                                    1,
                                    &mut (*new_controller).move_right,
                                );
                                process_xinput_digital_button(
                                    if (*new_controller).stick_average_y < -threshold {
                                        1
                                    } else {
                                        0
                                    },
                                    &(*old_controller).move_down,
                                    1,
                                    &mut (*new_controller).move_down,
                                );
                                process_xinput_digital_button(
                                    if (*new_controller).stick_average_x > threshold {
                                        1
                                    } else {
                                        0
                                    },
                                    &(*old_controller).move_up,
                                    1,
                                    &mut (*new_controller).move_up,
                                );

                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).action_down,
                                    XINPUT_GAMEPAD_B,
                                    &mut (*new_controller).action_down,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).action_right,
                                    XINPUT_GAMEPAD_A,
                                    &mut (*new_controller).action_right,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).action_left,
                                    XINPUT_GAMEPAD_Y,
                                    &mut (*new_controller).action_left,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).action_up,
                                    XINPUT_GAMEPAD_X,
                                    &mut (*new_controller).action_up,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).left_shoulder,
                                    XINPUT_GAMEPAD_LEFT_SHOULDER,
                                    &mut (*new_controller).left_shoulder,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).right_shoulder,
                                    XINPUT_GAMEPAD_RIGHT_SHOULDER,
                                    &mut (*new_controller).right_shoulder,
                                );

                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).start,
                                    XINPUT_GAMEPAD_START,
                                    &mut (*new_controller).start,
                                );
                                process_xinput_digital_button(
                                    pad.wButtons,
                                    &(*old_controller).select,
                                    XINPUT_GAMEPAD_START,
                                    &mut (*new_controller).select,
                                );
                            } else {
                                (*new_controller).is_connected = false;
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

                        // TODO: Sound is wrong because we haven't updated
                        // it to go with the frame loop
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
                            &mut new_input,
                            &buffer,
                            &sound_buffer,
                        );

                        if sound_is_valid && bytes_to_write > 0 {
                            fill_sound_buffer(
                                &mut sound_output,
                                byte_to_lock,
                                bytes_to_write,
                                &mut sound_buffer,
                            );
                        }

                        let work_counter = get_wall_clock();
                        let work_seconds_elapsed = get_seconds_elapsed(last_counter, work_counter);

                        // TODO: NOT TESTED YET! PROBABLY BUGGY!!!
                        let mut seconds_elapsed_for_frame = work_seconds_elapsed;
                        if seconds_elapsed_for_frame < target_seconds_per_frame {
                            if sleep_is_granular {
                                let sleep_ms = (1000.0
                                    * (target_seconds_per_frame - seconds_elapsed_for_frame))
                                    as DWORD;
                                if sleep_ms > 0 {
                                    Sleep(sleep_ms);
                                }
                            }

                            let test_seconds_elapsed_for_frame =
                                get_seconds_elapsed(last_counter, get_wall_clock());
                            debug_assert!(
                                test_seconds_elapsed_for_frame < target_seconds_per_frame
                            );

                            while seconds_elapsed_for_frame < target_seconds_per_frame {
                                seconds_elapsed_for_frame =
                                    get_seconds_elapsed(last_counter, get_wall_clock());
                            }
                        } else {
                            warn!("missed frame rate");
                        }

                        let dimension = get_window_dimension(window);
                        display_buffer_in_window(
                            &GLOBAL_BACK_BUFFER,
                            device_context,
                            dimension.width,
                            dimension.height,
                        );

                        std::mem::swap(&mut new_input, &mut old_input);

                        let end_counter = get_wall_clock();
                        let ms_per_frame = 1000.0 * get_seconds_elapsed(last_counter, end_counter);
                        last_counter = end_counter;

                        let fps = 0.0;

                        trace!("{}ms/f, {}f/s", ms_per_frame, fps);
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
