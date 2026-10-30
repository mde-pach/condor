use std::path::Path;
use std::sync::Arc;

use crate::paths::format_path;

/// The kinds of findings, in the order vulture concatenates them when reporting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemKind {
    Attribute,
    Class,
    Function,
    Import,
    Method,
    Property,
    Variable,
    UnreachableCode,
    /// A `--make-whitelist` entry whose definition no longer exists.
    StaleWhitelistEntry,
}

impl ItemKind {
    pub const ALL: [ItemKind; 9] = [
        ItemKind::Attribute,
        ItemKind::Class,
        ItemKind::Function,
        ItemKind::Import,
        ItemKind::Method,
        ItemKind::Property,
        ItemKind::Variable,
        ItemKind::UnreachableCode,
        ItemKind::StaleWhitelistEntry,
    ];

    /// The `typ` string vulture uses in messages and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            ItemKind::Attribute => "attribute",
            ItemKind::Class => "class",
            ItemKind::Function => "function",
            ItemKind::Import => "import",
            ItemKind::Method => "method",
            ItemKind::Property => "property",
            ItemKind::Variable => "variable",
            ItemKind::UnreachableCode => "unreachable_code",
            ItemKind::StaleWhitelistEntry => "whitelist_entry",
        }
    }

    /// Parse a kind name (as in `--only`) or its noqa code.
    pub fn parse(s: &str) -> Option<ItemKind> {
        let s = s.trim().to_ascii_lowercase();
        ItemKind::ALL.into_iter().find(|k| {
            k.as_str() == s || k.noqa_code().eq_ignore_ascii_case(&s) || (s == "unreachable" && *k == ItemKind::UnreachableCode)
        })
    }

    /// The `# noqa` code for this kind.
    pub fn noqa_code(self) -> &'static str {
        match self {
            ItemKind::Attribute => "V101",
            ItemKind::Class => "V102",
            ItemKind::Function => "V103",
            ItemKind::Import => "V104",
            ItemKind::Method => "V105",
            ItemKind::Property => "V106",
            ItemKind::Variable => "V107",
            ItemKind::UnreachableCode => "V201",
            ItemKind::StaleWhitelistEntry => "V301",
        }
    }
}

/// A definition (or unreachable code chunk) found in one file.
#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    pub file: Arc<Path>,
    pub first_line: u32,
    pub last_line: u32,
    /// Custom message; `None` means `unused <typ> '<name>'`.
    pub message: Option<String>,
    pub confidence: u8,
    /// Language-defined role of the item for project-wide rules (0: none).
    pub tag: u8,
    /// Language-defined context for `tag`, such as the enclosing class.
    pub context: Option<String>,
}

impl Item {
    pub fn size(&self) -> u32 {
        self.last_line - self.first_line + 1
    }

    pub fn message(&self) -> String {
        match &self.message {
            Some(m) => m.clone(),
            None => format!("unused {} '{}'", self.kind.as_str(), self.name),
        }
    }

    /// The identity vulture uses to deduplicate items: `(filename, first_lineno, name)`.
    pub fn key(&self) -> (&Path, u32, &str) {
        (&self.file, self.first_line, &self.name)
    }

    pub fn report(&self, cwd: &Path, add_size: bool) -> String {
        let size = if add_size {
            let n = self.size();
            format!(", {n} {}", if n == 1 { "line" } else { "lines" })
        } else {
            String::new()
        };
        format!(
            "{}:{}: {} ({}% confidence{size})",
            format_path(&self.file, cwd).display(),
            self.first_line,
            self.message(),
            self.confidence
        )
    }

    pub fn whitelist_string(&self, cwd: &Path) -> String {
        let filename = format_path(&self.file, cwd);
        if matches!(self.kind, ItemKind::UnreachableCode | ItemKind::StaleWhitelistEntry) {
            return format!("# {} ({}:{})", self.message(), filename.display(), self.first_line);
        }
        let prefix = match self.kind {
            ItemKind::Attribute | ItemKind::Method | ItemKind::Property => "_.",
            _ => "",
        };
        format!(
            "{prefix}{}  # unused {} ({}:{})",
            self.name,
            self.kind.as_str(),
            filename.display(),
            self.first_line
        )
    }
}
