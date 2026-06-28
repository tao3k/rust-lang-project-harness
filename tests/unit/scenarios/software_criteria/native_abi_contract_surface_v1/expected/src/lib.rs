//! Fixture with a native ABI layout and its co-located ABI contract.

pub mod native_abi {
    pub const NATIVE_ABI_VERSION: u32 = 1;
    pub const NATIVE_ABI_ID: &str = "native-abi-contract-surface.v1";
    pub const NATIVE_HEADER_PATH: &str = "include/native_abi.h";
    pub const NATIVE_HEADER_SOURCE: &str = include_str!("../include/native_abi.h");

    /// Borrowed UTF-8 bytes passed across the native C ABI.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct NativeUtf8 {
        pub ptr: *const u8,
        pub len: usize,
    }
}
