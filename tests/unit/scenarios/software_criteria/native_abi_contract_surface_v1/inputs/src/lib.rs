//! Fixture with a native ABI layout but no co-located ABI contract.

pub mod native_abi {
    /// Borrowed UTF-8 bytes passed across the native C ABI.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct NativeUtf8 {
        pub ptr: *const u8,
        pub len: usize,
    }
}
