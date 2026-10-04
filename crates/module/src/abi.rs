//! The three names the server and the module know each other by
//! (DECISIONS 102), and the one place in the workspace where `unsafe` is
//! written: to export two functions and to import one.
//!
//! No pointer is read here. The server writes a call into a buffer this
//! module owns and sized, and the module reads its own buffer; a reply goes
//! out as an address and a length the server reads within the module's
//! memory, or refuses.

use std::sync::Mutex;

/// Where the server writes a call, sized by [`reserve`] first.
static INBOX: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[link(wasm_import_module = "host")]
#[allow(unsafe_code)]
unsafe extern "C" {
    /// Hands the server one reply, whole. It is read before this returns.
    safe fn reply(at: *const u8, len: usize);
}

/// Sizes the inbox for a call of `len` bytes, and says where it is.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn reserve(len: usize) -> *mut u8 {
    let mut inbox = INBOX.lock().expect("the server calls one call at a time");
    inbox.clear();
    inbox.resize(len, 0);
    inbox.as_mut_ptr()
}

/// Serves the call in the inbox, and hands back every reply to it. The call
/// is taken out of the inbox first: run again with nothing written, it
/// serves nothing.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn call() {
    let call = std::mem::take(&mut *INBOX.lock().expect("the server calls one call at a time"));
    for said in crate::serve(&call) {
        reply(said.as_ptr(), said.len());
    }
}
