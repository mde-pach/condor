//! Given some Python code, which dead code does condor report?

use std::path::Path;
use std::sync::Arc;

use condor_core::{Analysis, ScanOptions};

/// Scan `files` (name, source) together; return `(kind, name, line, confidence)`
/// for each reported item, in report order.
fn findings(files: &[(&str, &str)]) -> Vec<(String, String, u32, u8)> {
    let python = condor_python::Python::bundled();
    let opts = ScanOptions {
        cwd: "/project".into(),
        call_hints: condor_python::plugins::call_hints(&python.plugins),
        ..ScanOptions::default()
    };
    let mut analysis = Analysis::default();
    for (name, source) in files {
        let path: Arc<Path> = Arc::from(Path::new("/project").join(name).as_path());
        analysis.push(condor_python::scanner::scan(source, path, &opts));
    }
    condor_python::plugins::apply(&mut analysis, &python.plugins);
    analysis.check_whitelist_entries(Path::new("/project"));
    analysis
        .unused_code(0, false)
        .into_iter()
        .map(|i| (i.kind.as_str().to_string(), i.name.clone(), i.first_line, i.confidence))
        .collect()
}

fn names(files: &[(&str, &str)]) -> Vec<String> {
    findings(files).into_iter().map(|(_, name, _, _)| name).collect()
}

#[test]
fn unused_helper_is_reported() {
    let code = "def useful_load():\n    return 1\n\ndef main():\n    return 2\n\nmain()\n";
    assert_eq!(findings(&[("app.py", code)]), vec![("function".into(), "useful_load".into(), 1, 60)]);
}

#[test]
fn use_in_another_file_counts() {
    let files = [("a.py", "def helper():\n    pass\n"), ("b.py", "from a import helper\nhelper()\n")];
    assert_eq!(names(&files), Vec::<String>::new());
}

#[test]
fn confidences() {
    let code = "import os\n\ndef f(arg):\n    return 1\n    print('dead')\n\nf(1)\n";
    assert_eq!(
        findings(&[("m.py", code)]),
        vec![
            ("import".into(), "os".into(), 1, 90),
            ("variable".into(), "arg".into(), 3, 100),
            ("unreachable_code".into(), "return".into(), 5, 100),
        ]
    );
}

#[test]
fn names_in_type_comments_are_used() {
    let code = "from typing import Dict, List\nx = {}  # type: Dict[str, int]\nprint(x)\n";
    assert_eq!(names(&[("m.py", code)]), vec!["List".to_string()]);
}

#[test]
fn format_strings_with_locals_use_names() {
    let code = "def f():\n    a = 1\n    b = 2\n    return '{a}'.format(**locals()) + '%(b)s' % locals()\nf()\n";
    assert_eq!(names(&[("m.py", code)]), Vec::<String>::new());
}

#[test]
fn all_getattr_and_hasattr_use_names() {
    let code = "__all__ = ['exported']\ndef exported(): pass\ndef by_getattr(): pass\ndef by_hasattr(): pass\n\
                getattr(x, 'by_getattr')\nhasattr(x, 'by_hasattr')\n";
    assert_eq!(names(&[("m.py", code)]), Vec::<String>::new());
}

#[test]
fn noqa_suppresses_findings() {
    let code = "import os  # noqa: F401\ndef f():  # noqa: V103\n    pass\ndef g():  # noqa\n    pass\ndef h():  # noqa: V101\n    pass\n";
    assert_eq!(names(&[("m.py", code)]), vec!["h".to_string()]);
}

#[test]
fn constant_conditions() {
    let code = "def f(x):\n    if False:\n        x()\n    while 0:\n        x()\n    return 1 if True else 2\nf(1)\n";
    let messages: Vec<String> = findings(&[("m.py", code)]).into_iter().map(|(_, n, l, _)| format!("{n}:{l}")).collect();
    assert_eq!(messages, vec!["if:2", "while:4", "ternary:6"]);
}

#[test]
fn ignored_names() {
    let code = "_private = 1\n__dunder__ = 2\nclass C:\n    def __init__(self):\n        pass\n";
    assert_eq!(names(&[("m.py", code)]), vec!["C".to_string()]);
}

#[test]
fn syntax_error_is_invalid_input() {
    let opts = ScanOptions::default();
    let scan = condor_python::scanner::scan("def f(:\n", Arc::from(Path::new("bad.py")), &opts);
    assert!(scan.invalid_input);
    assert!(scan.items.is_empty());
}

// ----- condor's rules beyond vulture (docs/improvements.md) -----------------

#[test]
fn i1_interface_parameters_are_not_reported() {
    let code = "\
from typing import Protocol, overload, override
from abc import abstractmethod

class Logger(Protocol):
    def log(self, level, msg): ...

class Base:
    @abstractmethod
    def run(self, job): pass

    def hook(self, event):
        raise NotImplementedError

    @overload
    def get(self, key: int) -> int: ...

class Child(Base):
    @override
    def run(self, job):
        return 1

def placeholder(arg):
    pass

for o in (Logger, Child): o.log, o.run, o.hook, o.get
";
    assert_eq!(names(&[("m.py", code)]), vec!["placeholder".to_string(), "arg".to_string()]);
}

