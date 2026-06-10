//! soaim binary — a thin shim over [`soaim::run`].

fn main() -> anyhow::Result<()> {
    soaim::run()
}
