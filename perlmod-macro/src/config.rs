//! To facilitate moving towards a new convention of how to organize `perlmod` code, we add a way
//! to "configure" `perlmod`'s defaults via the environment.
//!
//! Currently the only option is:
//! - `PERLMOD_NON_PUB_EXPORTS=<deny|warn>`: Deny or warn about non-`pub` exports.

use std::error::Error as StdError;
use std::fmt;
use std::sync::LazyLock;

/// Invalid `Action` string.
#[derive(Debug)]
pub struct InvalidAction(String);

impl StdError for InvalidAction {}

impl fmt::Display for InvalidAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid action ({:?}), must be 'warn' or 'deny'", self.0)
    }
}

/// Whether we should warn about something or throw an error.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Action {
    #[default]
    Allow,
    Warn,
    Deny,
}

impl std::str::FromStr for Action {
    type Err = InvalidAction;

    fn from_str(s: &str) -> Result<Self, InvalidAction> {
        match s {
            "allow" => Ok(Self::Allow),
            "warn" => Ok(Self::Warn),
            "deny" => Ok(Self::Deny),
            other => Err(InvalidAction(other.to_string())),
        }
    }
}

fn get_action(var_name: &str) -> Option<Action> {
    match std::env::var(var_name) {
        Ok(action) => Some(action.parse().expect("failed to parse action")),
        Err(std::env::VarError::NotPresent) => None,
        Err(other) => panic!("failed to parse {var_name:?}: {other:#}"),
    }
}

static NON_PUB_EXPORTS: LazyLock<Action> =
    LazyLock::new(|| get_action("PERLMOD_NON_PUB_EXPORTS").unwrap_or_default());

pub fn non_pub_exports() -> Action {
    *NON_PUB_EXPORTS
}
