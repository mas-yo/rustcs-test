use libc::{dlopen, dlsym};
use libloading;
use simple_error::SimpleError;
use std::env::*;
use std::error::*;
use std::ffi::CString;
use std::fs::{self, DirEntry};
use std::io;
use std::mem;
use std::os::raw::*;
use std::path::Path;
use std::prelude::*;
use std::ptr;
use std::time::{Duration, Instant};

fn visit_dirs<F>(dir: &Path, cb: &mut F) -> io::Result<()>
where
    F: FnMut(&DirEntry),
{
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

fn cstring<S: AsRef<str>>(s: S) -> CString {
    CString::new(s.as_ref()).unwrap()
}

type CoreClrInitFn = extern "C" fn(
    *const c_char,
    *const c_char,
    c_int,
    *const *const c_char,
    *const *const c_char,
    *const *const c_void,
    *const c_uint,
) -> c_int;
/*
CORECLR_HOSTING_API(coreclr_initialize,
            const char* exePath,
            const char* appDomainFriendlyName,
            int propertyCount,
            const char** propertyKeys,
            const char** propertyValues,
            void** hostHandle,
            unsigned int* domainId);
*/

type CoreClrShutDownFn = extern "C" fn(*const c_void, c_uint) -> c_int;
/*
CORECLR_HOSTING_API(coreclr_shutdown,
            void* hostHandle,
            unsigned int domainId);
*/

type CoreClrCreateDelegateFn = extern "C" fn(
    *const c_void,
    c_uint,
    *const c_char,
    *const c_char,
    *const c_char,
    *const *const c_void,
) -> c_int;
/*
CORECLR_HOSTING_API(coreclr_create_delegate,
            void* hostHandle,
            unsigned int domainId,
            const char* entryPointAssemblyName,
            const char* entryPointTypeName,
            const char* entryPointMethodName,
            void** delegate);
*/

struct ObjInfo {
    name: *mut c_char,
    x: i32,
    y: i32,
}

type DoWorkFn = extern "C" fn(*const c_char, c_int, c_int, *const c_double, *const c_void);

type OnRecvFn = extern "C" fn(c_int, c_int, *const c_void);

type SendFn = extern "C" fn(*const ObjInfo, i32);

extern "C" fn Send(data: *const ObjInfo, size: i32) {
    unsafe {
        let objs = std::slice::from_raw_parts(data, size as usize);
        for o in objs {
            // let s = CString::from_raw(o.name);
            // println!("{} {} {}", s.into_string().unwrap(), o.x, o.y);
        }
    }
}

//typedef char* (*doWork_ptr)(const char* jobName, int iterations, int dataSize, double* data, report_callback_ptr callbackFunction);
type CallBackFn = extern "C" fn(i32) -> i32;

#[no_mangle]
extern "C" fn ReportProgressCallback(i: i32) -> i32 {
    println!("callback {}", i);
    32
}
fn load_clr() -> Result<(*const c_void, c_uint, OnRecvFn), Box<Error>> {
    let libcoreclr =
        libloading::Library::new("/home/myoshida/dev/rustcs-test/game/bin/libcoreclr.so")?;
    // libloading::Library::new("/home/myoshida/dev/dotnet-samples/core/hosting/HostWithCoreClrHost/bin/linux/libcoreclr.so");
    // if libcoreclr.is_err() {
    //     eprintln!("failed to load libcoreclr.so");
    //     return 1;
    // }

    // let libcoreclr = libcoreclr.unwrap();

    let coreclr_initialize: libloading::Symbol<CoreClrInitFn> =
        unsafe { libcoreclr.get(b"coreclr_initialize\0").unwrap() };

    let coreclr_create_delegate: libloading::Symbol<CoreClrCreateDelegateFn> =
        unsafe { libcoreclr.get(b"coreclr_create_delegate\0").unwrap() };

    let coreclr_shutdown: libloading::Symbol<CoreClrShutDownFn> =
        unsafe { libcoreclr.get(b"coreclr_shutdown\0").unwrap() };

    let mut tpa_list = String::new();
    visit_dirs(
        Path::new("/home/myoshida/dev/rustcs-test/game/bin"),
        // Path::new("/home/myoshida/dev/dotnet-samples/core/hosting/HostWithCoreClrHost/bin/linux"),
        &mut |dir| {
            let path = dir.path().to_str().unwrap().to_string();
            if path.ends_with(".dll") {
                let path = format!("{}:", path);
                tpa_list.push_str(path.as_ref());
            }
        },
    );

    // println!("{}", tpa_list);

    let tpa_list = cstring(tpa_list);

    let TRUSTED_PLATFORM_ASSEMBLIES = cstring("TRUSTED_PLATFORM_ASSEMBLIES");

    let property_keys: [*const c_char; 1] = [TRUSTED_PLATFORM_ASSEMBLIES.as_ptr()];

    let property_values: [*const c_char; 1] = [tpa_list.as_ptr()];

    let host_handle: *const c_void = ptr::null();
    let domain_id: c_uint = 0;

    let self_path = cstring(current_exe().unwrap().to_str().unwrap());
    let Game = cstring("main");

    let result = coreclr_initialize(
        self_path.as_ptr(),
        Game.as_ptr(),
        1_i32,
        property_keys.as_ptr(),
        property_values.as_ptr(),
        &host_handle,
        &domain_id,
    );

    if result < 0 {
        eprintln!("core clr start error");
        return Err(Box::new(SimpleError::new("core clr start error")));
    }

    let ASSEMBLY_NAME = cstring("game, Version=1.0.0.0"); //, Culture=neutral, PublicKeyToken=null");//("game, Version=1.0.0.0");
    let TYPE_NAME = cstring("game.Class1");

    {
        let on_recv: *const c_void = ptr::null();
        let METHOD_NAME = cstring("OnReceive");
        let result = coreclr_create_delegate(
            host_handle,
            domain_id,
            ASSEMBLY_NAME.as_ptr(),
            TYPE_NAME.as_ptr(),
            METHOD_NAME.as_ptr(),
            &on_recv,
        );
        if result < 0 {
            eprintln!("coreclr_create_delegate error {:#x}", result);
            return Err(Box::new(SimpleError::new("coreclr_create_delegate error")));
        }
        unsafe {
            let on_recv = std::mem::transmute::<*const c_void, OnRecvFn>(on_recv);
            let sendfn = std::mem::transmute::<SendFn, *const c_void>(Send);

            return Ok((host_handle, domain_id, on_recv));
            // let start = Instant::now();
            // for i in 0..1000000 {
            //     on_recv(10, 20, sendfn);
            // }
            // let end = start.elapsed();
            // println!("time: {}", end.as_millis());
        }
    }

    // let result = coreclr_shutdown(host_handle, domain_id);
    // if result < 0 {
    //     eprintln!("coreclr_shutdown error {:#x}", result);
    //     return 1;
    // }
}

fn main() -> Result<(), Box<Error>> {
    let (handle, domain_id, on_recv_fn) = load_clr()?;

    Ok(())

    // let lib = libloading::Library::new("/home/myoshida/dev/rustcs-test/base/target/debug/libbase.so");
    // if lib.is_err() {
    //     eprintln!("failed to load libbase.so");
    //     std::process::exit(1);
    // }

    // let lib = lib.unwrap();

    // let start : Result<libloading::Symbol<extern fn() -> i32>, _> = unsafe {
    //     lib.get("start\0".as_bytes())
    // };

    // if start.is_err() {
    //     eprintln!("failed to get symbol 'start'");
    //     std::process::exit(1);
    // }

    // let start = start.unwrap();
    // let result = start();
    // if result != 0 {
    //     eprintln!("function 'start' error {}", result);
    // }
    // std::process::exit(result);
}
