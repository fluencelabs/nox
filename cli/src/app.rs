use clap::App;
use publisher;

pub fn init<'a, 'b>() -> App<'a, 'b> {
    App::new("Fluence CLI")
        .version("0.1.0")
        .author("Fluence Labs")
        .about("Console utility for deploying code to fluence cluster")
        .subcommand(publisher::subcommand())
}