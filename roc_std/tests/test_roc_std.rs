#[macro_use]
extern crate pretty_assertions;
// #[macro_use]
// extern crate indoc;
extern crate quickcheck;
extern crate roc_std;

use core::ffi::c_void;

const ROC_SMALL_STR_CAPACITY: usize = core::mem::size_of::<roc_std::RocStr>() - 1;

#[no_mangle]
pub unsafe extern "C" fn roc_alloc(size: usize, _alignment: u32) -> *mut c_void {
    libc::malloc(size)
}

#[no_mangle]
pub unsafe extern "C" fn roc_realloc(
    c_ptr: *mut c_void,
    new_size: usize,
    _old_size: usize,
    _alignment: u32,
) -> *mut c_void {
    libc::realloc(c_ptr, new_size)
}

#[no_mangle]
pub unsafe extern "C" fn roc_dealloc(c_ptr: *mut c_void, _alignment: u32) {
    libc::free(c_ptr)
}

#[cfg(test)]
#[no_mangle]
pub unsafe extern "C" fn roc_panic(c_ptr: *mut c_void, tag_id: u32) {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    match tag_id {
        0 => {
            let c_str = CStr::from_ptr(c_ptr as *const c_char);
            let string = c_str.to_str().unwrap();
            panic!("roc_panic during test: {}", string);
        }
        _ => todo!(),
    }
}

#[cfg(test)]
#[no_mangle]
pub unsafe extern "C" fn roc_memcpy(dst: *mut c_void, src: *mut c_void, n: usize) -> *mut c_void {
    libc::memcpy(dst, src, n)
}

#[cfg(test)]
#[no_mangle]
pub unsafe extern "C" fn roc_memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void {
    libc::memset(dst, c, n)
}

#[cfg(test)]
mod test_roc_std {
    use roc_std::RocResult;
    use roc_std::RocStr;

    fn roc_str_byte_representation(string: &RocStr) -> [u8; RocStr::SIZE] {
        unsafe { core::mem::transmute_copy(string) }
    }

    #[test]
    fn roc_str_empty() {
        let actual = roc_str_byte_representation(&RocStr::empty());

        let mut expected = [0u8; RocStr::SIZE];
        expected[RocStr::SIZE - 1] = RocStr::MASK;

        assert_eq!(actual, expected);
    }

    #[test]
    fn roc_str_single_char() {
        let actual = roc_str_byte_representation(&RocStr::from("a"));

        let mut expected = [0u8; RocStr::SIZE];
        expected[0] = b'a';
        expected[RocStr::SIZE - 1] = RocStr::MASK | 1;

        assert_eq!(actual, expected);
    }

    #[test]
    fn roc_str_max_small_string() {
        let s = str::repeat("a", RocStr::SIZE - 1);
        let actual = roc_str_byte_representation(&RocStr::from(s.as_str()));

        let mut expected = [0u8; RocStr::SIZE];
        expected[..RocStr::SIZE - 1].copy_from_slice(s.as_bytes());
        expected[RocStr::SIZE - 1] = RocStr::MASK | s.len() as u8;

        assert_eq!(actual, expected);
    }

    #[test]
    fn empty_string_from_str() {
        let a = RocStr::from("");
        let b = RocStr::empty();

        assert_eq!(a, b);
    }

    #[test]
    fn empty_string_length() {
        let string = RocStr::from("");

        assert_eq!(string.len(), 0);
    }

    #[test]
    fn empty_string_capacity() {
        let string = RocStr::empty();

        assert_eq!(string.capacity(), super::ROC_SMALL_STR_CAPACITY);
    }

    #[test]
    fn roc_result_to_rust_result() {
        let greeting = "Hello, World!";
        let roc_result: RocResult<String, ()> = RocResult::ok(greeting.into());

        match roc_result.into() {
            Ok(answer) => {
                assert_eq!(answer.as_str(), greeting);
            }
            Err(()) => {
                panic!("Received an Err when Ok was expected.")
            }
        }
    }

    #[test]
    fn roc_result_is_ok() {
        let greeting = "Hello, World!";
        let roc_result: RocResult<String, ()> = RocResult::ok(greeting.into());

        assert!(roc_result.is_ok());
        assert!(!roc_result.is_err());
    }

    #[test]
    fn roc_result_is_err() {
        let greeting = "Hello, World!";
        let roc_result: RocResult<(), String> = RocResult::err(greeting.into());

        assert!(!roc_result.is_ok());
        assert!(roc_result.is_err());
    }
}

#[cfg(test)]
mod temp_c_str {
    use core::slice;
    use roc_std::RocStr;
    use std::ffi::CStr;

    fn verify_temp_c(string: &str) {
        // temp_c_utf8
        {
            let roc_str = RocStr::from(string);
            let answer = roc_str.utf8_nul_terminated(|ptr, len| {
                let bytes = unsafe { slice::from_raw_parts(ptr.cast(), len + 1) };
                let c_str = CStr::from_bytes_with_nul(bytes).unwrap();

                assert_eq!(c_str.to_str(), Ok(string));

                42
            });

            assert_eq!(Ok(42), answer);
        }

        // temp_c_utf16
        {
            let roc_str = RocStr::from(string);
            let answer = roc_str.utf16_nul_terminated(|ptr, len| {
                let bytes = unsafe { slice::from_raw_parts(ptr.cast(), len + 1) };

                // Verify that it's nul-terminated
                assert_eq!(bytes[len], 0);

                let string = String::from_utf16(&bytes[0..len]).unwrap();

                assert_eq!(string.as_str(), string);

                42
            });

            assert_eq!(Ok(42), answer);
        }
    }

    #[test]
    fn empty_string() {
        verify_temp_c("");
    }

    /// e.g. "1" or "12" or "12345" etc.
    fn string_for_len(len: usize) -> String {
        let bytes: Vec<u8> = (1..=len as u8).collect();

        assert_eq!(bytes.len(), len);

        // The bytes should contain no nul characters.
        assert!(bytes.iter().all(|byte| *byte != 0));

        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn small_strings() {
        for len in 1..=super::ROC_SMALL_STR_CAPACITY {
            verify_temp_c(&string_for_len(len));
        }
    }

    #[test]
    fn no_excess_capacity() {
        // This is small enough that it should be a stack allocation for UTF-8
        verify_temp_c(&string_for_len(33));

        // This is big enough that it should be a heap allocation for UTF-8 and UTF-16
        verify_temp_c(&string_for_len(65));
    }
}
