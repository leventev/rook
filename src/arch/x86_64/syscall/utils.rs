use core::{slice, str::from_utf8};

use alloc::string::String;

// TODO
pub fn get_userspace_string(ptr: *const u8, len: usize) -> Option<String> {
    if ptr.is_null() || len == 0 {
        return None;
    }
    
    let str = unsafe {
        let str = slice::from_raw_parts(ptr, len);
        // TODO: handle utf8 parse error
        from_utf8(str).unwrap()
    };

    // TODO: check if the memory we are copying from is valid
    Some(String::from(str))
}
