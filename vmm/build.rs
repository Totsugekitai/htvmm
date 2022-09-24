fn main() {
    cc::Build::new()
        .warnings(true)
        .file("src/entry.s")
        .compile("entry.o");
}
