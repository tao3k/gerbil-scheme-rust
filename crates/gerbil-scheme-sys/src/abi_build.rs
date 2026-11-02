//! Checked UTF-8 transport for the native, owner-local package build/test.
use crate::{GerbilBorrowedUtf8, GerbilRootId, GerbilStatus};
use std::{ffi::CString, os::raw::c_char};

unsafe extern "C" {
    fn gerbil_scheme_rust_package_build_cstr(
        directory: *const c_char,
        result: *mut i32,
    ) -> GerbilStatus;
    fn gerbil_scheme_rust_package_clean_cstr(
        directory: *const c_char,
        result: *mut i32,
    ) -> GerbilStatus;
    fn gerbil_scheme_rust_package_test_cstr(
        directory: *const c_char,
        tests: *const c_char,
        result: *mut i32,
    ) -> GerbilStatus;
}

unsafe fn directory_string(value: GerbilBorrowedUtf8) -> Result<CString, GerbilStatus> {
    if value.ptr.is_null() && value.len != 0 {
        return Err(GerbilStatus::NullPointer);
    }
    if value.len == 0 || value.len > isize::MAX as usize {
        return Err(GerbilStatus::InvalidValue);
    }
    // SAFETY: callers retain the readable span for this call.
    let text = unsafe { value.as_str() }.map_err(|_| GerbilStatus::InvalidValue)?;
    CString::new(text).map_err(|_| GerbilStatus::InvalidValue)
}

/// Execute the package's ordinary `build.ss` main in this live native owner.
/// The operation result is separate from transport status; failure leaves it unchanged.
/// # Safety
/// The default build runtime must be live on its owner thread. Spans must remain
/// readable and result must be null or writable and not overlap those spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_build(
    directory: GerbilBorrowedUtf8,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let directory = match unsafe { directory_string(directory) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    // SAFETY: checked strings and output remain live through the native call.
    unsafe { gerbil_scheme_rust_package_build_cstr(directory.as_ptr(), result) }
}

/// Invoke the package-owned ordinary clean action without a CLI child.
/// # Safety
/// Same owner, readable-span and output requirements as `package_build`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_clean(
    directory: GerbilBorrowedUtf8,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let directory = match unsafe { directory_string(directory) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    // SAFETY: checked strings and output remain live through the owner call.
    unsafe { gerbil_scheme_rust_package_clean_cstr(directory.as_ptr(), result) }
}

