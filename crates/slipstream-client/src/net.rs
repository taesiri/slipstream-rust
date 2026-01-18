#[cfg(windows)]
pub use windows_sys::Win32::Networking::WinSock::{
    SOCKADDR as Sockaddr, SOCKADDR_STORAGE as SockaddrStorage,
};

#[cfg(not(windows))]
pub use libc::{sockaddr as Sockaddr, sockaddr_storage as SockaddrStorage};
