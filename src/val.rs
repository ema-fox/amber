use std::rc::Rc;

#[derive(Clone)]
pub struct AFn(pub Rc<dyn Fn (Val) -> Result<Val, Val>>);

impl std::fmt::Debug for AFn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("fn")
    }
}

impl std::cmp::PartialEq for AFn {
    fn eq(&self, _other: &Self) -> bool {
        panic!()
    }
}


impl std::cmp::Eq for AFn {}

impl std::hash::Hash for AFn {
    fn hash<H>(&self, _: &mut H) {
        panic!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Val {
    Int(i64),
    Str(String),
    List(Vec<Val>),
    Dict(im::HashMap<Val, Val>),
    Fn(AFn)
}


impl Val {
    pub fn get<K>(&self, k: K) -> &Val
    where
        K: Into<Val>
    {
        if let Val::Dict(d) = self {
            d.get(&k.into()).unwrap()
        } else {
            panic!();
        }
    }
}

impl TryFrom<&Val> for String {
    type Error = &'static str;

    fn try_from(v: &Val) -> Result<Self, Self::Error> {
        if let Val::Str(s) = v {
            Ok(s.clone())
        } else {
            Err("Not a Val::Str")
        }
    }
}

impl TryFrom<Val> for String {
    type Error = &'static str;

    fn try_from(v: Val) -> Result<Self, Self::Error> {
        if let Val::Str(s) = v {
            Ok(s)
        } else {
            Err("Not a Val::Str")
        }
    }
}

impl From<&str> for Val {
    fn from(s: &str) -> Self {
        Val::Str(s.to_string())
    }
}

impl From<Vec<Val>> for Val {
    fn from(xs: Vec<Val>) -> Self {
        Val::List(xs)
    }
}
