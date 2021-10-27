fn main() {
    // get perl's MULTIPLICITY flag:
    //     perl -MConfig -e 'print $Config{usemultiplicity}'
    let perl_multiplicity = std::process::Command::new("perl")
        .arg("-MConfig")
        .arg("-e")
        .arg("print $Config{usemultiplicity}")
        .output()
        .expect("failed to get perl usemultiplicity flag");

    // pass the multiplicity cfg flag:
    if perl_multiplicity.stdout == b"define" {
        println!("cargo:rustc-cfg=perlmod=\"multiplicity\"");
    }
}
