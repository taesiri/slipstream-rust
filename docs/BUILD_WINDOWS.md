# Windows Build Notes (Slipstream Client)

This document captures the Windows build steps and the changes made to get
`slipstream-client` compiling and running on Windows.

## Build Prereqs

- Visual Studio 2022 with C++ build tools (MSVC)
- CMake (in PATH)
- Rust stable toolchain (MSVC target)
- vcpkg

## One-Time Setup

1) Install vcpkg and OpenSSL:

```powershell
git clone https://github.com/microsoft/vcpkg.git D:\vcpkg
D:\vcpkg\bootstrap-vcpkg.bat
D:\vcpkg\vcpkg.exe install openssl:x64-windows-static-md
```

2) Ensure the vcpkg root is visible when building:

```powershell
$env:VCPKG_ROOT="D:\vcpkg"
```

## Build (Debug)

```powershell
cargo build -p slipstream-client
```

Binary output:
`D:\slipstream-rust\target\debug\slipstream-client.exe`

## Build (Release)

```powershell
$env:VCPKG_ROOT="D:\vcpkg"
cargo build -p slipstream-client --release
```

Binary output:
`D:\slipstream-rust\target\release\slipstream-client.exe`

## Run Example

```powershell
.\slipstream-client.exe --tcp-listen-port 1080 --resolver 1.1.1.1:53 --domain example.com
```

## Notes

- The resolver/domain must be reachable; if the QUIC session closes immediately,
  verify the DNS server or domain is responding.
- The Windows binary is typically self-contained. If a machine is missing the
  MSVC runtime, install the Microsoft Visual C++ Redistributable.

## Changes Made for Windows

### Build system / tooling

- Use `cc::Build` for C sources and Windows-friendly compilation.
- Run picoquic auto-build via direct CMake on Windows.
- Discover `pkgconf` from vcpkg and provide `OPENSSL_ROOT_DIR` to CMake.
- Build only required picoquic/picotls targets on Windows to avoid failing tests.
- Skip `pthread` and `ssl`/`crypto` link directives on Windows.
- Teach picoquic lib discovery about Windows `.lib` and `Debug/Release` dirs.

### C / Picoquic tweaks

- Add `Ws2tcpip.h` include for `wincompat.h`.
- Ensure picoquic packet loop headers pull in `wincompat.h` on Windows.
- Unify `picoquic_packet_loop_v3` signature with `picoquic_thread_return_t`.
- Use `_WINDOWS`/`WIN32` defines for MSVC C compilation.
- Fix MSVC static assert in `picotls_layout.c`.
- Add `wintimeofday` implementation for Windows.
- Fix WinSock include order in `wincompat_time.c`.
- Make Windows detection in `picoquic.h` accept `_WIN32`.

### Rust adjustments

- Use `windows-sys` socket types on Windows; keep `libc` for non-Windows.
- Add Windows-only socket conversion helpers in `slipstream-ffi`.
- Add Windows-only socket type aliases in `slipstream-client`.
- Bind UDP to IPv4 on Windows to avoid IPv6/dual-stack issues.
- Increase UDP receive buffer size to avoid `WSAEMSGSIZE`.
- Cap MTU on Windows to reduce oversized UDP payloads.
- Disable dual-stack address normalization on Windows.

