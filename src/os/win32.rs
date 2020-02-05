//! equivalent to win32_handmade.cpp

use crate::common::*;
use std::{ffi::*, iter::once, mem::*, os::windows::ffi::OsStrExt, ptr::null_mut};
use winapi::{
    ctypes::c_void,
    shared::{minwindef::LRESULT, minwindef::*, mmreg::*, windef::*, winerror::*},
    um::{
        dsound::*, errhandlingapi::GetLastError, fileapi::*, handleapi::*, libloaderapi::*,
        memoryapi::*, minwinbase::*, mmsystem::*, profileapi::*, synchapi::*, timeapi::*,
        winbase::*, wingdi::*, winnt::*, winuser::*, xinput::*,
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
const VK_P: i32 = 'P' as i32;
const VK_L: i32 = 'L' as i32;

struct OffscreenBuffer {
    info: BITMAPINFO,
    memory: *mut c_void,
    width: i32,
    height: i32,
    pitch: i32,
    bytes_per_pixel: i32,
}

struct SoundOutput {
    running_sample_index: u32,
    bytes_per_sample: u32,
    sound_buffer_size: DWORD,
    safety_bytes: DWORD,
    samples_per_second: u32,
    // TODO: Should running sample index be in bytes as well?
    // TODO: Math gets simpler if we add a "bytes per second" field?
}

struct DebugTimeMarker {
    output_play_cursor: DWORD,
    output_write_cursor: DWORD,
    output_location: DWORD,
    output_byte_count: DWORD,
    expected_flip_play_cursor: DWORD,

    flip_play_cursor: DWORD,
    flip_write_cursor: DWORD,
}

static mut GLOBAL_RUNNING: bool = true;
static mut GLOBAL_PAUSE: bool = false;
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

fn win32_string(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(once(0)).collect()
}

unsafe fn get_exe_file_name(state: &mut State) {
    let _ = GetModuleFileNameW(
        null_mut(),
        &mut state.exe_file_name[0],
        state.exe_file_name.len() as DWORD,
    );
    let mut scan = 0;
    while state.exe_file_name[scan] != 0 {
        if state.exe_file_name[scan] == ('\\' as u16) {
            state.one_past_last_exe_file_name_slash = scan + 1;
        }
        scan += 1;
    }
}

unsafe fn build_exe_path_file_name(state: &State, file_name: &str, dest: &mut [u16; MAX_PATH]) {
    let file_name_w = win32_string(file_name);
    let mut i = 0;
    while i < state.one_past_last_exe_file_name_slash {
        dest[i] = state.exe_file_name[i];
        i += 1;
    }
    for u in file_name_w {
        dest[i] = u;
        i += 1;
    }
}

struct GameCode {
    game_code_dll: HMODULE,
    dll_last_write_time: FILETIME,
    update_and_render: GameUpdateAndRender,
    get_sound_samples: GameGetSoundSamples,
    is_valid: bool,
}

struct ReplayBuffer {
    file_handle: HANDLE,
    memory_map: HANDLE,
    file_name: [u16; MAX_PATH],
    memory_block: *mut u8,
}

struct State {
    total_size: usize,
    game_memory_block: *mut u8,
    replay_buffers: [ReplayBuffer; 4],
    recording_handle: HANDLE,
    input_recording_index: usize,
    playback_handle: HANDLE,
    input_playing_index: usize,
    exe_file_name: [u16; MAX_PATH],
    one_past_last_exe_file_name_slash: usize,
}

unsafe fn get_last_write_time(filename: &[u16; MAX_PATH]) -> FILETIME {
    let mut last_write_time: FILETIME = zeroed();
    let mut data: WIN32_FILE_ATTRIBUTE_DATA = zeroed();
    if GetFileAttributesExW(
        filename.as_ptr(),
        GetFileExInfoStandard,
        &mut data as *mut WIN32_FILE_ATTRIBUTE_DATA as *mut c_void,
    ) != 0
    {
        last_write_time = data.ftLastWriteTime;
    }
    last_write_time
}

unsafe fn load_game_code(
    source_dll_path: &[u16; MAX_PATH],
    temp_dll_path: &[u16; MAX_PATH],
) -> GameCode {
    trace!("==load_game_code==");
    let mut result: GameCode = zeroed();

    // TODO: Automatic determination of when updates are necessary.

    CopyFileW(source_dll_path.as_ptr(), temp_dll_path.as_ptr(), FALSE);
    result.game_code_dll = LoadLibraryW(temp_dll_path.as_ptr());
    if !result.game_code_dll.is_null() {
        let c_update_and_render = CString::new("update_and_render").unwrap();
        let c_get_sound_samples = CString::new("get_sound_samples").unwrap();

        let update_and_render_ptr =
            GetProcAddress(result.game_code_dll, c_update_and_render.as_ptr());
        let get_sound_samples_ptr =
            GetProcAddress(result.game_code_dll, c_get_sound_samples.as_ptr());

        result.update_and_render = transmute(update_and_render_ptr);
        result.get_sound_samples = transmute(get_sound_samples_ptr);
        result.is_valid = !update_and_render_ptr.is_null() && !get_sound_samples_ptr.is_null();
        if result.is_valid {
            trace!("successfully loaded game functions")
        } else {
            error!("could not get the function pointers");
            result.update_and_render = zeroed();
            result.get_sound_samples = zeroed();
        }
    } else {
        error!("could not load game code dll");
    }

    trace!("==load_game_code DONE==");
    result
}

unsafe fn unload_game_code(game_code: &mut GameCode) {
    trace!("==unload_game_code==");
    if !game_code.game_code_dll.is_null() {
        FreeLibrary(game_code.game_code_dll);
        game_code.game_code_dll = zeroed();
    } else {
        warn!("dll memory was already null...")
    }
    game_code.is_valid = false;

    // These lines will crash the program in release mode
    // game_code.update_and_render = zeroed();
    // game_code.get_sound_samples = zeroed();
    trace!("==unload_game_code DONE==")
}

// TODO: investigate XAudio2
// waiting on https://github.com/retep998/winapi-rs/pull/602
// or WASAPI
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

unsafe fn resize_dib_section(buffer: &mut OffscreenBuffer, width: i32, height: i32) {
    if !buffer.memory.is_null() {
        VirtualFree(buffer.memory, 0, MEM_RELEASE);
    }

    buffer.width = width;
    buffer.height = height;

    let bytes_per_pixel = 4;
    buffer.bytes_per_pixel = bytes_per_pixel;

    // When the biHeight field is negative, this is the clue to
    // Windows to treat this bitmap as top-down, not bottom-up, meaning that
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

unsafe fn display_buffer_in_window(buffer: &OffscreenBuffer, device_context: HDC) {
    // For prototyping purposes, we're going to always blit
    // 1-to-1 pixels to make sure we don't introduce artifacts with
    // stretching while we are learning to code the renderer
    StretchDIBits(
        device_context,
        0,
        0,
        buffer.width,
        buffer.height,
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

    match message {
        WM_ACTIVATEAPP => {
            // if w_param > 0 {
            //     SetLayeredWindowAttributes(window, RGB(0, 0, 0), 255, LWA_ALPHA);
            // } else {
            //     SetLayeredWindowAttributes(window, RGB(0, 0, 0), 64, LWA_ALPHA);
            // }
        }
        WM_CLOSE | WM_DESTROY => GLOBAL_RUNNING = false,
        WM_KEYUP | WM_KEYDOWN => panic!("Keyboard input came in through a non-dispatch message!"),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let device_context = BeginPaint(window, &mut paint);
            display_buffer_in_window(&GLOBAL_BACK_BUFFER, device_context);
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
    source_buffer: &mut GameSoundOutputBuffer,
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

fn process_keyboard_message(new_state: &mut GameButtonState, is_down: bool) {
    if new_state.ended_down != is_down {
        new_state.ended_down = is_down;
        new_state.half_transition_count += 1;
    }
}

fn process_xinput_digital_button(
    xinput_button_state: WORD,
    old_state: &GameButtonState,
    button_bit: WORD,
    new_state: &mut GameButtonState,
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

unsafe fn get_input_file_location(
    state: &State,
    input_stream: bool,
    slot_index: usize,
    dest: &mut [u16; MAX_PATH],
) {
    let file_name = format!(
        "loop_edit_{}_{}.rec",
        slot_index,
        if input_stream { "input" } else { "state" }
    );
    build_exe_path_file_name(state, &file_name, dest);
}

unsafe fn get_replay_buffer(state: &mut State, index: usize) -> *mut ReplayBuffer {
    debug_assert!(index < state.replay_buffers.len());
    &mut state.replay_buffers[index]
}

unsafe fn begin_recording_input(state: &mut State, input_recording_index: usize) {
    let replay_buffer = get_replay_buffer(state, input_recording_index);
    if !(*replay_buffer).memory_block.is_null() {
        state.input_recording_index = input_recording_index;

        let mut file_name = zeroed();
        get_input_file_location(state, true, input_recording_index, &mut file_name);
        state.recording_handle = CreateFileW(
            file_name.as_ptr(),
            GENERIC_WRITE,
            0,
            null_mut(),
            CREATE_ALWAYS,
            0,
            null_mut(),
        );
        RtlCopyMemory(
            (*replay_buffer).memory_block as *mut c_void,
            state.game_memory_block as *mut c_void,
            state.total_size as usize,
        );
    } else {
        warn!("Replay buffer memory block was null when trying to begin recording.");
    }
}

unsafe fn end_recording_input(state: &mut State) {
    CloseHandle(state.recording_handle);
    state.input_recording_index = 0;
}

unsafe fn begin_input_playback(state: &mut State, input_playing_index: usize) {
    let replay_buffer = get_replay_buffer(state, input_playing_index);
    if !(*replay_buffer).memory_block.is_null() {
        state.input_playing_index = input_playing_index;

        let mut file_name = zeroed();
        get_input_file_location(state, true, input_playing_index, &mut file_name);
        state.playback_handle = CreateFileW(
            file_name.as_ptr(),
            GENERIC_READ,
            0,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        );
        RtlCopyMemory(
            state.game_memory_block as *mut c_void,
            (*replay_buffer).memory_block as *mut c_void,
            state.total_size as usize,
        );
    } else {
        warn!("Replay buffer memory block was null when trying to begin playback.");
    }
}

unsafe fn end_input_playback(state: &mut State) {
    CloseHandle(state.playback_handle);
    state.input_playing_index = 0;
}

unsafe fn record_input(state: &mut State, new_input: *mut GameInput) {
    let mut bytes_written = 0;
    WriteFile(
        state.recording_handle,
        new_input as *mut c_void,
        size_of::<GameInput>() as u32,
        &mut bytes_written,
        null_mut(),
    );
}

unsafe fn play_back_input(state: &mut State, new_input: *mut GameInput) {
    let mut bytes_read = 0;
    if ReadFile(
        state.playback_handle,
        new_input as *mut c_void,
        size_of::<GameInput>() as u32,
        &mut bytes_read,
        null_mut(),
    ) != 0
        && bytes_read == 0
    {
        // We've hit the end of the stream, go back to the beginning
        let playing_index = state.input_playing_index;
        end_input_playback(state);
        begin_input_playback(state, playing_index);
        ReadFile(
            state.playback_handle,
            new_input as *mut c_void,
            size_of::<GameInput>() as u32,
            &mut bytes_read,
            null_mut(),
        );
    }
}

unsafe fn process_pending_messages(
    state: &mut State,
    keyboard_controller: &mut GameControllerInput,
) {
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
                            debug!("DOWN");
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
                        #[cfg(debug_assertions)]
                        VK_P => {
                            if is_down {
                                GLOBAL_PAUSE = !GLOBAL_PAUSE;
                            }
                        }
                        #[cfg(debug_assertions)]
                        VK_L => {
                            if is_down {
                                if state.input_playing_index == 0 {
                                    if state.input_recording_index == 0 {
                                        debug!("Not recording, starting to record.");
                                        begin_recording_input(state, 1);
                                    } else {
                                        debug!("Recording, starting playback.");
                                        end_recording_input(state);
                                        begin_input_playback(state, 1);
                                    }
                                } else {
                                    debug!("Playing, canceling cycle.");
                                    end_input_playback(state);
                                }
                            }
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

unsafe fn debug_draw_vertical(
    back_buffer: &OffscreenBuffer,
    x: i32,
    top: i32,
    bottom: i32,
    color: u32,
) {
    let actual_top = if top <= 0 { 0 } else { top };

    let actual_bottom = if bottom > back_buffer.height {
        back_buffer.height
    } else {
        bottom
    };

    if (x >= 0) && (x < back_buffer.width) {
        let mut pixel = back_buffer
            .memory
            .offset((x * back_buffer.bytes_per_pixel + actual_top * back_buffer.pitch) as isize)
            as *mut u8;
        #[allow(clippy::cast_ptr_alignment)]
        for _ in actual_top..actual_bottom {
            *(pixel as *mut u32) = color;
            pixel = pixel.offset(back_buffer.pitch as isize);
        }
    }
}

fn debug_draw_sound_buffer_marker(
    back_buffer: &OffscreenBuffer,
    c: f32,
    pad_x: i32,
    top: i32,
    bottom: i32,
    value: DWORD,
    color: u32,
) {
    let x_f32 = c * value as f32;
    let x = pad_x + x_f32 as i32;
    unsafe {
        debug_draw_vertical(back_buffer, x, top, bottom, color);
    }
}

unsafe fn debug_sync_display(
    back_buffer: &OffscreenBuffer,
    markers: &mut [DebugTimeMarker],
    current_marker_index: isize,
    sound_output: &SoundOutput,
) {
    let pad_x = 16;
    let pad_y = 16;

    let line_height = 64;

    let c = (back_buffer.width - 2 * pad_x) as f32 / sound_output.sound_buffer_size as f32;
    for (marker_index, marker) in markers.iter().enumerate() {
        debug_assert!(marker.output_play_cursor < sound_output.sound_buffer_size);
        debug_assert!(marker.output_write_cursor < sound_output.sound_buffer_size);
        debug_assert!(marker.output_location < sound_output.sound_buffer_size);
        debug_assert!(marker.output_byte_count < sound_output.sound_buffer_size);
        debug_assert!(marker.flip_play_cursor < sound_output.sound_buffer_size);
        debug_assert!(marker.flip_write_cursor < sound_output.sound_buffer_size);

        let play_color = 0xFF_FF_FF_FF;
        let write_color = 0xFF_FF_00_00;
        let expected_flip_color = 0xFF_FF_FF_00;
        let play_window_color = 0xFF_FF_00_FF;

        let mut top = pad_y;
        let mut bottom = pad_y + line_height;

        let current_marker_index_u = if current_marker_index < 0 {
            0
        } else {
            current_marker_index
        } as usize;

        if marker_index == current_marker_index_u {
            top += line_height + pad_y;
            bottom += line_height + pad_y;

            let first_top = top;

            debug_draw_sound_buffer_marker(
                back_buffer,
                c,
                pad_x,
                top,
                bottom,
                marker.output_play_cursor,
                play_color,
            );
            debug_draw_sound_buffer_marker(
                back_buffer,
                c,
                pad_x,
                top,
                bottom,
                marker.output_write_cursor,
                write_color,
            );

            top += line_height + pad_y;
            bottom += line_height + pad_y;

            debug_draw_sound_buffer_marker(
                back_buffer,
                c,
                pad_x,
                top,
                bottom,
                marker.output_location,
                play_color,
            );
            debug_draw_sound_buffer_marker(
                back_buffer,
                c,
                pad_x,
                top,
                bottom,
                marker.output_location + marker.output_byte_count,
                write_color,
            );

            top += line_height + pad_y;
            bottom += line_height + pad_y;

            debug_draw_sound_buffer_marker(
                back_buffer,
                c,
                pad_x,
                first_top,
                bottom,
                marker.expected_flip_play_cursor,
                expected_flip_color,
            );
        }

        debug_draw_sound_buffer_marker(
            back_buffer,
            c,
            pad_x,
            top,
            bottom,
            marker.flip_play_cursor,
            play_color,
        );
        debug_draw_sound_buffer_marker(
            back_buffer,
            c,
            pad_x,
            top,
            bottom,
            marker.flip_play_cursor + 1920 * sound_output.bytes_per_sample,
            play_window_color,
        );
        debug_draw_sound_buffer_marker(
            back_buffer,
            c,
            pad_x,
            top,
            bottom,
            marker.flip_write_cursor,
            write_color,
        );
    }
}

// TODO: refactor me and remove this allow
#[allow(clippy::cognitive_complexity)]
pub fn main() {
    unsafe {
        let mut win32_state = zeroed();

        let mut performance_count_frequency_result = zeroed();
        QueryPerformanceFrequency(&mut performance_count_frequency_result);
        GLOBAL_PERF_COUNT_FREQUENCY = *performance_count_frequency_result.QuadPart();

        get_exe_file_name(&mut win32_state);
        let mut source_game_code_dll_full_path: [u16; MAX_PATH] = [0; MAX_PATH];
        build_exe_path_file_name(
            &win32_state,
            "game.dll",
            &mut source_game_code_dll_full_path,
        );
        let mut temp_game_code_dll_full_path: [u16; MAX_PATH] = [0; MAX_PATH];
        build_exe_path_file_name(
            &win32_state,
            "game_temp.dll",
            &mut temp_game_code_dll_full_path,
        );

        // Set the Windows scheduler granularity to 1ms
        // so that our Sleep() can be more granular
        let desired_scheduler_ms = 1;
        let sleep_is_granular = timeBeginPeriod(desired_scheduler_ms) == TIMERR_NOERROR;

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

        resize_dib_section(&mut GLOBAL_BACK_BUFFER, 960, 540);

        if RegisterClassW(&window_class) > 0 {
            let window = CreateWindowExW(
                0, // WS_EX_TOPMOST | WS_EX_LAYERED,
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
                // TODO: How do we reliably query this on Windows?
                let refresh_dc = GetDC(window);
                let refresh_rate = GetDeviceCaps(refresh_dc, VREFRESH);
                ReleaseDC(window, refresh_dc);
                let monitor_refresh_hz = if refresh_rate > 1 { refresh_rate } else { 60 };
                let game_update_hz = monitor_refresh_hz as f32 / 2.0;
                let target_seconds_per_frame = 1.0 / game_update_hz;

                // TODO: Make this like sixty seconds?
                let samples_per_second = 48_000;
                let bytes_per_sample = (size_of::<i16>() * 2) as u32;
                let sound_buffer_size = samples_per_second * bytes_per_sample as u32;
                // TODO: Actually compute this variance and see
                // what the lowest reasonable value is.
                let safety_bytes = (((samples_per_second as f32 * bytes_per_sample as f32)
                    / game_update_hz)
                    / 2.0) as u32;
                let mut sound_output = SoundOutput {
                    running_sample_index: 0,
                    bytes_per_sample,
                    sound_buffer_size,
                    safety_bytes,
                    samples_per_second,
                };
                init_direct_sound(
                    window,
                    sound_output.samples_per_second,
                    sound_output.sound_buffer_size,
                );
                clear_sound_buffer(&sound_output);
                (*GLOBAL_SOUND_BUFFER).Play(0, 0, DSBPLAY_LOOPING);

                GLOBAL_RUNNING = true;

                if false {
                    // This tests the PlayCursor/WriteCursor update frequency
                    // On the my gaming machine, it was 1920 samples.
                    while GLOBAL_RUNNING {
                        let mut play_cursor = zeroed();
                        let mut write_cursor = zeroed();
                        (*GLOBAL_SOUND_BUFFER)
                            .GetCurrentPosition(&mut play_cursor, &mut write_cursor);

                        println!("PC:{} WC:{}", play_cursor, write_cursor);
                    }
                }

                // TODO: Pool with bitmap virtualalloc
                let samples = VirtualAlloc(
                    null_mut(),
                    sound_output.sound_buffer_size as usize,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                ) as *mut i16;

                let base_address: LPVOID = if cfg!(debug_assertions) {
                    null_mut::<VOID>().wrapping_add(terabytes(2))
                } else {
                    null_mut::<VOID>()
                };
                let mut game_memory: GameMemory = zeroed();
                game_memory.permanent_storage_size = megabytes(64);
                game_memory.transient_storage_size = gigabytes(1);

                // TODO: Handle various memory footprints (using system metrics)
                // TODO: Use MEM_LARGE_PAGES and call adjust token
                // privileges when not on Windows XP?
                win32_state.total_size =
                    game_memory.permanent_storage_size + game_memory.transient_storage_size;
                win32_state.game_memory_block = VirtualAlloc(
                    base_address,
                    win32_state.total_size as usize,
                    MEM_RESERVE | MEM_COMMIT,
                    PAGE_READWRITE,
                ) as *mut u8;
                game_memory.permanent_storage = win32_state.game_memory_block;
                game_memory.transient_storage = game_memory
                    .permanent_storage
                    .wrapping_add(game_memory.permanent_storage_size);

                for replay_index in 1..win32_state.replay_buffers.len() {
                    let replay_buffer =
                        (&mut win32_state.replay_buffers[replay_index]) as *mut ReplayBuffer;
                    get_input_file_location(
                        &win32_state,
                        false,
                        replay_index,
                        &mut (*replay_buffer).file_name,
                    );
                    (*replay_buffer).file_handle = CreateFileW(
                        (*replay_buffer).file_name.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        0,
                        null_mut(),
                        CREATE_ALWAYS,
                        0,
                        null_mut(),
                    );
                    if (*replay_buffer).file_handle == INVALID_HANDLE_VALUE {
                        let last_error = GetLastError();
                        error!("Could not create file: {:?}", last_error,);
                    }
                    let mut max_size: LARGE_INTEGER = zeroed();
                    *max_size.QuadPart_mut() = win32_state.total_size as i64;
                    (*replay_buffer).memory_map = CreateFileMappingW(
                        (*replay_buffer).file_handle,
                        null_mut(),
                        PAGE_READWRITE,
                        max_size.u().HighPart as u32,
                        max_size.u().LowPart as u32,
                        null_mut(),
                    );
                    let last_error = GetLastError();
                    if (*replay_buffer).memory_map.is_null() {
                        error!("Could not map file to object: {:?}", last_error);
                    } else if last_error == ERROR_ALREADY_EXISTS {
                        warn!("Object existed before mapping call.");
                    }
                    (*replay_buffer).memory_block = MapViewOfFile(
                        (*replay_buffer).memory_map,
                        FILE_MAP_ALL_ACCESS,
                        0,
                        0,
                        win32_state.total_size as usize,
                    ) as *mut u8;
                    if (*replay_buffer).memory_block.is_null() {
                        let last_error = GetLastError();
                        error!("Replay buffer {} error: {:?}", replay_index, last_error);
                    }
                }

                if !samples.is_null()
                    && !game_memory.permanent_storage.is_null()
                    && !game_memory.transient_storage.is_null()
                {
                    let mut new_input: GameInput = zeroed();
                    let mut old_input: GameInput = zeroed();
                    new_input.seconds_to_advance_over_update = target_seconds_per_frame;

                    let mut last_counter = get_wall_clock();
                    let mut flip_wall_clock = get_wall_clock();

                    let mut debug_time_marker_index = 0;
                    let mut debug_time_markers: [DebugTimeMarker; 30] = zeroed();

                    let mut audio_latency_bytes;
                    let mut audio_latency_seconds;
                    let mut sound_is_valid = false;

                    let mut game = load_game_code(
                        &source_game_code_dll_full_path,
                        &temp_game_code_dll_full_path,
                    );

                    while GLOBAL_RUNNING {
                        let new_dll_write_time =
                            get_last_write_time(&source_game_code_dll_full_path);
                        if CompareFileTime(&new_dll_write_time, &game.dll_last_write_time) != 0 {
                            unload_game_code(&mut game);
                            game = load_game_code(
                                &source_game_code_dll_full_path,
                                &temp_game_code_dll_full_path,
                            );
                        }

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

                        process_pending_messages(
                            &mut win32_state,
                            new_keyboard_controller.as_mut().unwrap(),
                        );

                        if !GLOBAL_PAUSE {
                            let mut mouse_p: POINT = zeroed();
                            GetCursorPos(&mut mouse_p);
                            ScreenToClient(window, &mut mouse_p);
                            new_input.mouse_x = mouse_p.x;
                            new_input.mouse_y = mouse_p.y;
                            new_input.mouse_z = 0;
                            process_keyboard_message(
                                &mut new_input.mouse_buttons[0],
                                GetKeyState(VK_LBUTTON) & (1 << 15) != 0,
                            );
                            process_keyboard_message(
                                &mut new_input.mouse_buttons[1],
                                GetKeyState(VK_MBUTTON) & (1 << 15) != 0,
                            );
                            process_keyboard_message(
                                &mut new_input.mouse_buttons[2],
                                GetKeyState(VK_RBUTTON) & (1 << 15) != 0,
                            );
                            process_keyboard_message(
                                &mut new_input.mouse_buttons[3],
                                GetKeyState(VK_XBUTTON1) & (1 << 15) != 0,
                            );
                            process_keyboard_message(
                                &mut new_input.mouse_buttons[4],
                                GetKeyState(VK_XBUTTON2) & (1 << 15) != 0,
                            );
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
                                    (*new_controller).is_analog = (*old_controller).is_analog;

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
                                    }

                                    if pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0 {
                                        (*new_controller).stick_average_y = -1.0;
                                        (*new_controller).is_analog = false;
                                    }

                                    if pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0 {
                                        (*new_controller).stick_average_x = -1.0;
                                        (*new_controller).is_analog = false;
                                    }

                                    if pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0 {
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

                                    // TODO: How to handle nintendo vs xbox controller differences?
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

                            let mut buffer = GameOffscreenBuffer {
                                memory: GLOBAL_BACK_BUFFER.memory,
                                width: GLOBAL_BACK_BUFFER.width,
                                height: GLOBAL_BACK_BUFFER.height,
                                pitch: GLOBAL_BACK_BUFFER.pitch,
                                bytes_per_pixel: GLOBAL_BACK_BUFFER.bytes_per_pixel,
                            };

                            if win32_state.input_recording_index != 0 {
                                record_input(&mut win32_state, &mut new_input);
                            }

                            if win32_state.input_playing_index != 0 {
                                play_back_input(&mut win32_state, &mut new_input);
                            }
                            (game.update_and_render)(&mut game_memory, &mut new_input, &mut buffer);

                            let audio_wall_clock = get_wall_clock();
                            let from_begin_to_audio_seconds =
                                get_seconds_elapsed(flip_wall_clock, audio_wall_clock);

                            let mut play_cursor = zeroed();
                            let mut write_cursor = zeroed();
                            match (*GLOBAL_SOUND_BUFFER)
                                .GetCurrentPosition(&mut play_cursor, &mut write_cursor)
                            {
                                DS_OK => {
                                    /* Here is how sound output computation works.
                                        We define a safety value that is the number
                                        of samples we think our game update loop
                                        may vary by (let's say up to 2ms)

                                        When we wake up to write audio, we will look
                                        and see what the play cursor position is and we
                                        will forecast ahead where we think the play
                                        cursor will be on the next frame boundary.
                                        We will then look to see if the write cursor is
                                        before that by at least our safety value.  If
                                        it is, the target fill position is that frame
                                        boundary plus one frame.  This gives us perfect
                                        audio sync in the case of a card that has low
                                        enough latency.
                                        If the write cursor is _after_ that safety
                                        margin, then we assume we can never sync the
                                        audio perfectly, so we will write one frame's
                                        worth of audio plus the safety margin's worth
                                        of guard samples.
                                    */
                                    if !sound_is_valid {
                                        sound_output.running_sample_index =
                                            write_cursor / sound_output.bytes_per_sample;
                                        sound_is_valid = true;
                                    }

                                    let byte_to_lock = (sound_output.running_sample_index
                                        * sound_output.bytes_per_sample)
                                        % sound_output.sound_buffer_size;

                                    let expected_sound_bytes_per_frame = ((sound_output
                                        .samples_per_second
                                        * sound_output.bytes_per_sample)
                                        as f32
                                        / game_update_hz)
                                        as u32;

                                    let seconds_left_until_flip =
                                        target_seconds_per_frame - from_begin_to_audio_seconds;
                                    let expected_bytes_until_flip = ((seconds_left_until_flip
                                        / target_seconds_per_frame)
                                        * expected_sound_bytes_per_frame as f32)
                                        as u32;
                                    let expected_frame_boundary_byte =
                                        play_cursor.wrapping_add(expected_bytes_until_flip);

                                    let mut safe_write_cursor = write_cursor;
                                    if safe_write_cursor < play_cursor {
                                        safe_write_cursor += sound_output.sound_buffer_size;
                                    }
                                    debug_assert!(safe_write_cursor >= play_cursor);
                                    safe_write_cursor += sound_output.safety_bytes;

                                    let audio_card_is_low_latency =
                                        safe_write_cursor < expected_frame_boundary_byte;

                                    let target_cursor = if audio_card_is_low_latency {
                                        expected_frame_boundary_byte
                                            .wrapping_add(expected_sound_bytes_per_frame)
                                    } else {
                                        write_cursor
                                            + expected_sound_bytes_per_frame
                                            + sound_output.safety_bytes
                                    } % sound_output.sound_buffer_size;

                                    let bytes_to_write = if byte_to_lock > target_cursor {
                                        (sound_output.sound_buffer_size - byte_to_lock)
                                            + target_cursor
                                    } else {
                                        target_cursor - byte_to_lock
                                    };

                                    let mut sound_buffer = GameSoundOutputBuffer {
                                        samples_per_second: sound_output.samples_per_second,
                                        sample_count: bytes_to_write
                                            / sound_output.bytes_per_sample,
                                        samples,
                                    };
                                    (game.get_sound_samples)(&mut game_memory, &mut sound_buffer);

                                    #[cfg(debug_assertions)]
                                    {
                                        let mut marker =
                                            &mut debug_time_markers[debug_time_marker_index];
                                        marker.output_play_cursor = play_cursor;
                                        marker.output_write_cursor = write_cursor;
                                        marker.output_location = byte_to_lock;
                                        marker.output_byte_count = bytes_to_write;
                                        marker.expected_flip_play_cursor =
                                            expected_frame_boundary_byte;
                                    }

                                    let mut unwrapped_write_cursor = write_cursor;
                                    if unwrapped_write_cursor < play_cursor {
                                        unwrapped_write_cursor += sound_output.sound_buffer_size;
                                    }
                                    audio_latency_bytes = unwrapped_write_cursor - play_cursor;
                                    audio_latency_seconds = (audio_latency_bytes
                                        / sound_output.bytes_per_sample)
                                        / sound_output.samples_per_second;

                                    trace!(
                                        "BTL:{} TC:{} BTW:{} - PC:{} WC:{} DELTA:{} ({}s)",
                                        byte_to_lock,
                                        target_cursor,
                                        bytes_to_write,
                                        play_cursor,
                                        write_cursor,
                                        audio_latency_bytes,
                                        audio_latency_seconds
                                    );

                                    fill_sound_buffer(
                                        &mut sound_output,
                                        byte_to_lock,
                                        bytes_to_write,
                                        &mut sound_buffer,
                                    );
                                }
                                e => {
                                    warn!("Sound invalid: {}", e);
                                    sound_is_valid = false;
                                }
                            }

                            let work_counter = get_wall_clock();
                            let work_seconds_elapsed =
                                get_seconds_elapsed(last_counter, work_counter);

                            // TODO: NOT TESTED YET! PROBABLY BUGGY!!!
                            let mut seconds_elapsed_for_frame = work_seconds_elapsed;
                            if seconds_elapsed_for_frame < target_seconds_per_frame {
                                if sleep_is_granular {
                                    let sleep_ms = 1000.0
                                        * (target_seconds_per_frame - seconds_elapsed_for_frame);
                                    if sleep_ms > 0.0 {
                                        Sleep(sleep_ms as u32);
                                    }
                                }

                                let test_seconds_elapsed_for_frame =
                                    get_seconds_elapsed(last_counter, get_wall_clock());
                                if test_seconds_elapsed_for_frame < target_seconds_per_frame {
                                    warn!("missed sleep");
                                }

                                while seconds_elapsed_for_frame < target_seconds_per_frame {
                                    seconds_elapsed_for_frame =
                                        get_seconds_elapsed(last_counter, get_wall_clock());
                                }
                            } else {
                                trace!("missed frame rate");
                            }

                            let end_counter = get_wall_clock();
                            let ms_per_frame =
                                1000.0 * get_seconds_elapsed(last_counter, end_counter);
                            last_counter = end_counter;

                            // debug_sync_display(
                            //     &GLOBAL_BACK_BUFFER,
                            //     &mut debug_time_markers,
                            //     debug_time_marker_index as isize - 1,
                            //     &sound_output,
                            // );
                            let device_context = GetDC(window);
                            display_buffer_in_window(&GLOBAL_BACK_BUFFER, device_context);
                            ReleaseDC(window, device_context);

                            flip_wall_clock = get_wall_clock();

                            #[cfg(debug_assertions)]
                            {
                                let mut play_cursor = zeroed();
                                let mut write_cursor = zeroed();
                                if (*GLOBAL_SOUND_BUFFER)
                                    .GetCurrentPosition(&mut play_cursor, &mut write_cursor)
                                    == DS_OK
                                {
                                    debug_assert!(
                                        debug_time_marker_index < debug_time_markers.len()
                                    );
                                    let mut marker =
                                        &mut debug_time_markers[debug_time_marker_index];
                                    marker.flip_play_cursor = play_cursor;
                                    marker.flip_write_cursor = write_cursor;
                                }
                            }

                            std::mem::swap(&mut new_input, &mut old_input);

                            let fps = 0.0;

                            trace!("{}ms/f, {}f/s", ms_per_frame, fps);

                            #[cfg(debug_assertions)]
                            {
                                debug_time_marker_index += 1;
                                if debug_time_marker_index == debug_time_markers.len() {
                                    debug_time_marker_index = 0;
                                }
                            }
                        }
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