#[test]
fn i2_declarative_class_fields_are_used() {
    let code = "\
from dataclasses import dataclass
from enum import Enum
from typing import TypedDict

@dataclass
class User:
    name: str
    CONSTANT = 1

class Admin(User):
    level: int

class Movie(TypedDict):
    title: str

class Color(Enum):
    RED = 1

class Plain:
    field: int

print(User, Admin, Movie, Color, Plain)
";
    assert_eq!(names(&[("m.py", code)]), vec!["CONSTANT".to_string(), "field".to_string()]);
}

#[test]
fn i2_subclasses_across_files() {
    let files = [
        ("models.py", "from pydantic import BaseModel\nclass Base(BaseModel):\n    id: int\n"),
        ("user.py", "from models import Base\nclass User(Base):\n    email: str\nprint(User)\n"),
    ];
    assert_eq!(names(&files), Vec::<String>::new());
}

#[test]
fn i3_pytest_fixtures() {
    let files = [
        (
            "tests/conftest.py",
            "import pytest\n\n@pytest.fixture\ndef db():\n    return 1\n\n@pytest.fixture(name='client')\ndef make_client(db):\n    return db\n\n\
             @pytest.fixture(autouse=True)\ndef reset():\n    pass\n\n@pytest.fixture\ndef unused_fixture():\n    pass\n",
        ),
        ("tests/test_app.py", "def test_x(client, tmp_path):\n    pass\n"),
    ];
    assert_eq!(names(&files), vec!["unused_fixture".to_string()]);
}

#[test]
fn i5_generator_stub_is_not_unreachable() {
    let code = "class A:\n    def items(self):\n        raise NotImplementedError\n        yield\nA().items\n";
    assert_eq!(names(&[("m.py", code)]), Vec::<String>::new());
}

#[test]
fn i6_self_references_do_not_count() {
    let code = "\
def walk(node):
    return [walk(c) for c in node]

class Tree:
    def visit(self):
        self.visit()
    @classmethod
    def build(cls):
        return Tree()
    def save(self):
        super().save()
    def close(self):
        self._f.close()

def data():
    data = 1
    return data

def resolve_user(info):
    return info

class Query:
    def resolve_user(self, info):
        return resolve_user(info)

Query().resolve_user
";
    // `save` calls the base class and `close` delegates, so both count as
    // used; `data` binds its own name, so its reference is to the variable;
    // inside method `resolve_user`, the bare name is the module function.
    assert_eq!(
        names(&[("m.py", code)]),
        vec!["walk".to_string(), "Tree".to_string(), "visit".to_string(), "build".to_string(), "cls".to_string()]
    );
}

#[test]
fn i7_stale_whitelist_entries() {
    let files = [
        ("pkg/a.py", "def renamed():\n    pass\nrenamed()\n"),
        ("pkg/b.py", "def zeta():\n    pass\n"),
        ("whitelist.py", "zeta  # unused function (pkg/a.py:1)\nrenamed  # unused function (pkg/a.py:1)\n"),
    ];
    let found = findings(&files);
    assert!(found.contains(&("function".into(), "zeta".into(), 1, 60)), "{found:?}");
    assert!(found.contains(&("whitelist_entry".into(), "zeta".into(), 1, 100)), "{found:?}");
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn bundled_plugins_are_valid() {
    let names: Vec<String> = condor_python::Python::bundled().plugins.into_iter().map(|p| p.plugin.name).collect();
    for expected in ["abc", "dataclasses", "enum", "typing", "pytest", "pydantic", "attrs", "django", "celery", "flask", "fastapi", "click"] {
        assert!(names.iter().any(|n| n == expected), "{expected} missing from {names:?}");
    }
}

#[test]
fn plugins_resolve_imports() {
    // `pd.BaseModel` resolves to `pydantic.BaseModel`; a local `BaseModel`
    // in a project that does not import pydantic is not a model.
    let files = [
        ("a.py", "import pydantic as pd\nclass A(pd.BaseModel):\n    x: int\nprint(A)\n"),
        ("b.py", "class BaseModel:\n    pass\nclass B(BaseModel):\n    y: int\nprint(B)\n"),
    ];
    assert_eq!(names(&files), vec!["y".to_string()]);
}

#[test]
fn framework_entry_points() {
    let files = [
        ("api.py", "from fastapi import FastAPI\napp = FastAPI()\n\n@app.get('/')\ndef index():\n    pass\n\ndef helper():\n    pass\n"),
        ("cli.py", "import click\n\n@click.group()\ndef cli():\n    pass\n\n@cli.command()\ndef run():\n    pass\n"),
    ];
    assert_eq!(names(&files), vec!["helper".to_string()]);
}
