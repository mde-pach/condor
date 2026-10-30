//! The long-running modes: `condor query` (and its daemon) and `condor server`.

#[cfg(unix)]
use std::io::BufRead;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use condor::engine::Engine;
use condor::query::Engines;
use condor::run::{Output, run};
use lsp_server::Message;
use serde_json::{Value, json};

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
    // Analysed paths are resolved (`/var` is `/private/var` on macOS).
    condor_core::paths::resolve(&dir, &dir)
}

fn query(engines: &mut Engines, dir: &Path, args: &[&str]) -> (i32, String) {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut out = Output::default();
    let code = engines.query(&argv, dir, &mut out);
    (code, String::from_utf8(out.stdout).unwrap())
}

fn oneshot(dir: &Path) -> (i32, String) {
    let mut out = Output::default();
    let code = run(&[".".to_string()], dir, &mut out);
    (code, String::from_utf8(out.stdout).unwrap())
}

#[test]
fn query_follows_changes_across_files() {
    let dir = project("query_changes", &[("a.py", "def f():\n    pass\n"), ("b.py", "from a import f\nf()\n")]);
    let mut engines = Engines::default();
    assert_eq!(query(&mut engines, &dir, &[]), (0, String::new()));
    assert_eq!(query(&mut engines, &dir, &[]), oneshot(&dir));

    std::fs::write(dir.join("b.py"), "").unwrap();
    let expected = "a.py:1: unused function 'f' (60% confidence)\n";
    assert_eq!(query(&mut engines, &dir, &[]), (3, expected.to_string()));
    assert_eq!(query(&mut engines, &dir, &[]), oneshot(&dir));

    std::fs::write(dir.join("c.py"), "from a import f\nf()\n").unwrap();
    assert_eq!(query(&mut engines, &dir, &[]), (0, String::new()));
    std::fs::remove_file(dir.join("c.py")).unwrap();
    assert_eq!(query(&mut engines, &dir, &[]).0, 3);
}

#[test]
fn query_files_limit_the_report() {
    let dir = project("query_files", &[("a.py", "def f():\n    pass\n"), ("b.py", "def g():\n    pass\n")]);
    let mut engines = Engines::default();
    assert_eq!(query(&mut engines, &dir, &["b.py"]), (3, "b.py:1: unused function 'g' (60% confidence)\n".into()));
    assert_eq!(query(&mut engines, &dir, &["--only", "class", "b.py"]), (0, String::new()));
}

#[test]
fn query_rereads_the_configuration() {
    let dir = project("query_config", &[("a.py", "def f():\n    pass\n")]);
    let mut engines = Engines::default();
    assert_eq!(query(&mut engines, &dir, &[]).0, 3);
    std::fs::write(dir.join("pyproject.toml"), "[tool.condor]\nignore_names = [\"f\"]\n").unwrap();
    assert_eq!(query(&mut engines, &dir, &[]), (0, String::new()));
}

#[test]
fn engine_uses_unsaved_buffers() {
    let dir = project("overlay", &[("a.py", "def f():\n    pass\n"), ("b.py", "")]);
    let config = condor::config::Config { paths: vec![".".into()], ..Default::default() };
    let mut engine = Engine::new(config.clone(), &dir).unwrap();
    let unused = |engine: &mut Engine| {
        engine.refresh(false).unwrap();
        let analysis = engine.analyze(&mut Output::default(), false).unwrap();
        condor::run::select(&analysis, &config, &dir).iter().map(|i| i.name.clone()).collect::<Vec<_>>()
    };
    assert_eq!(unused(&mut engine), ["f"]);
    engine.set_overlay(dir.join("b.py"), Some("from a import f\nf()\n".into()));
    assert!(unused(&mut engine).is_empty());
    engine.set_overlay(dir.join("b.py"), None);
    assert_eq!(unused(&mut engine), ["f"]);
}

#[cfg(unix)]
#[test]
fn query_through_the_daemon() {
    let dir = project("daemon", &[("a.py", "def f():\n    pass\n"), ("b.py", "from a import f\nf()\n")]);
    // A short private directory: socket paths are limited to about 100 bytes.
    let runtime = std::env::temp_dir().join(format!("cdt{}", std::process::id()));
    std::fs::create_dir_all(&runtime).unwrap();
    let condor = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_condor"))
            .args(args)
            .current_dir(&dir)
            .env("XDG_RUNTIME_DIR", &runtime)
            .env("CONDOR_DAEMON_IDLE", "60")
            .output()
            .unwrap();
        (out.status.code().unwrap(), String::from_utf8(out.stdout).unwrap())
    };
    assert_eq!(condor(&["query"]), (0, String::new()));
    let sockets: Vec<PathBuf> = std::fs::read_dir(runtime.join(format!("condor-{}", unsafe { libc::getuid() })))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(sockets.len(), 1, "the first query starts a daemon");

    std::fs::write(dir.join("b.py"), "").unwrap();
    assert_eq!(condor(&["query"]), (3, "a.py:1: unused function 'f' (60% confidence)\n".into()));
    assert_eq!(condor(&["query"]), condor(&["."]));
    assert_eq!(condor(&["query", "b.py"]), (0, String::new()));

    let mut stream = std::os::unix::net::UnixStream::connect(&sockets[0]).unwrap();
    stream.write_all(b"{\"op\": \"shutdown\"}\n").unwrap();
    let mut answer = String::new();
    BufReader::new(&stream).read_line(&mut answer).unwrap();
    for _ in 0..200 {
        if !sockets[0].exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!sockets[0].exists(), "the daemon removes its socket on shutdown");
    let _ = std::fs::remove_dir_all(&runtime);
}

