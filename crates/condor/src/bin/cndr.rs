//! `cndr`: the same program as `condor`, named after the PyPI package so
//! that `uvx cndr` runs it.

fn main() {
    std::process::exit(condor::run::main());
}
