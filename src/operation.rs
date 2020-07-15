use crate::error::MixError;

/// An action that can be performed by the package database.
#[derive(Debug, PartialEq)]
pub enum Operation {
    /// Install packages.
    Install(Vec<String>),
    /// Uninstall packages.
    Remove(Vec<String>),
    /// Bring the cache up to date.
    Synchronize,
    /// Bring packages up to date.
    Update(Option<Vec<String>>),
    /// Download files for a package.
    Fetch(Vec<String>),
    /// List the installed packages.
    List,
}

/// Implements behaviors corresponding to an `Operation`.
/// # Todo:
/// Remove.
pub trait Actionable {
    /// Install the given packages to the system
    fn install(&mut self, packages: &[String]) -> Result<(), MixError>;
    /// Remove the given packages from the system
    fn remove(&mut self, packages: &[String]) -> Result<(), MixError>;
    /// Bring the local package cache in sync with the remote cache
    fn synchronize(&mut self) -> Result<(), MixError>;
    /// Bring the given packages to the newest version, defaulting to every installed package
    fn update(&mut self, packages: &Option<Vec<String>>) -> Result<(), MixError>;
    /// Get the files of the given packages
    fn fetch(&self, packages: &[String]) -> Result<(), MixError>;
    /// List the packages currently installed
    fn list(&self) -> Result<(), MixError>;
}

impl Operation {
    /// Create a new `Operation` from the command provided.
    /// # Panics:
    /// This will `panic!()` when there is an unrecognized subcommand or unexpected presence of packages.
    /// This should never happen, because `mix::arguments::parse_arguments` should be able to error on invalid subcommands.
    pub fn new(subcommand: &str, packages: &Option<Vec<String>>) -> Self {
        if packages.is_none() {
            match subcommand {
                "synchronize" => return Self::Synchronize,
                "update" => return Self::Update(None),
                "list" => return Self::List,
                _ => (),
            }
        };
        let packages = packages.clone().unwrap();
        match subcommand {
            "install" => Self::Install(packages),
            "remove" => Self::Remove(packages),
            "update" => Self::Update(Some(packages)),
            "fetch" => Self::Fetch(packages),
            _ => unimplemented!("The subcommand {} is not known.", subcommand),
        }
    }

    /// Calls the corresponding method on the given `Actionable`.
    pub fn execute<T: Actionable>(&self, executor: &mut T) -> Result<(), MixError> {
        match self {
            Operation::Install(packages) => executor.install(packages),
            Operation::Remove(packages) => executor.remove(packages),
            Operation::Synchronize => executor.synchronize(),
            Operation::Update(packages) => executor.update(packages),
            Operation::Fetch(packages) => executor.fetch(packages),
            Operation::List => executor.list(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! is_enum_variant {
        ($v:expr, $p:pat) => {
            if let $p = $v {
                true
            } else {
                false
            }
        };
    }

    #[test]
    fn install_subcommand_creates_install() {
        let action = Operation::new("install", &Some(vec![]));
        assert!(is_enum_variant!(action, Operation::Install {..}));
    }

    #[test]
    fn remove_subcommand_creates_remove() {
        let action = Operation::new("remove", &Some(vec![]));
        assert!(is_enum_variant!(action, Operation::Remove {..}));
    }

    #[test]
    fn synchronize_subcommand_creates_synchronize() {
        let action = Operation::new("synchronize", &None);
        assert!(is_enum_variant!(action, Operation::Synchronize {..}));
    }

    #[test]
    fn update_subcommand_creates_update() {
        let action = Operation::new("update", &None);
        assert!(is_enum_variant!(action, Operation::Update {..}));
    }

    #[test]
    fn fetch_subcommand_creates_fetch() {
        let action = Operation::new("fetch", &Some(vec![]));
        assert!(is_enum_variant!(action, Operation::Fetch {..}));
    }

    #[test]
    fn list_subcommand_creates_list() {
        let action = Operation::new("list", &None);
        assert!(is_enum_variant!(action, Operation::List {..}));
    }
}