/// Run native `GxTest` without starting another Gerbil command-line process.
/// # Safety
/// Same runtime, readable-span and output ownership requirements as `package_build`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_test(
    directory: GerbilBorrowedUtf8,
    tests: GerbilBorrowedUtf8,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let directory = match unsafe { directory_string(directory) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let tests = match unsafe { directory_string(tests) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    // SAFETY: checked strings and output remain live through the native call.
    unsafe { gerbil_scheme_rust_package_test_cstr(directory.as_ptr(), tests.as_ptr(), result) }
}

unsafe extern "C" {
    fn gerbil_scheme_rust_package_paths_cstr(
        prefix: *const c_char,
        libraries: *const c_char,
        result: *mut i32,
    ) -> GerbilStatus;
    fn gerbil_scheme_rust_package_main_cstr(
        directory: *const c_char,
        arguments: *const *const c_char,
        count: u32,
        result: *mut i32,
    ) -> GerbilStatus;
}
/// Invoke the owner-local package paths operation with checked borrowed text.
/// # Safety
/// The tooling image must be live on its owner thread. All input spans must
/// be readable and the nonnull output writable and disjoint for this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_paths(
    prefix: GerbilBorrowedUtf8,
    libraries: GerbilBorrowedUtf8,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let prefix = match unsafe { directory_string(prefix) } {
        Ok(text) => text,
        Err(status) => return status,
    };
    let libraries = match unsafe { directory_string(libraries) } {
        Ok(text) => text,
        Err(status) => return status,
    };
    // SAFETY: converted strings and output remain live for the complete call.
    unsafe { gerbil_scheme_rust_package_paths_cstr(prefix.as_ptr(), libraries.as_ptr(), result) }
}
/// Invoke `build.ss` main with its ordinary arguments, without evaluating caller code.
/// # Safety
/// Same owner/span/output rules as `package_build`. The argument array must be
/// readable for count elements; each nested UTF-8 span must also be readable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_main(
    directory: GerbilBorrowedUtf8,
    arguments: *const GerbilBorrowedUtf8,
    count: usize,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() || (count != 0 && arguments.is_null()) {
        return GerbilStatus::NullPointer;
    }
    let Ok(length) = u32::try_from(count) else {
        return GerbilStatus::InvalidValue;
    };
    if count > isize::MAX as usize / std::mem::size_of::<GerbilBorrowedUtf8>() {
        return GerbilStatus::InvalidValue;
    }
    let directory = match unsafe { directory_string(directory) } {
        Ok(text) => text,
        Err(status) => return status,
    };
    // SAFETY: caller guarantees readable elements; empty spans avoid null slice construction.
    let arguments = if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(arguments, count) }
    };
    let mut strings = Vec::with_capacity(count);
    for argument in arguments {
        let text = if argument.is_empty() {
            CString::default()
        } else {
            // SAFETY: the caller retains this nonempty argument span.
            match unsafe { directory_string(*argument) } {
                Ok(text) => text,
                Err(status) => return status,
            }
        };
        strings.push(text);
    }
    let pointers: Vec<_> = strings.iter().map(|value| value.as_ptr()).collect();
    // SAFETY: all strings, pointer array and output live until foreign return.
    unsafe {
        gerbil_scheme_rust_package_main_cstr(directory.as_ptr(), pointers.as_ptr(), length, result)
    }
}

unsafe extern "C" {
    fn gerbil_scheme_rust_package_load_closure_cstr(
        imports: GerbilRootId,
        output: *const c_char,
        result: *mut i32,
    ) -> GerbilStatus;
    fn gerbil_scheme_rust_package_build_spec_cstr(
        directory: *const c_char,
        specification: *const c_char,
        result: *mut i32,
    ) -> GerbilStatus;
    fn gerbil_scheme_rust_package_stage_cstr(
        source: *const c_char,
        output: *const c_char,
        imports: GerbilRootId,
        result: *mut i32,
    ) -> GerbilStatus;
}

/// Compile an existing build.ss specification through native std/make.
/// # Safety
/// Same live-owner, span and output requirements as `package_build`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_build_spec(
    directory: GerbilBorrowedUtf8,
    specification: GerbilBorrowedUtf8,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let directory = match unsafe { directory_string(directory) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let specification = match unsafe { directory_string(specification) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    unsafe {
        gerbil_scheme_rust_package_build_spec_cstr(
            directory.as_ptr(),
            specification.as_ptr(),
            result,
        )
    }
}

/// Call the existing native programme graph producer with rooted imports.
/// # Safety
/// Same live-owner and spans as `package_build`. Imports must be a live root in
/// that owner; result must be writable and must not overlap the borrowed spans.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_stage(
    source: GerbilBorrowedUtf8,
    output: GerbilBorrowedUtf8,
    imports: GerbilRootId,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let source = match unsafe { directory_string(source) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let output = match unsafe { directory_string(output) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    unsafe {
        gerbil_scheme_rust_package_stage_cstr(source.as_ptr(), output.as_ptr(), imports, result)
    }
}

/// Stage the existing native expander load closure from rooted module symbols.
/// # Safety
/// Imports must belong to the live owner. Output and result obey `package_build`
/// borrowing requirements and must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gerbil_scheme_rust_package_load_closure(
    imports: GerbilRootId,
    output: GerbilBorrowedUtf8,
    result: *mut i32,
) -> GerbilStatus {
    if result.is_null() {
        return GerbilStatus::NullPointer;
    }
    let output = match unsafe { directory_string(output) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    unsafe { gerbil_scheme_rust_package_load_closure_cstr(imports, output.as_ptr(), result) }
}
