use core::{mem::zeroed, ptr::null_mut};
use winapi::{
    ctypes::c_void,
    shared::{minwindef::*, winerror::*},
    um::{
        errhandlingapi::GetLastError, fileapi::*, libloaderapi::*, minwinbase::*, winbase::*,
        winnt::*, winuser::*,
    },
};

pub fn copy_file_overwrite(source_path: &[u16; MAX_PATH], dest_path: &[u16; MAX_PATH]) {
    let result = unsafe { CopyFileW(source_path.as_ptr(), dest_path.as_ptr(), FALSE) };

    if result == 0 {
        panic!("CopyFileW failed");
    }
    if unsafe { GetLastError() == ERROR_FILE_NOT_FOUND } {
        panic!("ERROR_FILE_NOT_FOUND");
    }

    match unsafe { GetLastError() } {
        ERROR_FILE_NOT_FOUND => panic!("ERROR_FILE_NOT_FOUND"),
        ERROR_ACCESS_DENIED => panic!("ERROR_ACCESS_DENIED"),
        code => panic!("Error code: {}", code),
    }
}

pub fn get_file_attributes(file_name: &[u16; MAX_PATH]) -> WIN32_FILE_ATTRIBUTE_DATA {
    unsafe {
        let mut data: WIN32_FILE_ATTRIBUTE_DATA = zeroed();
        let result = GetFileAttributesExW(
            file_name.as_ptr(),
            GetFileExInfoStandard,
            &mut data as *mut WIN32_FILE_ATTRIBUTE_DATA as *mut c_void,
        );

        if result == 0 {
            panic!("GetFileAttributesExW failed");
        }

        data
    }
}

pub fn get_module_file_name() -> [u16; MAX_PATH] {
    let mut file_name = [0u16; MAX_PATH];
    let result = unsafe { GetModuleFileNameW(null_mut(), &mut file_name[0], MAX_PATH as u32) };

    if result == 0 {
        panic!("GetModuleFileNameW failed");
    }
    if unsafe { GetLastError() == ERROR_INSUFFICIENT_BUFFER } {
        panic!("ERROR_INSUFFICIENT_BUFFER");
    }

    file_name
}

pub fn get_proc_address(h_module: HMODULE, proc_name: LPCSTR) -> FARPROC {
    let result = unsafe { GetProcAddress(h_module, proc_name) };
    if result.is_null() {
        panic!("GetProcAddress failed");
    }

    result
}

pub fn load_library(path: &[u16; MAX_PATH]) -> HMODULE {
    let result = unsafe { LoadLibraryW(path.as_ptr()) };
    if result.is_null() {
        panic!("LoadLibraryW failed");
    }

    result
}

pub fn peek_message_remove() -> Option<MSG> {
    unsafe {
        let mut message = zeroed();
        if PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
            Some(message)
        } else {
            None
        }
    }
}
