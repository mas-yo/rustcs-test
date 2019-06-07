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
use std::str::FromStr;
use std::str::from_utf8;
use std::net::SocketAddr;
use std::collections::*;
use std::fmt::Debug;
use tokio::codec::{Decoder, Encoder, Framed};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::stream::SplitStream;
use tokio::prelude::stream::SplitSink;
use bytes::BytesMut;
use bytes::BufMut;
use futures::prelude::*;

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

#[derive(Debug, Clone)]
pub struct ObjInfo {
    name: *mut c_char,
    x: i32,
    y: i32,
}
// unsafe impl Send for Box<*const ObjInfo> {}

type DoWorkFn = extern "C" fn(*const c_char, c_int, c_int, *const c_double, *const c_void);

type SetSendFn = extern "C" fn(*const c_void);
type OnRecvFn = extern "C" fn(c_int, c_int);

type SendFn = extern "C" fn(*const ObjInfo, i32);

static mut s_tx : Option<SplitSink<Framed<TcpStream,Codec>>> = None;

extern "C" fn Send(data: *const ObjInfo, size: i32) {
    unsafe {
        if s_tx.is_none() {return;}
        let tx = s_tx.as_mut().unwrap();
        // tx.start_send(S2C::RequestLoginInfo);
        tx.start_send(S2C::ObjList((data,size)));
        tx.poll_complete();
        // sender.send(S2C::ObjList((data,size)));
    }

    // unsafe {
    //     let objs = std::slice::from_raw_parts(data, size as usize);
    //     for o in objs {
    //         // let s = CString::from_raw(o.name);
    //         // println!("{} {} {}", s.into_string().unwrap(), o.x, o.y);
    //     }
    // }
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
        let set_send_fn: *const c_void = ptr::null();
        let METHOD_NAME = cstring("SetSendFn");
        let result = coreclr_create_delegate(
            host_handle,
            domain_id,
            ASSEMBLY_NAME.as_ptr(),
            TYPE_NAME.as_ptr(),
            METHOD_NAME.as_ptr(),
            &set_send_fn,
        );
        if result < 0 {
            eprintln!("coreclr_create_delegate SetSendFn error {:#x}", result);
            return Err(Box::new(SimpleError::new("coreclr_create_delegate error")));
        }
        unsafe {
            let set_send_fn = std::mem::transmute::<*const c_void, SetSendFn>(set_send_fn);
            let sendfn = std::mem::transmute::<SendFn, *const c_void>(Send);

            set_send_fn(sendfn);
        }
    }
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

#[derive(Debug, Clone)]
pub enum C2S {
    // ResponseLoginInfo(String),
    // TouchUI(UIID),
    InputText(String),
    //    EnterRoom,
}

impl FromStr for C2S {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let splitted: Vec<&str> = s.split(',').collect();

        if let Some(cmd) = splitted.get(0) {
            // if *cmd == "response_login_info" {
            //     return Ok(C2S::ResponseLoginInfo(splitted.get(1).unwrap().to_string()));
            // }
            // if *cmd == "touch_ui" {
            //     return Ok(C2S::TouchUI(
            //         splitted.get(1).unwrap().parse::<UIID>().unwrap(),
            //     ));
            // }
            if *cmd == "input_text" {
                return Ok(C2S::InputText(splitted.get(1).unwrap().to_string()));
            }
        }

        Err(())
    }
}

#[derive(Debug, Clone)]
pub enum S2C {
    RequestLoginInfo,
    Message(String),
    ObjList((*const ObjInfo,i32)),
    // ShowUI(UIID, bool),
    // AddText(UIID, String),
}

impl ToString for S2C {
    fn to_string(&self) -> String {
        match self {
            S2C::RequestLoginInfo => "request_login_info".to_string(),
            S2C::Message(msg) => format!("> {}", msg),
            S2C::ObjList((ptr, size)) => {
                unsafe {
                let objs = std::slice::from_raw_parts(ptr, *size as usize);
                println!("sizxe: {}", objs.len());
                let mut s = String::new();
                for o in objs {
                    s.push_str(&format!("{}:{},{}/", "testname", 1, 2));
                }
                s
                }
            }
            // S2C::ShowUI(ui_id, show) => format!("show_ui,{},{}", ui_id, if *show { 1 } else { 0 }),
            // S2C::AddText(ui_id, text) => format!("add_text,{},{}", ui_id, text),
        }
    }
}

