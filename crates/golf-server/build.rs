fn main() {
    // rust-embed snapshots web/dist inside a proc macro, which cargo can't
    // track on its own — without this, a rebuilt frontend is silently NOT
    // re-embedded and the binary keeps serving stale assets.
    println!("cargo:rerun-if-changed=../../web/dist");
}
