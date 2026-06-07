use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::interpreter::LoxValue;

/// represents a scope
#[derive(Clone, Debug)]
pub struct Environment {
    // Rc means I can have shared ownership of the object, useful when copying environments between scopes
    // RefCell provies "interior mutability", meaning `env` doesn't have to be mutably borrowed everywhere and ownership checks are deferred to runtime instead of compile time.
    // In exchange, multiple owners can all try to mutate the data
    // (as long as they follow the normal rules; it's a panic if there are two mutable borrows at once)

    // this used to be:
    // values: Rc<RefCell<HashMap<String, LoxValue>>>,
    // but anyhow requires its errors to be sendable, and one of the error tyeps is actually a return, which holds a function definition which holds a closure (an env), whose Rc isn't sendable. So we'll use the threadsafe version (`Arc<Mutex<...>>`) which is a little slower, but threadsafe.
    values: Arc<Mutex<HashMap<String, LoxValue>>>,
    enclosing: Option<Box<Environment>>,
}

impl PartialEq for Environment {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: Arc::new(Mutex::new(HashMap::new())),
            enclosing: None,
        }
    }

    pub fn child_scope(&self) -> Self {
        Environment {
            values: Arc::new(Mutex::new(HashMap::new())),
            // because the underlying `values` is an `Rc<>`, a `.clone()` doesn't actually copy data, just increments the reference count
            enclosing: Some(self.clone().into()),
        }
    }

    pub fn get(&self, name: &str) -> Option<LoxValue> {
        self.values
            .lock()
            .unwrap()
            .get(name)
            .cloned() // TODO: this means we clone on every variable read, which isn't great!
            .or_else(|| {
                if let Some(parent) = &self.enclosing {
                    parent.get(name)
                } else {
                    None
                }
            })
    }

    // set for the first time
    // var a = 3;
    pub fn define(&self, name: &str, value: LoxValue) {
        self.values.lock().unwrap().insert(name.into(), value);
    }

    // TODO: make this print nicely, but not be a real doctest
    /** updates an existing variable, but can't create.
     * var a; a = 3; // ok
     * b = 3; // err, `b` is not defined
     * Returns `Some` if the write was successful and `None` otherwise.
     */
    pub fn assign(&self, name: &str, value: LoxValue) -> Option<LoxValue> {
        if self.values.lock().unwrap().contains_key(name) {
            self.values
                .lock()
                .unwrap()
                .insert(name.into(), value.clone());
            Some(value)
        } else if let Some(parent) = &self.enclosing {
            parent.assign(name, value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_looks_for_vars_in_parent_envs() {
        let root_env = Environment::new();
        let child_env = root_env.child_scope();
        root_env.define("name", LoxValue::String("david".to_string()));

        assert_eq!(
            child_env.get("name"),
            Some(LoxValue::String("david".to_string()))
        );
    }

    #[test]
    fn it_can_assign_in_parent_envs() {
        let root_env = Environment::new();
        let child_env = root_env.child_scope();
        root_env.define("name", LoxValue::Nil);

        assert_eq!(
            child_env.assign("name", LoxValue::String("david".to_string())),
            Some(LoxValue::String("david".to_string()))
        );
    }

    #[test]
    fn it_errors_if_var_never_defined_during_assign() {
        let root_env = Environment::new();
        let child_env = root_env.child_scope();

        assert_eq!(
            child_env.assign("name", LoxValue::String("david".to_string())),
            None
        );
    }
}
