//! `condor server [OPTIONS] [PATHS]`: a language server over stdio. It shows
//! the unused code of the whole workspace as warnings, updated as files are
//! edited, and offers a `# noqa` quick fix. Options and paths are those of
//! the command line; without paths, the workspace is scanned (`.`).
//!
//! Syntax errors are left to other tools (ruff, the editor's Python support).

use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use condor_core::Item;
use condor_core::paths::resolve;
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Diagnostic, DiagnosticSeverity, DiagnosticTag,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams,
    NumberOrString, Position, PublishDiagnosticsParams, Range, TextEdit, Uri, WorkspaceEdit,
};
use serde_json::json;

use crate::args;
use crate::config::{self, Config, Outcome};
use crate::engine::Engine;
use crate::run::{self, Output};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// `condor server`: serve the editor on stdin and stdout.
pub fn main(argv: &[String]) -> i32 {
    let (connection, io_threads) = Connection::stdio();
    let result = serve(&connection, argv);
    drop(connection);
    let joined = io_threads.join();
    match result.and(joined.map_err(Into::into)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("condor server: {e}");
            1
        }
    }
}

fn serve(connection: &Connection, argv: &[String]) -> Result<()> {
    let (id, params) = connection.initialize_start()?;
    let init: InitializeParams = serde_json::from_value(params)?;
    #[allow(deprecated)] // root_uri: still sent by clients without workspace folders
    let root_uri = init.workspace_folders.as_ref().and_then(|f| f.first()).map(|f| f.uri.clone()).or(init.root_uri);
    let root = match root_uri.as_ref().and_then(uri_to_path) {
        Some(root) => root,
        None => std::env::current_dir()?,
    };
    connection.initialize_finish(
        id,
        json!({
            "capabilities": {
                "textDocumentSync": {"openClose": true, "change": 1, "save": true},
                "codeActionProvider": {"codeActionKinds": ["quickfix"]},
            },
            "serverInfo": {"name": "condor", "version": env!("CARGO_PKG_VERSION")},
        }),
    )?;
    let watch = init
        .capabilities
        .workspace
        .and_then(|w| w.did_change_watched_files)
        .and_then(|d| d.dynamic_registration)
        .unwrap_or(false);
    if watch {
        // Pick up files changed outside the editor (git checkout, formatters).
        let params = json!({"registrations": [{
            "id": "condor-watch",
            "method": "workspace/didChangeWatchedFiles",
            "registerOptions": {"watchers": [{"globPattern": "**/*.py"}, {"globPattern": "**/*.{toml,json}"}]},
        }]});
        let request = Request::new("condor-watch".to_string().into(), "client/registerCapability".into(), params);
        connection.sender.send(request.into())?;
    }

    let mut server = Server::new(root, argv.to_vec());
    server.update(connection)?;
    while let Ok(first) = connection.receiver.recv() {
        // Handle everything already queued before analysing again.
        let mut dirty = false;
        for message in std::iter::once(first).chain(connection.receiver.try_iter().collect::<Vec<_>>()) {
            match message {
                Message::Request(request) => {
                    if connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    if dirty {
                        server.update(connection)?;
                        dirty = false;
                    }
                    let response = server.request(request);
                    connection.sender.send(response.into())?;
                }
                Message::Notification(notification) => dirty |= server.notify(notification),
                Message::Response(_) => {}
            }
        }
        if dirty {
            server.update(connection)?;
        }
    }
    Ok(())
}

struct Server {
    root: PathBuf,
    /// `root` with symbolic links resolved, as in the analysed paths.
    real_root: PathBuf,
    argv: Vec<String>,
    engine: Option<Engine>,
    /// Open documents: their URI as the editor names them, and their text.
    documents: HashMap<PathBuf, (Uri, String)>,
    published: HashMap<PathBuf, Vec<Diagnostic>>,
    error: Option<String>,
}

impl Server {
    fn new(root: PathBuf, argv: Vec<String>) -> Server {
        let real_root = resolve(&root, &root);
        Server { root, real_root, argv, engine: None, documents: HashMap::new(), published: HashMap::new(), error: None }
    }

