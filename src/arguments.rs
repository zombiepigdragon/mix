use crate::action::Action;
use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version};
use clap::{AppSettings, Arg, ArgMatches, SubCommand};

/// Convert the matched arguments to a set of package names. Requires packages be stored in a value named "target".
fn arguments_to_package_names(values: Option<&ArgMatches>) -> Option<Vec<String>> {
    match values {
        Some(values) => {
            if let Some(packages) = values.values_of("target") {
                Some(packages.map(String::from).collect())
            } else {
                None
            }
        }
        None => None,
    }
}

/// Takes the command line arguments for the program and returns the `Action` to execute.
/// ```rust
/// # use mix::arguments::parse_arguments;
/// parse_arguments(std::env::args_os()); // Use the provided command line
/// parse_arguments(vec!["mix", "install", "foo"]); // or make one up
/// ```
pub fn parse_arguments<I, T>(arguments: I) -> clap::Result<Action>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let app = app_from_crate!()
        .subcommand(
            SubCommand::with_name("install")
                .about("Installs a package")
                .arg(
                    Arg::with_name("target")
                        .help("The package(s) to install")
                        .min_values(1)
                        .required(true)
                        .index(1),
                )
                .setting(AppSettings::ArgRequiredElseHelp)
                .visible_alias("in"),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Removes a package")
                .arg(
                    Arg::with_name("target")
                        .help("The package(s) to remove")
                        .min_values(1)
                        .required(true)
                        .index(1),
                )
                .setting(AppSettings::ArgRequiredElseHelp)
                .visible_alias("re"),
        )
        .subcommand(
            SubCommand::with_name("synchronize")
                .about("Synchronizes the package database")
                .visible_alias("sy"),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Updates a package")
                .arg(
                    Arg::with_name("target")
                        .help("The packages to update")
                        .min_values(1)
                        .index(1),
                )
                .visible_alias("up"),
        )
        .subcommand(
            SubCommand::with_name("fetch")
                .about("Downloads a package without installing it")
                .arg(
                    Arg::with_name("target")
                        .help("The package(s) to fetch")
                        .min_values(1)
                        .required(true)
                        .index(1),
                )
                .setting(AppSettings::ArgRequiredElseHelp)
                .visible_alias("fe"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("Lists the installed packages")
                .visible_alias("li"),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp);

    let matches = app.get_matches_from_safe(arguments)?;

    let (subcommand_name, subcommand_arguments) = matches.subcommand();
    Ok(Action::new(
        subcommand_name,
        &arguments_to_package_names(subcommand_arguments),
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use std::error::Error;

    #[test]
    fn no_arguments_help_displayed() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix"]);
        assert!(result.is_err());
        let no_arg_message = result.unwrap_err().to_string();
        let help_message = parse_arguments(vec!["mix", "--help"])
            .unwrap_err()
            .to_string();
        assert_eq!(no_arg_message, help_message);
        Ok(())
    }

    #[test]
    fn help_requested_help_displayed() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "--help"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, clap::ErrorKind::HelpDisplayed);
        let result = parse_arguments(vec!["mix", "-h"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, clap::ErrorKind::HelpDisplayed);
        Ok(())
    }

    #[test]
    fn version_requested_version_displayed() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "--version"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, clap::ErrorKind::VersionDisplayed);
        let result = parse_arguments(vec!["mix", "-V"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, clap::ErrorKind::VersionDisplayed);
        Ok(())
    }

    #[test]
    fn package_install_returns_install() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "install", "test_package"])?;
        assert_eq!(result, Action::Install(vec!["test_package".to_string()]));
        let result = parse_arguments(vec!["mix", "install", "test_package_1", "test_package_2"])?;
        assert_eq!(
            result,
            Action::Install(vec![
                "test_package_1".to_string(),
                "test_package_2".to_string()
            ])
        );
        Ok(())
    }

    #[test]
    fn package_remove_returns_remove() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "remove", "test_package"])?;
        assert_eq!(result, Action::Remove(vec!["test_package".to_string()]));
        let result = parse_arguments(vec!["mix", "remove", "test_package_1", "test_package_2"])?;
        assert_eq!(
            result,
            Action::Remove(vec![
                "test_package_1".to_string(),
                "test_package_2".to_string()
            ])
        );
        Ok(())
    }

    #[test]
    fn package_synchronize_returns_synchronize() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "synchronize"])?;
        assert_eq!(result, Action::Synchronize(None));
        Ok(())
    }

    #[test]
    fn package_update_returns_update() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "update"])?;
        assert_eq!(
            result,
            Action::Synchronize(Some(Box::new(Action::Update(None))))
        );
        let result = parse_arguments(vec!["mix", "update", "test_package"])?;
        assert_eq!(
            result,
            Action::Synchronize(Some(Box::new(Action::Update(Some(vec![
                "test_package".to_string()
            ])))))
        );
        let result = parse_arguments(vec!["mix", "update", "test_package_1", "test_package_2"])?;
        assert_eq!(
            result,
            Action::Synchronize(Some(Box::new(Action::Update(Some(vec![
                "test_package_1".to_string(),
                "test_package_2".to_string()
            ])))))
        );
        Ok(())
    }

    #[test]
    fn package_list_returns_list() -> Result<(), Box<dyn Error>> {
        let result = parse_arguments(vec!["mix", "list"])?;
        assert_eq!(result, Action::List);
        Ok(())
    }
}
