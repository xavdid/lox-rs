use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreter::LoxValue;

/** represents a scope */
#[derive(Clone)]
pub struct Environment {
    // Rc means I can have shared ownership of the object, useful when copying environments between scopes
    // RefCell provies "interior mutability", meaning `env` doesn't have to be mutably borrowed everywhere and .
    //   ownership checks are deferred to runtime instead of compile time
    //   in exchange, multiple owners can all try to mutate the data (as long as they follow the normal rules; it's a panic if there are two mutable borrows at once)
    values: Rc<RefCell<HashMap<String, LoxValue>>>,
    enclosing: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: Rc::new(RefCell::new(HashMap::new())),
            enclosing: None,
        }
    }

    pub fn child_scope(&self) -> Self {
        Environment {
            values: Rc::new(RefCell::new(HashMap::new())),
            // because the underlying `values` is an `Rc<>`, a `.clone()` doesn't actually copy data, just increments the reference count
            enclosing: Some(self.clone().into()),
        }
    }

    pub fn get(&self, name: &str) -> Option<LoxValue> {
        self.values
            .borrow()
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
        self.values.borrow_mut().insert(name.into(), value);
    }

    // updates an existing variable, but can't create
    // var a; a = 3; // ok
    // b = 3; // err, `b` is not defined
    /** Returns `Some` if the write was successful and `None` otherwise. */
    pub fn assign(&self, name: &str, value: LoxValue) -> Option<LoxValue> {
        if self.values.borrow().contains_key(name) {
            self.values.borrow_mut().insert(name.into(), value.clone());
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

    #[test]
    fn it_looks_for_vars_in_parent_envs() {
        let root_env = Environment::new();
        let child_env = root_env.child_scope();
        root_env.define("name", LoxValue::LString("david".to_string()));

        assert_eq!(
            child_env.get("name"),
            Some(LoxValue::LString("david".to_string()))
        );
    }

    #[test]
    fn it_can_assign_in_parent_envs() {
        let root_env = Environment::new();
        let child_env = root_env.child_scope();
        root_env.define("name", LoxValue::Nil);

        assert_eq!(
            child_env.assign("name", LoxValue::LString("david".to_string())),
            Some(LoxValue::LString("david".to_string()))
        );
    }

    #[test]
    fn it_errors_if_var_never_defined_during_assign() {
        let root_env = Environment::new();
        let child_env = root_env.child_scope();

        assert_eq!(
            child_env.assign("name", LoxValue::LString("david".to_string())),
            None
        );
    }
}