#[derive(Default)]
pub struct Codec {
    next_index: usize,
}

impl Codec {
    pub fn new() -> Self {
        Self { next_index: 0 }
    }
}

impl Decoder for Codec {
    type Item = C2S;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        let mut load = 0;
        for i in 0..0 {
            load = load + 1;
        }
        if let Some(newline_offset) = buf[self.next_index..].iter().position(|b| *b == b'\n') {
            let newline_index = newline_offset + self.next_index;

            let line = buf.split_to(newline_index + 1);

            let line = &line[..line.len() - 1];

            let line = from_utf8(&line).expect("invalid utf8 data");

            self.next_index = 0;

            if let Ok(cmd) = C2S::from_str(line) {
                return Ok(Some(cmd));
            }

            panic!("unknown command");
        } else {
            self.next_index = buf.len();

            Ok(None)
        }
    }
}

impl Encoder for Codec {
    type Item = S2C;
    type Error = io::Error;

    fn encode(&mut self, cmd: S2C, buf: &mut BytesMut) -> Result<(), io::Error> {
        // let mut file = File::open("/dev/null").unwrap();
        let mut load = 0;
        for i in 0..0 {
            load = load + 1;
        }

        let mut line = cmd.to_string();

        buf.reserve(line.len() + 1);
        buf.put(line);
        buf.put_u8(b'\n');

        Ok(())
    }
}


pub(crate) fn server(
    addr: &SocketAddr,
    on_recv_fn : OnRecvFn,
)
// -> impl Stream<Item = Framed<TcpStream, Codec>, Error = ()>
// where
//     C:'static + Decoder<Item = C2S, Error = std::io::Error>
//         + Encoder<Item = S2C, Error = std::io::Error>
//         + Default
//         + Send,
{
    let listener = TcpListener::bind(addr).unwrap();

    let server = listener
        .incoming()
        .for_each(move|socket| {
            let framed = Framed::new(socket, Codec::default());
            let (tx,rx) = framed.split();
            // let i:i32 = tx;
            unsafe{
            s_tx = Some(tx);
            }
            let recv = rx.for_each(move|cmd|{
                match cmd {
                    C2S::InputText(txt) => {
                        on_recv_fn(1,2);
                    }
                }
                Ok(())
            })
            .map_err(|_|());
            tokio::spawn(recv);
            Ok(())
        })
        .map(|_|())
        .map_err(|_| println!("tcp incoming error"));

    tokio::run(server);
}

enum AsyncSendItem<P, D> {
    Peer(P),
    SendData(D),
}

fn async_sender<S, I>() -> futures::sync::mpsc::Sender<(Option<u32>, AsyncSendItem<S, I>)>
where
    S: 'static + Send + Sink<SinkItem = I>,
    I: 'static + Send + Clone + Debug,
    S::SinkError: Debug,
{
    let mut peers_tx = HashMap::new();
    let (tx, rx) = futures::sync::mpsc::channel::<(Option<u32>, AsyncSendItem<S, I>)>(1024);
    let task = rx.for_each(move |(peer_id, item)| {
        match item {
            AsyncSendItem::Peer(peer) => {
                if peer_id.is_some() {
                    peers_tx.insert(peer_id.unwrap(), peer.wait());
                }
            }
            AsyncSendItem::SendData(data) => {
                peers_tx.retain(|id, tx| {
                    if peer_id.is_some() && peer_id.unwrap() != *id {
                        return true;
                    }
                    // println!("send {:?} to {}", data, id);
                    if let Err(e) = tx.send(data.clone()) {
                        // println!("send err! {:?}", e);
                        return false;
                    }
                    if let Err(_) = tx.flush() {
                        println!("flush err");
                        return false;
                    }
                    true
                });
            }
        }
        Ok(())
    });

    tokio::spawn(task.map_err(|e| {
        println!("async send error {:?}", e);
    }));

    tx.clone()
}

fn main() -> Result<(), Box<Error>> {
    let (handle, domain_id, on_recv_fn) = load_clr()?;

    let addr = SocketAddr::from_str("192.168.32.243:29180").unwrap();
    server(&addr, on_recv_fn);

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
