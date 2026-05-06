use std::os::raw::{c_int, c_long};

pub const GRG_OK: c_int = 0;

pub type GrgCtx = *mut std::ffi::c_void;
pub type GrgKey = *mut std::ffi::c_void;

unsafe extern "C" {
    /// Initialise a context with default crypto settings.
    /// `header` is the 3-char magic string (e.g. b"GRG\0").
    pub fn grg_context_initialize_defaults(header: *const u8) -> GrgCtx;

    pub fn grg_context_free(gctx: GrgCtx);

    /// Update context crypto parameters from an existing file.
    pub fn grg_update_gctx_from_file(gctx: GrgCtx, path: *const u8) -> c_int;

    /// Generate a key from a passphrase (hashed internally).
    pub fn grg_key_gen(pwd: *const u8, pwd_len: c_int) -> GrgKey;

    pub fn grg_key_free(gctx: GrgCtx, key: GrgKey);

    /// Decrypt a file, allocating `*orig_data` (caller must free with `grg_free`).
    pub fn grg_decrypt_file(
        gctx: GrgCtx,
        key: GrgKey,
        path: *const u8,
        orig_data: *mut *mut u8,
        orig_dim: *mut c_long,
    ) -> c_int;

    /// Encrypt `orig_data` (length `orig_dim`) and write it to `path`.
    pub fn grg_encrypt_file(
        gctx: GrgCtx,
        key: GrgKey,
        path: *const u8,
        orig_data: *const u8,
        orig_dim: c_long,
    ) -> c_int;

    /// Free memory allocated by libgringotts (e.g. the buffer from `grg_decrypt_file`).
    pub fn grg_free(gctx: GrgCtx, alloc_data: *mut std::ffi::c_void, dim: c_long);
}
