macro_rules! wrap {
    ($variable: ident, $variable_c: ident, $value: expr) => {
        let $variable = ::std::sync::Arc::new(::std::sync::Mutex::new($value));
        let $variable_c = $variable.clone();
    };
}

macro_rules! unwrap {
    ($first_variable: ident $(, $variable: ident)*) => {
        let $first_variable = *$first_variable.lock().unwrap();
        $(let $variable = *$variable.lock().unwrap();)*
    };
}

pub(super) use wrap;
pub(super) use unwrap;
