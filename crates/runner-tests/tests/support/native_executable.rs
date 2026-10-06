use std::{io, path::Path};

pub fn install(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<()> {
    let source = source.as_ref();
    let destination = destination.as_ref();
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        // A clone has its own executable vnode without an open writable
        // descriptor that a concurrent fork could inherit.
        let from = CString::new(source.as_os_str().as_bytes())?;
        let to = CString::new(destination.as_os_str().as_bytes())?;
        if unsafe { libc::clonefile(from.as_ptr(), to.as_ptr(), 0) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    std::fs::hard_link(source, destination)
}