    fn config(&self) -> std::result::Result<Config, String> {
        let mut cli = args::parse(&self.argv).map_err(|e| e.stderr.trim().to_string())?;
        cli.default_paths = vec![".".into()];
        match config::make_config(cli, &self.root, None) {
            Outcome::Config(config, _) => Ok(config),
            Outcome::InputError(msg, _) => Err(msg),
            Outcome::Exit(exit) => Err(exit.stderr.trim().to_string()),
        }
    }

    /// Bring the engine up to date with the configuration and the files.
    fn engine(&mut self) -> std::result::Result<&mut Engine, String> {
        let config = self.config()?;
        let stale = self.engine.as_ref().is_none_or(|e| e.config != config || e.plugins_changed());
        if stale {
            let mut engine = Engine::new(config, &self.root)?;
            for (path, (_, text)) in &self.documents {
                engine.set_overlay(path.clone(), Some(text.clone()));
            }
            self.engine = Some(engine);
        }
        let engine = self.engine.as_mut().expect("set above");
        engine.refresh()?;
        Ok(engine)
    }

    /// Analyse and publish the diagnostics that changed.
    fn update(&mut self, connection: &Connection) -> Result<()> {
        let mut diagnostics: HashMap<PathBuf, Vec<Diagnostic>> = HashMap::new();
        match self.engine() {
            Ok(engine) => {
                let analysis = engine.analyze(&mut Output::default(), false).unwrap_or_default();
                let (config, cwd) = (&engine.config, &engine.cwd);
                let mut lines: HashMap<PathBuf, Vec<String>> = HashMap::new();
                for item in run::select(&analysis, config, cwd) {
                    let path = item.file.to_path_buf();
                    let text = lines.entry(path.clone()).or_insert_with(|| {
                        let text = engine.overlay(&path).map(String::from).or_else(|| std::fs::read_to_string(&path).ok());
                        text.unwrap_or_default().lines().map(String::from).collect()
                    });
                    diagnostics.entry(path).or_default().push(diagnostic(item, text));
                }
                self.error = None;
            }
            Err(msg) => {
                // Shown once; the previous diagnostics stay until it is fixed.
                if self.error.as_ref() != Some(&msg) {
                    let params = json!({"type": 1, "message": format!("condor: {msg}")});
                    connection.sender.send(Notification::new("window/showMessage".into(), params).into())?;
                    self.error = Some(msg);
                }
                return Ok(());
            }
        }
        let cleared: Vec<PathBuf> = self.published.keys().filter(|p| !diagnostics.contains_key(*p)).cloned().collect();
        for path in cleared {
            self.publish(connection, &path, Vec::new())?;
        }
        for (path, diags) in diagnostics {
            if self.published.get(&path) != Some(&diags) {
                self.publish(connection, &path, diags)?;
            }
        }
        Ok(())
    }

    fn publish(&mut self, connection: &Connection, path: &Path, diagnostics: Vec<Diagnostic>) -> Result<()> {
        let uri = match self.documents.get(path) {
            Some((uri, _)) => uri.clone(),
            None => path_to_uri(&self.shown(path)).ok_or("path is not a valid URI")?,
        };
        let params = PublishDiagnosticsParams { uri, diagnostics: diagnostics.clone(), version: None };
        connection.sender.send(Notification::new("textDocument/publishDiagnostics".into(), params).into())?;
        if diagnostics.is_empty() {
            self.published.remove(path);
        } else {
            self.published.insert(path.to_path_buf(), diagnostics);
        }
        Ok(())
    }

    /// An analysed path as the editor knows it (below the workspace root as
    /// opened, not its resolved form).
    fn shown(&self, path: &Path) -> PathBuf {
        match path.strip_prefix(&self.real_root) {
            Ok(rel) => self.root.join(rel),
            Err(_) => path.to_path_buf(),
        }
    }

    /// Handle a notification; `true` if the analysis must be updated.
    fn notify(&mut self, notification: Notification) -> bool {
        match notification.method.as_str() {
            "textDocument/didOpen" => {
                let Ok(p) = serde_json::from_value::<DidOpenTextDocumentParams>(notification.params) else { return false };
                self.set_document(p.text_document.uri, Some(p.text_document.text))
            }
            "textDocument/didChange" => {
                let Ok(p) = serde_json::from_value::<DidChangeTextDocumentParams>(notification.params) else { return false };
                // Full synchronisation: the last change is the whole text.
                let Some(change) = p.content_changes.into_iter().last() else { return false };
                self.set_document(p.text_document.uri, Some(change.text))
            }
            "textDocument/didClose" => {
                let Ok(p) = serde_json::from_value::<DidCloseTextDocumentParams>(notification.params) else { return false };
                self.set_document(p.text_document.uri, None)
            }
            "textDocument/didSave" | "workspace/didChangeWatchedFiles" => true,
            _ => false,
        }
    }

