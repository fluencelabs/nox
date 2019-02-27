use clap::{value_t, App, Arg, ArgMatches, SubCommand};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub fn interactive_setup() -> () {
    let mut rl = Editor::<()>::new();
    println!("write your contract address...");
    let line1 = rl.readline(">").unwrap();
    println!("your contract address is: {}", line1);

}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("setup")
        .about("Setup Fluence CLI with common parameters.")

}