/// A language client talking to `condor server` over stdio.
struct Client {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i32,
}

impl Client {
    fn start(root: &Path) -> Client {
        let root = condor::server::path_to_uri(root).unwrap();
        Client::start_with(json!({"rootUri": root.as_str(), "capabilities": {}}))
    }

    fn start_with(init: Value) -> Client {
        let mut child = Command::new(env!("CARGO_BIN_EXE_condor"))
            .arg("server")
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let reader = BufReader::new(child.stdout.take().unwrap());
        let mut client = Client { child, reader, next_id: 0 };
        let init = client.request("initialize", init);
        assert_eq!(init["serverInfo"]["name"], "condor");
        client.notify("initialized", json!({}));
        client
    }

    fn send(&mut self, message: Message) {
        message.write(&mut self.child.stdin.as_mut().unwrap()).unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(lsp_server::Notification::new(method.into(), params).into());
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id: lsp_server::RequestId = self.next_id.into();
        self.send(lsp_server::Request::new(id.clone(), method.into(), params).into());
        loop {
            if let Message::Response(response) = Message::read(&mut self.reader).unwrap().unwrap() {
                assert_eq!(response.id, id);
                return response.response_result.unwrap();
            }
        }
    }

    /// The next diagnostics published for a file whose URI ends with `name`.
    fn diagnostics(&mut self, name: &str) -> Vec<Value> {
        loop {
            if let Message::Notification(n) = Message::read(&mut self.reader).unwrap().unwrap()
                && n.method == "textDocument/publishDiagnostics" && n.params["uri"].as_str().unwrap().ends_with(name) {
                    return n.params["diagnostics"].as_array().unwrap().clone();
                }
        }
    }
}

#[test]
fn language_server() {
    let dir = project("lsp", &[("a.py", "def f():\n    pass\n\n\ndef g():\n    pass\n"), ("b.py", "from a import f\nf()\n")]);
    let mut client = Client::start(&dir);
    let diags = client.diagnostics("a.py");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0]["message"], "unused function 'g' (60% confidence)");
    assert_eq!(diags[0]["code"], "V103");
    assert_eq!(diags[0]["range"], json!({"start": {"line": 4, "character": 0}, "end": {"line": 4, "character": 8}}));

    // An unsaved edit in another file updates a.py.
    let b = condor::server::path_to_uri(&dir.join("b.py")).unwrap();
    let doc = json!({"uri": b.as_str(), "languageId": "python", "version": 1, "text": "from a import f\nf()\n"});
    client.notify("textDocument/didOpen", json!({"textDocument": doc}));
    let change = json!({"textDocument": {"uri": b.as_str(), "version": 2}, "contentChanges": [{"text": "from a import f, g\nf()\ng()\n"}]});
    client.notify("textDocument/didChange", change);
    assert_eq!(client.diagnostics("a.py"), Vec::<Value>::new());

    let change = json!({"textDocument": {"uri": b.as_str(), "version": 3}, "contentChanges": [{"text": "from a import f\nf()\n"}]});
    client.notify("textDocument/didChange", change);
    let diags = client.diagnostics("a.py");
    assert_eq!(diags.len(), 1);

    let a = condor::server::path_to_uri(&dir.join("a.py")).unwrap();
    let params = json!({
        "textDocument": {"uri": a.as_str()},
        "range": diags[0]["range"],
        "context": {"diagnostics": diags},
    });
    let actions = client.request("textDocument/codeAction", params);
    let edit = &actions[0]["edit"]["changes"][a.as_str()][0];
    assert_eq!(edit["newText"], "  # noqa: V103");
    assert_eq!(edit["range"]["start"], json!({"line": 4, "character": 8}));

    client.request("shutdown", Value::Null);
    client.notify("exit", Value::Null);
    assert!(client.child.wait().unwrap().success());
}

