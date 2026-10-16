//! Isolated, read-only spike. Not linked into the application or a recovery policy.
//! Build: rustc --edition 2021 main.rs -o boot-identity-spike.exe
use std::mem::{offset_of, size_of};

#[repr(C)]
#[derive(Default)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
#[derive(Default)]
struct BootEnvironment {
    id: Guid,
    firmware_type: u32,
    flags: u64,
}

struct Observation {
    boot_id: Option<String>,
    reason: &'static str,
    ntstatus: Option<i32>,
    returned_bytes: u32,
}

fn decode(status: i32, returned: u32, info: &BootEnvironment) -> Observation {
    let reason = if status != 0 {
        "nt-query-failed"
    } else if returned != size_of::<BootEnvironment>() as u32 {
        "unsupported-layout"
    } else if info.firmware_type > 2 {
        "unsupported-firmware-type"
    } else if info.id.data1 == 0
        && info.id.data2 == 0
        && info.id.data3 == 0
        && info.id.data4 == [0; 8]
    {
        "empty-boot-id"
    } else {
        "observed"
    };
    let id = &info.id;
    let boot_id = (reason == "observed").then(|| {
        format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            id.data1, id.data2, id.data3, id.data4[0], id.data4[1], id.data4[2],
            id.data4[3], id.data4[4], id.data4[5], id.data4[6], id.data4[7]
        )
    });
    Observation { boot_id, reason, ntstatus: Some(status), returned_bytes: returned }
}

#[cfg(windows)]
fn observe(class: u32) -> Observation {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }
    type Query = unsafe extern "system" fn(u32, *mut c_void, u32, *mut u32) -> i32;
    // Ntdll is already loaded. Resolve at runtime; absence is not reboot evidence.
    let name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
    let module = unsafe { GetModuleHandleW(name.as_ptr()) };
    if module.is_null() {
        return Observation { boot_id: None, reason: "ntdll-unavailable", ntstatus: None, returned_bytes: 0 };
    }
    let address = unsafe { GetProcAddress(module, b"NtQuerySystemInformation\0".as_ptr()) };
    if address.is_null() {
        return Observation { boot_id: None, reason: "query-unavailable", ntstatus: None, returned_bytes: 0 };
    }
    let query: Query = unsafe { std::mem::transmute(address) };
    let mut info = BootEnvironment::default();
    let mut returned = 0;
    let status = unsafe {
        query(class, (&mut info as *mut BootEnvironment).cast(), size_of::<BootEnvironment>() as u32, &mut returned)
    };
    decode(status, returned, &info)
}

#[cfg(not(windows))]
fn observe(_: u32) -> Observation {
    Observation { boot_id: None, reason: "unsupported-platform", ntstatus: None, returned_bytes: 0 }
}

fn main() {
    if std::env::args_os().len() != 1 {
        eprintln!("Usage: boot-identity-spike (read-only JSON snapshot; no arguments)");
        std::process::exit(2);
    }
    let start = std::time::Instant::now();
    let result = observe(90); // SystemBootEnvironmentInformation, decimal 90 (0x5A).
    let boot_id = result.boot_id.as_ref().map(|id| format!("\"{id}\"")).unwrap_or("null".into());
    let status = result.ntstatus.map(|s| format!("\"0x{:08x}\"", s as u32)).unwrap_or("null".into());
    println!(
        "{{\"schemaVersion\":1,\"algorithm\":\"nt-boot-environment-v1\",\"platform\":\"{}\",\"arch\":\"{}\",\"pid\":{},\"status\":\"{}\",\"reason\":\"{}\",\"bootId\":{},\"ntstatus\":{},\"returnedBytes\":{},\"expectedBytes\":{},\"bootFlagsOffset\":{},\"queryMicros\":{}}}",
        std::env::consts::OS, std::env::consts::ARCH, std::process::id(),
        if result.boot_id.is_some() { "observed" } else { "unknown" },
        result.reason, boot_id, status, result.returned_bytes,
        size_of::<BootEnvironment>(), offset_of!(BootEnvironment, flags), start.elapsed().as_micros()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> BootEnvironment {
        BootEnvironment { id: Guid { data1: 0x12345678, data2: 0x9abc, data3: 0xdef0, data4: [1,2,3,4,5,6,7,8] }, firmware_type: 2, flags: 0 }
    }
    #[test]
    fn abi_and_guid_byte_order() {
        assert_eq!(size_of::<BootEnvironment>(), 32);
        assert_eq!(offset_of!(BootEnvironment, flags), 24);
        assert_eq!(decode(0, 32, &fixture()).boot_id.as_deref(), Some("12345678-9abc-def0-0102-030405060708"));
    }
    #[test]
    fn api_error_and_unknown_layout_never_become_a_boot_identity() {
        assert_eq!(decode(0xc0000003u32 as i32, 32, &fixture()).reason, "nt-query-failed");
        for bytes in [0, 16, 24, 31, 33, u32::MAX] {
            assert_eq!(decode(0, bytes, &fixture()).reason, "unsupported-layout");
        }
        assert_eq!(decode(0, 32, &BootEnvironment::default()).reason, "empty-boot-id");
        let mut info = fixture(); info.firmware_type = 3;
        assert_eq!(decode(0, 32, &info).reason, "unsupported-firmware-type");
    }
    #[cfg(windows)]
    #[test]
    fn native_invalid_information_class_is_unknown() {
        let result = observe(0x7fff_ffff);
        assert!(result.boot_id.is_none());
        assert_eq!(result.reason, "nt-query-failed");
    }
    #[cfg(windows)]
    #[test]
    fn native_queries_are_stable_within_the_process() {
        let first = observe(90);
        assert!(first.boot_id.is_some(), "{}; status {:?}; returned {}", first.reason, first.ntstatus, first.returned_bytes);
        for _ in 0..32 { assert_eq!(observe(90).boot_id, first.boot_id); }
    }
}
