use clap::ErrorKind;
use mix::arguments::parse_arguments;
use mix::package::{PackageDatabase, PackageList};
use std::env;
use std::error::Error;
use std::process;

fn main() -> Result<(), Box<dyn Error>> {
    let action = parse_arguments(env::args_os());
    let action = match action {
        Ok(action) => action,
        Err(error) => {
            eprintln!("{}", error);
            let result = match error.kind {
                ErrorKind::HelpDisplayed => 0,
                ErrorKind::VersionDisplayed => 0,
                _ => 1,
            };
            process::exit(result);
        }
    };
    let mut package_list = PackageList::load(".mix.db".into())?;
    action.execute(&mut package_list)?;
    if package_list.needs_save() {
        package_list.save()?;
    }
    Ok(())
}
