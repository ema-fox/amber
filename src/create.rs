use crate::val::{Val};

pub fn lit(v: Val) -> Val {
    Val::from(im::HashMap::from(vec![
        (Val::from("op"), "lit".into()),
        ("val".into(), v)
    ]))
}

pub fn deref<T>(name: T) -> Val where T: Into<Val>{
    Val::from(im::HashMap::from(vec![
        (Val::from("op"), Val::from("deref")),
        ("name".into(), name.into())
    ]))
}

pub fn inst(op: &str, args: Vec<Val>) -> Val {
    Val::from(im::HashMap::from(vec![
        (Val::from("op"), Val::from(op)),
        ("args".into(), args.into())
    ]))
}
