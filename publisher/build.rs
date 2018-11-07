use std::process::Command;

fn main() {
    println!("Installing npm...");

    let mut install_cmd = Command::new("npm");
    install_cmd.current_dir("../bootstrap/");
    install_cmd.args(&["install"]);
    install_cmd.status().unwrap();

    println!("Compiling sol files...");

    let mut gen_cmd = Command::new("npm");
    gen_cmd.current_dir("../bootstrap/");
    gen_cmd.args(&["run", "compile-sol"]);
    gen_cmd.status().unwrap();
}
