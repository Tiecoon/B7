extern crate cc;

fn main() {
    cc::Build::new().file("src/b77.c").compile("b77.a");
}
