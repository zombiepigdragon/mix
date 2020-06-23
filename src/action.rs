use std::error::Error;

/// An action that can be performed by the package database.
#[derive(Debug, PartialEq)]
pub enum Action {
    /// Install packages.
    Install(Vec<String>),
    /// Uninstall packages.
    Remove(Vec<String>),
    /// Bring the cache up to date.
    Synchronize(Option<Box<Action>>),
    /// Bring packages up to date.
    Update(Option<Vec<String>>),
    /// Download files for a package.
    Fetch(Vec<String>),
    /// List the installed packages.
    List,
}

/// Implements behaviors corresponding to an `Action`.
pub trait Actionable {
    /// Install the given packages to the system
    fn install(&mut self, packages: &[String]) -> Result<(), Box<dyn Error>>;
    /// Remove the given packages from the system
    fn remove(&mut self, packages: &[String]) -> Result<(), Box<dyn Error>>;
    /// Bring the local package cache in sync with the remote cache, then run `next_action`
    fn synchronize(&mut self, next_action: &Option<Box<Action>>) -> Result<(), Box<dyn Error>>;
    /// Bring the given packages to the newest version, defaulting to every installed package
    fn update(&mut self, packages: &Option<Vec<String>>) -> Result<(), Box<dyn Error>>;
    /// Get the files of the given packages
    fn fetch(&self, packages: &[String]) -> Result<(), Box<dyn Error>>;
    /// List the packages currently installed
    fn list(&self) -> Result<(), Box<dyn Error>>;
}

impl Action {
    /// Create a new `Action` from the command provided.
    /// # Panics:
    /// This will `panic!()` when there is an unrecognized subcommand or unexpected presence of packages.
    /// This should never happen, because `mix::arguments::parse_arguments` should be able to error on invalid subcommands.
    pub fn new(subcommand: &str, packages: &Option<Vec<String>>) -> Self {
        if packages.is_none() {
            match subcommand {
                "synchronize" => return Self::Synchronize(None),
                "update" => return Self::Synchronize(Some(Box::new(Self::Update(None)))),
                "list" => return Self::List,
                _ => (),
            }
        };
        let packages = packages.clone().unwrap();
        match subcommand {
            "install" => Self::Install(packages),
            "remove" => Self::Remove(packages),
            "update" => Self::Synchronize(Some(Box::new(Self::Update(Some(packages))))),
            "fetch" => Self::Fetch(packages),
            _ => unimplemented!("The subcommand {} is not known.", subcommand),
        }
    }

    /// Calls the corresponding method on the given `Actionable`.
    pub fn execute<T: Actionable>(&self, executor: &mut T) -> Result<(), Box<dyn Error>> {
        match self {
            Action::Install(packages) => executor.install(packages),
            Action::Remove(packages) => executor.remove(packages),
            Action::Synchronize(next_action) => executor.synchronize(next_action),
            Action::Update(packages) => executor.update(packages),
            Action::Fetch(packages) => executor.fetch(packages),
            Action::List => executor.list(),
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
        let action = Action::new("install", &Some(vec![]));
        assert!(is_enum_variant!(action, Action::Install {..}));
    }

    #[test]
    fn remove_subcommand_creates_remove() {
        let action = Action::new("remove", &Some(vec![]));
        assert!(is_enum_variant!(action, Action::Remove {..}));
    }

    #[test]
    fn synchronize_subcommand_creates_synchronize() {
        let action = Action::new("synchronize", &None);
        assert!(is_enum_variant!(action, Action::Synchronize {..}));
    }

    #[test]
    fn update_subcommand_creates_synchronize_with_update() {
        let action = Action::new("update", &Some(vec![]));
        if let Action::Synchronize(next_action) = action {
            assert!(next_action.is_some());
            assert!(is_enum_variant!(*next_action.unwrap(), Action::Update {..}));
        }
        let action = Action::new("update", &None);
        if let Action::Synchronize(next_action) = action {
            assert!(next_action.is_some());
            assert!(is_enum_variant!(*next_action.unwrap(), Action::Update {..}));
        }
    }

    #[test]
    fn fetch_subcommand_creates_fetch() {
        let action = Action::new("fetch", &Some(vec![]));
        assert!(is_enum_variant!(action, Action::Fetch {..}));
    }

    #[test]
    fn list_subcommand_creates_list() {
        let action = Action::new("list", &None);
        assert!(is_enum_variant!(action, Action::List {..}));
    }
}
