use crate::database::Database;
use crate::package::Package;

/// The results of selecting a set of packages.
pub enum SelectResults<'a> {
    /// All of the packages selected were found.
    Results(Vec<&'a Package>),
    /// Of the selected packages, these were not found.
    NotFound(Vec<String>),
}

/// Turns a set of package names into their respective package objects.
pub fn packages_from_names<'a, I>(
    package_names: &[&str],
    database: &'a Database,
) -> SelectResults<'a>
where
    I: Iterator<Item = Package>,
{
    let mut packages_found = Vec::new();
    let mut packages_not_found = Vec::new();
    for package_name in package_names {
        let package = database.get_package(package_name);
        if let Some(package) = package {
            packages_found.push(package)
        } else {
            packages_not_found.push(package_name);
        }
    }
    if packages_not_found.is_empty() {
        SelectResults::NotFound(
            packages_not_found
                .iter()
                .map(|s| String::from(**s))
                .collect(),
        )
    } else {
        SelectResults::Results(packages_found)
    }
}
