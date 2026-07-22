use std::rc::Rc;

use im::HashMap;

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
    Dict(HashMap<Val, Val>),
    Fn(AFn)
}


impl Val {
    pub fn get<K>(&self, k: K) -> Option<&Val>
    where
        K: Into<Val>
    {
        if let Val::Dict(d) = self {
            d.get(&k.into())
        } else {
            None
        }
    }

    pub fn insert<K, V>(&mut self, k: K, v: V)
    where
        K: Into<Val>,
        V: Into<Val>
    {
        if let Val::Dict(d) = self {
            d.insert(k.into(), v.into());
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

impl TryFrom<Val> for Vec<Val> {
    type Error = &'static str;

    fn try_from(v: Val) -> Result<Self, Self::Error> {
        if let Val::List(xs) = v {
            Ok(xs)
        } else {
            Err("Not a Val::List")
        }
    }
}

impl TryFrom<Val> for im::HashMap<Val, Val> {
    type Error = &'static str;

    fn try_from(v: Val) -> Result<Self, Self::Error> {
        if let Val::Dict(x) = v {
            Ok(x)
        } else {
            Err("Not a Val::Dict")
        }
    }
}

impl<T> TryFrom<Val> for Vec<T> where T: TryFrom<Val, Error = &'static str> {
    type Error = &'static str;

    fn try_from(v: Val) -> Result<Self, Self::Error> {
        if let Val::List(xs) = v {
            xs.into_iter().map(T::try_from).collect()
        } else {
            Err("Not a Val::List")
        }
    }
}

impl From<i64> for Val {
    fn from(x: i64) -> Self {
        Val::Int(x)
    }
}

impl From<String> for Val {
    fn from(s: String) -> Self {
        Val::Str(s)
    }
}

impl From<&str> for Val {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl<T> From<Vec<T>> for Val where Val: From<T> {
    fn from(xs: Vec<T>) -> Self {
        Val::List(xs.into_iter().map(Self::from).collect())
    }
}

impl<K, V> From<HashMap<K, V>> for Val where Val: From<K> + From<V>, K: Clone, V: Clone {
    fn from(m: HashMap<K, V>) -> Self {
        Val::Dict(m.iter().map(|(k, v)| (Self::from(k.clone()), Val::from(v.clone()))).collect())
    }
}
