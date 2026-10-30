//! `condor daemon`: keeps a project's analysis in memory and answers
//! `condor query` over a Unix socket, so each query only rescans the files
//! that changed. `condor query` starts it on demand; it exits after
//! [`IDLE`] without queries (`CONDOR_DAEMON_IDLE`, in seconds, overrides it).
//!
//! The socket is `condor-<uid>/<key>.sock` in `$XDG_RUNTIME_DIR` or the
//! temporary directory, private to the user, with one daemon per executable,
//! build and working directory. A request is one line of JSON,
//! `{"argv": [...]}`, answered with `{"code": .., "stdout": .., "stderr": ..}`;
//! `{"op": "shutdown"}` stops the daemon.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::query::Engines;
use crate::run::Output;

/// How long a daemon waits for a query before exiting.
pub const IDLE: Duration = Duration::from_secs(30 * 60);

/// How long `condor query` waits for a daemon it started.
const STARTUP: Duration = Duration::from_secs(5);

/// The longest socket path every platform accepts (`sun_path`).
const MAX_SOCKET_PATH: usize = 100;

fn fnv1a(parts: &[&[u8]]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.iter().chain(&[0]) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// The socket for a daemon serving `cwd`, or `None` if there is no safe
/// place for it.
pub fn socket_path(cwd: &Path) -> Option<PathBuf> {
    // SAFETY: getuid has no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    // An empty or relative XDG_RUNTIME_DIR is invalid and ignored (XDG spec).
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(format!("condor-{uid}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return None,
    }
    // Refuse a directory another user could have prepared.
    let meta = std::fs::symlink_metadata(&dir).ok()?;
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
        return None;
    }
    // A rebuilt or reinstalled executable gets its own daemon.
    let exe = std::env::current_exe().ok()?;
    let built = std::fs::metadata(&exe).ok()?.modified().ok()?;
    let built = built.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos().to_le_bytes();
    let key = fnv1a(&[
        exe.as_os_str().as_encoded_bytes(),
        &built,
        env!("CARGO_PKG_VERSION").as_bytes(),
        cwd.as_os_str().as_encoded_bytes(),
    ]);
    let path = dir.join(format!("{key:016x}.sock"));
    (path.as_os_str().len() <= MAX_SOCKET_PATH).then_some(path)
}

fn idle_timeout() -> Duration {
    std::env::var("CONDOR_DAEMON_IDLE").ok().and_then(|s| s.parse().ok()).map_or(IDLE, Duration::from_secs)
}

/// Send one request and read the answer.
fn request(stream: UnixStream, body: &Value) -> Option<Value> {
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(600))).ok()?;
    let mut writer = &stream;
    writer.write_all(format!("{body}\n").as_bytes()).ok()?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}

/// Answer `condor query ARGV` through the daemon for `cwd`, starting it if
/// needed. `None` if that fails, so the caller scans in-process.
pub fn ask(argv: &[String], cwd: &Path, out: &mut Output) -> Option<i32> {
    let path = socket_path(cwd)?;
    let stream = match UnixStream::connect(&path) {
        Ok(stream) => stream,
        Err(_) => {
            Command::new(std::env::current_exe().ok()?)
                .arg("daemon")
                .current_dir(cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .ok()?;
            let start = Instant::now();
            loop {
                if let Ok(stream) = UnixStream::connect(&path) {
                    break stream;
                }
                if start.elapsed() > STARTUP {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    };
    let answer = request(stream, &json!({ "argv": argv }))?;
    let code = i32::try_from(answer.get("code")?.as_i64()?).ok()?;
    out.stdout.extend_from_slice(answer.get("stdout")?.as_str()?.as_bytes());
    out.stderr.extend_from_slice(answer.get("stderr")?.as_str()?.as_bytes());
    Some(code)
}

/// `condor daemon`: serve queries for the current directory until idle.
pub fn main(cwd: &Path) -> i32 {
    let Some(path) = socket_path(cwd) else {
        eprintln!("Error: no private directory for the daemon's socket");
        return 1;
    };
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            // Another daemon serves this directory, or is binding it right
            // now; otherwise the socket is left over from a crash.
            for _ in 0..5 {
                if UnixStream::connect(&path).is_ok() {
                    return 0;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = std::fs::remove_file(&path);
            match UnixListener::bind(&path) {
                Ok(listener) => listener,
                Err(_) => return 0,
            }
        }
        Err(e) => {
            eprintln!("Error: cannot listen on {}: {e}", path.display());
            return 1;
        }
    };

    // (last activity, busy): the watchdog exits once idle and not busy.
    let activity = Arc::new(Mutex::new((Instant::now(), false)));
    let idle = idle_timeout();
    {
        let activity = Arc::clone(&activity);
        let path = path.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(idle.min(Duration::from_secs(1)));
                let state = activity.lock().unwrap();
                if !state.1 && state.0.elapsed() >= idle {
                    let _ = std::fs::remove_file(&path);
                    std::process::exit(0);
                }
            }
        });
    }

    let mut engines = Engines::default();
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        *activity.lock().unwrap() = (Instant::now(), true);
        let stop = serve(&stream, &mut engines, cwd);
        *activity.lock().unwrap() = (Instant::now(), false);
        if stop {
            break;
        }
    }
    let _ = std::fs::remove_file(&path);
    0
}

/// Answer one connection; `true` asks the daemon to stop.
fn serve(stream: &UnixStream, engines: &mut Engines, cwd: &Path) -> bool {
    // A client that stalls must not block the others.
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
    let mut line = String::new();
    if BufReader::new(stream).read_line(&mut line).is_err() {
        return false;
    }
    let Ok(body) = serde_json::from_str::<Value>(&line) else { return false };
    if body.get("op").and_then(Value::as_str) == Some("shutdown") {
        let mut writer = stream;
        let _ = writer.write_all(b"{}\n");
        return true;
    }
    let argv: Vec<String> = body
        .get("argv")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut out = Output::default();
    let code = engines.query(&argv, cwd, &mut out);
    let answer = json!({
        "code": code,
        "stdout": String::from_utf8_lossy(&out.stdout),
        "stderr": String::from_utf8_lossy(&out.stderr),
    });
    let mut writer = stream;
    let _ = writer.write_all(format!("{answer}\n").as_bytes());
    false
}
