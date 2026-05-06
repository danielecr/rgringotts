mod ffi;

use std::ffi::CString;

use thiserror::Error;

/// A single gringotts entry (title + body).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub id: usize,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Error)]
pub enum GringottsError {
    #[error("Failed to initialise libgringotts context")]
    ContextInit,
    #[error("Failed to decrypt file (code {0})")]
    Decrypt(i32),
    #[error("Failed to encrypt file (code {0})")]
    Encrypt(i32),
    #[error("Data contains invalid UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("XML parse error: {0}")]
    Xml(String),
    #[error("File path contains a NUL byte")]
    InvalidPath,
}

/// Decrypt a gringotts file and return its parsed entries.
///
/// # Safety
/// All unsafe calls are wrapped here; callers receive a safe `Vec<Entry>`.
pub fn load_file(file_path: &str, passphrase: &str) -> Result<Vec<Entry>, GringottsError> {
    let path = CString::new(file_path).map_err(|_| GringottsError::InvalidPath)?;
    let pwd = passphrase.as_bytes();

    unsafe {
        // Initialise context with the "GRG" magic header used by gringotts.
        let ctx = ffi::grg_context_initialize_defaults(b"GRG\0".as_ptr());
        if ctx.is_null() {
            return Err(GringottsError::ContextInit);
        }

        // Read encryption parameters stored inside the file.
        let rc = ffi::grg_update_gctx_from_file(ctx, path.as_ptr() as *const u8);
        if rc != ffi::GRG_OK {
            ffi::grg_context_free(ctx);
            return Err(GringottsError::Decrypt(rc));
        }

        let key = ffi::grg_key_gen(pwd.as_ptr(), pwd.len() as i32);
        if key.is_null() {
            ffi::grg_context_free(ctx);
            return Err(GringottsError::ContextInit);
        }

        let mut data_ptr: *mut u8 = std::ptr::null_mut();
        let mut data_len: i64 = 0;

        let rc = ffi::grg_decrypt_file(
            ctx,
            key,
            path.as_ptr() as *const u8,
            &mut data_ptr,
            &mut data_len,
        );

        if rc != ffi::GRG_OK {
            ffi::grg_key_free(ctx, key);
            ffi::grg_context_free(ctx);
            return Err(GringottsError::Decrypt(rc));
        }

        let xml = if !data_ptr.is_null() && data_len > 0 {
            let slice = std::slice::from_raw_parts(data_ptr, data_len as usize);
            let s = std::str::from_utf8(slice)?.to_owned();
            ffi::grg_free(ctx, data_ptr.cast(), data_len);
            s
        } else {
            if !data_ptr.is_null() {
                ffi::grg_free(ctx, data_ptr.cast(), data_len);
            }
            String::new()
        };

        ffi::grg_key_free(ctx, key);
        ffi::grg_context_free(ctx);

        parse_xml(&xml)
    }
}

/// Serialise `entries` as XML and encrypt them into `file_path`.
///
/// # Safety
/// All unsafe calls are wrapped here.
pub fn save_file(file_path: &str, passphrase: &str, entries: &[Entry]) -> Result<(), GringottsError> {
    let xml = serialize_xml(entries);
    let path = CString::new(file_path).map_err(|_| GringottsError::InvalidPath)?;
    let pwd = passphrase.as_bytes();

    unsafe {
        let ctx = ffi::grg_context_initialize_defaults(b"GRG\0".as_ptr());
        if ctx.is_null() {
            return Err(GringottsError::ContextInit);
        }

        let key = ffi::grg_key_gen(pwd.as_ptr(), pwd.len() as i32);
        if key.is_null() {
            ffi::grg_context_free(ctx);
            return Err(GringottsError::ContextInit);
        }

        let bytes = xml.as_bytes();
        let rc = ffi::grg_encrypt_file(
            ctx,
            key,
            path.as_ptr() as *const u8,
            bytes.as_ptr(),
            bytes.len() as i64,
        );

        ffi::grg_key_free(ctx, key);
        ffi::grg_context_free(ctx);

        if rc != ffi::GRG_OK {
            return Err(GringottsError::Encrypt(rc));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// XML helpers
// ---------------------------------------------------------------------------

fn parse_xml(xml: &str) -> Result<Vec<Entry>, GringottsError> {
    if xml.trim().is_empty() {
        return Ok(vec![]);
    }

    // libgringotts stores concatenated <entry> elements without a root.
    let wrapped = format!("<root>{}</root>", xml);

    let mut reader = quick_xml::Reader::from_str(&wrapped);
    reader.config_mut().trim_text(true);

    let mut entries = Vec::new();
    let mut buf = Vec::new();
    let mut in_entry = false;
    let mut in_title = false;
    let mut in_body = false;
    let mut current_title = String::new();
    let mut current_body = String::new();
    let mut id: usize = 0;

    loop {
        use quick_xml::events::Event;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"entry" => {
                    in_entry = true;
                    current_title.clear();
                    current_body.clear();
                }
                b"title" if in_entry => in_title = true,
                b"body" if in_entry => in_body = true,
                _ => {}
            },
            Ok(Event::Text(e)) => {
                let text = e.unescape().map_err(|e| GringottsError::Xml(e.to_string()))?;
                if in_title {
                    current_title = text.into_owned();
                } else if in_body {
                    current_body = text.into_owned();
                }
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"title" => in_title = false,
                b"body" => in_body = false,
                b"entry" => {
                    in_entry = false;
                    entries.push(Entry {
                        id,
                        title: current_title.clone(),
                        body: current_body.clone(),
                    });
                    id += 1;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(GringottsError::Xml(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

fn serialize_xml(entries: &[Entry]) -> String {
    let mut xml = String::new();
    for entry in entries {
        let t = quick_xml::escape::escape(&entry.title);
        let b = quick_xml::escape::escape(&entry.body);
        xml.push_str(&format!(
            "\n<entry>\n<title>{t}</title>\n<body>{b}</body>\n</entry>"
        ));
    }
    xml
}