    fn set_document(&mut self, uri: Uri, text: Option<String>) -> bool {
        let Some(path) = uri_to_path(&uri) else { return false };
        let path = resolve(&path, &self.root);
        match &text {
            Some(text) => {
                self.documents.insert(path.clone(), (uri, text.clone()));
            }
            None => {
                self.documents.remove(&path);
            }
        }
        if let Some(engine) = &mut self.engine {
            engine.set_overlay(path, text);
        }
        true
    }

    fn request(&mut self, request: Request) -> Response {
        if request.method != "textDocument/codeAction" {
            return Response::new_err(request.id, lsp_server::ErrorCode::MethodNotFound as i32, "unsupported".into());
        }
        let Ok(params) = serde_json::from_value::<CodeActionParams>(request.params) else {
            return Response::new_err(request.id, lsp_server::ErrorCode::InvalidParams as i32, "invalid".into());
        };
        let actions = uri_to_path(&params.text_document.uri)
            .map(|path| self.code_actions(&params, &resolve(&path, &self.root)))
            .unwrap_or_default();
        Response::new_ok(request.id, actions)
    }

    /// `# noqa: <code>` on the line of each condor diagnostic in range.
    fn code_actions(&self, params: &CodeActionParams, path: &Path) -> Vec<CodeActionOrCommand> {
        let text = match self.documents.get(path) {
            Some((_, text)) => text.clone(),
            None => std::fs::read_to_string(path).unwrap_or_default(),
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut actions = Vec::new();
        for diag in &params.context.diagnostics {
            let Some(NumberOrString::String(code)) = &diag.code else { continue };
            if diag.source.as_deref() != Some("condor") {
                continue;
            }
            let line = diag.range.start.line;
            let Some(text) = lines.get(line as usize) else { continue };
            if condor_python::noqa::search(text).is_some() {
                continue;
            }
            let end = Position::new(line, utf16_len(text));
            let edit = TextEdit::new(Range::new(end, end), format!("  # noqa: {code}"));
            let changes = HashMap::from([(params.text_document.uri.clone(), vec![edit])]);
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Ignore with `# noqa: {code}`"),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                edit: Some(WorkspaceEdit::new(changes)),
                ..CodeAction::default()
            }));
        }
        actions
    }
}

fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}

/// A warning on the item's first line (all its lines for unreachable code),
/// shown as unnecessary code.
fn diagnostic(item: &Item, lines: &[String]) -> Diagnostic {
    let first = item.first_line.saturating_sub(1);
    let last = if item.kind == condor_core::ItemKind::UnreachableCode { item.last_line.saturating_sub(1) } else { first };
    let line = |n: u32| lines.get(n as usize).map_or("", |l| l.trim_end_matches('\r'));
    let first_text = line(first);
    let indent = first_text.len() - first_text.trim_start().len();
    Diagnostic {
        range: Range::new(Position::new(first, utf16_len(&first_text[..indent])), Position::new(last, utf16_len(line(last)))),
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(item.kind.noqa_code().to_string())),
        source: Some("condor".into()),
        message: format!("{} ({}% confidence)", item.message(), item.confidence),
        tags: Some(vec![DiagnosticTag::UNNECESSARY]),
        ..Diagnostic::default()
    }
}

/// The path of a `file:` URI.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    let rest = uri.as_str().strip_prefix("file://")?;
    // Skip the authority (`localhost` or empty).
    let path = &rest[rest.find('/')?..];
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let path = String::from_utf8(decoded).ok()?;
    // `/C:/x` on Windows.
    if cfg!(windows) && path.as_bytes().get(2) == Some(&b':') {
        return Some(PathBuf::from(&path[1..]));
    }
    Some(PathBuf::from(path))
}

/// The `file:` URI of an absolute path.
pub fn path_to_uri(path: &Path) -> Option<Uri> {
    let path = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::from(if path.starts_with('/') { "file://" } else { "file:///" });
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri.parse().ok()
}