#[test]
fn engine_keeps_the_last_parsed_version_while_typing() {
    let dir = project("typing", &[("a.py", "def f():\n    pass\n"), ("b.py", "from a import f\nf()\n")]);
    let config = condor::config::Config { paths: vec![".".into()], ..Default::default() };
    let mut engine = Engine::new(config.clone(), &dir).unwrap();
    engine.refresh(true).unwrap();
    engine.set_overlay(dir.join("b.py"), Some("from a import f\nf()\nif x\n".into()));
    engine.refresh(false).unwrap();
    let analysis = engine.analyze(&mut Output::default(), false).unwrap();
    assert!(condor::run::select(&analysis, &config, &dir).is_empty(), "b.py still uses f");
    // Unsaved new files count too.
    engine.set_overlay(dir.join("b.py"), None);
    std::fs::write(dir.join("b.py"), "").unwrap();
    engine.set_overlay(dir.join("sub/new.py"), Some("from a import f\nf()\n".into()));
    engine.refresh(true).unwrap();
    let analysis = engine.analyze(&mut Output::default(), false).unwrap();
    assert!(condor::run::select(&analysis, &config, &dir).is_empty(), "sub/new.py uses f");
}

// Windows has no change time to tell such an edit apart.
#[cfg(unix)]
#[test]
fn query_sees_edits_that_restore_size_and_mtime() {
    let dir = project("same_stamp", &[("a.py", "def f():\n    pass\n")]);
    let mut engines = Engines::default();
    assert_eq!(query(&mut engines, &dir, &[]).1, "a.py:1: unused function 'f' (60% confidence)\n");
    let file = std::fs::File::options().write(true).open(dir.join("a.py")).unwrap();
    let modified = file.metadata().unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(dir.join("a.py"), "def g():\n    pass\n").unwrap();
    file.set_modified(modified).unwrap();
    assert_eq!(query(&mut engines, &dir, &[]).1, "a.py:1: unused function 'g' (60% confidence)\n");
}

#[test]
fn language_server_folders_and_noqa() {
    let one = project("folders_one", &[("a.py", "import os  # noqa: V107\r\r\ndef f():\r    pass\r")]);
    let two = project("folders_two", &[("b.py", "def g():\n    pass\n")]);
    let folders: Vec<Value> = [&one, &two]
        .iter()
        .map(|d| json!({"uri": condor::server::path_to_uri(d).unwrap().as_str(), "name": "x"}))
        .collect();
    let mut client = Client::start_with(json!({"workspaceFolders": folders, "capabilities": {}}));
    let mut seen = std::collections::HashMap::new();
    while seen.len() < 2 {
        if let Message::Notification(n) = Message::read(&mut client.reader).unwrap().unwrap()
            && n.method == "textDocument/publishDiagnostics" {
                seen.insert(n.params["uri"].as_str().unwrap().rsplit('/').next().unwrap().to_string(), n.params["diagnostics"].clone());
            }
    }
    assert_eq!(seen["b.py"][0]["message"], "unused function 'g' (60% confidence)");
    let diags = seen["a.py"].as_array().unwrap().clone();
    // Lines end with `\r` (and one `\r\n`): `def f` is on line 2.
    assert_eq!(diags[1]["range"]["start"]["line"], 2);
    let a = condor::server::path_to_uri(&one.join("a.py")).unwrap();
    let params = json!({"textDocument": {"uri": a.as_str()}, "range": diags[0]["range"], "context": {"diagnostics": [diags[0]]}});
    let actions = client.request("textDocument/codeAction", params);
    let edit = &actions[0]["edit"]["changes"][a.as_str()][0];
    assert_eq!(edit["newText"], ",V104");
    assert_eq!(edit["range"]["start"], json!({"line": 0, "character": 23}));
    client.request("shutdown", Value::Null);
    client.notify("exit", Value::Null);
    assert!(client.child.wait().unwrap().success());
}

#[test]
fn language_server_rejects_oversized_messages() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_condor"))
        .arg("server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"Content-Length: 99999999999999999\r\n\r\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("too large"));
}

#[cfg(unix)]
#[test]
fn a_stalled_client_does_not_block_the_daemon() {
    let dir = project("stalled", &[("a.py", "x = 1\n")]);
    let runtime = std::env::temp_dir().join(format!("cds{}", std::process::id()));
    std::fs::create_dir_all(&runtime).unwrap();
    let query = || {
        Command::new(env!("CARGO_BIN_EXE_condor"))
            .arg("query")
            .current_dir(&dir)
            .env("XDG_RUNTIME_DIR", &runtime)
            .env("CONDOR_DAEMON_IDLE", "60")
            .output()
            .unwrap()
    };
    assert_eq!(query().status.code(), Some(3));
    let socket = std::fs::read_dir(runtime.join(format!("condor-{}", unsafe { libc::getuid() })))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut stalled = std::os::unix::net::UnixStream::connect(&socket).unwrap();
    stalled.write_all(b"{\"argv\": [").unwrap();
    let start = std::time::Instant::now();
    assert_eq!(query().status.code(), Some(3));
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
    drop(stalled);
    let mut stop = std::os::unix::net::UnixStream::connect(&socket).unwrap();
    stop.write_all(b"{\"op\": \"shutdown\"}\n").unwrap();
    let mut answer = String::new();
    let _ = BufReader::new(&stop).read_line(&mut answer);
    let _ = std::fs::remove_dir_all(&runtime);
}
