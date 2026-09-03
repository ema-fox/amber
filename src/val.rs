use std::rc::Rc;

use im::{HashMap, Vector};
use crate::sparsevec::SparseVec;

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
    Coll(SparseVec<Val>, HashMap<Val, Val>),
    Fn(AFn)
}


impl Val {
    pub fn get<K>(&self, k: K) -> Option<&Val>
    where
        K: Into<Val>, Val: From<K>
    {
        let k2: Val = k.into();
        match self {
            Val::Coll(xs, d) => if let Val::Int(i) = k2 {
                xs.get(i as usize)
            } else {
                d.get(&k2)
            }
            _ => {
                None
            }
        }
    }

    pub fn insert<K, V>(&mut self, k: K, v: V)
    where
        K: Into<Val>,
        V: Into<Val>
    {
        if let Val::Coll(xs, d) = self {
            let k2: Val = k.into();
            if let Val::Int(i) = k2 {
                todo!("{:?}{:?}", xs, i);
            } else {
                d.insert(k2, v.into());
            }
        } else {
            panic!();
        }
    }
    pub fn first_index(&self) -> i64 {
        if let Val::Coll(xs, _) = self {
            xs.first_index().unwrap() as i64
        } else {
            panic!();
        }
    }

    pub fn start_index(&self) -> i64 {
        if let Val::Coll(xs, _) = self {
            xs.start_index() as i64
        } else {
            panic!();
        }
    }

    pub fn last_index(&self) -> Option<i64> {
        if let Val::Coll(xs, _) = self {
            xs.last_index().map(|i| i as i64)
        } else {
            panic!();
        }
    }

    pub fn values(&self) -> Vector<Val> {
        if let Val::Coll(xs, d) = self {
            let mut res: Vector<Val> = xs.unsparse().into_iter().cloned().collect();
            res.extend(d.values().cloned());
            res
        } else {
            panic!();
        }
    }

    pub fn union(self, other: Self) -> Self {
        if let (Val::Coll(mut xs, d), Val::Coll(ys, d2)) = (self, other) {
            xs.merge(ys);
            Val::Coll(xs, d.union_with(d2, |_, b| b))
        } else {
            panic!();
        }
    }

    pub fn retain(self, f: impl Fn(Val) -> bool) -> Self {
        if let Val::Coll(xs, d) = self {
            let mut dres = d.clone();
            dres.retain(|k, _v| f(k.clone()));
            Val::Coll(xs.retain(|i| f(Val::from(i as i64))), dres)
        } else {
            panic!();
        }
    }

    pub fn unsparse(&self) -> Self {
         if let Val::Coll(xs, d) = self {
             Val::Coll(SparseVec::from(xs.unsparse().into_iter().cloned().collect::<Vec<_>>()),
                       d.clone())
        } else {
            panic!();
        }
    }

    pub fn bake<E>(&self, f: impl Fn(Val) -> Result<Val, E>) -> Result<Val, E> {
        if let Val::Coll(xs, d) = self {
            Ok(Val::Coll(xs.bake(|i| f(Val::from(i as i64)))?,
                         HashMap::from(d.keys().map(|k| f(k.clone()).map(|x| (k.clone(), x)))
                                       .collect::<Result<Vec<_>, _>>()?)))
        } else {
            panic!();
        }
    }

    pub fn bake_some(&self, f: impl Fn(Val) -> Option<Val>) -> Self {
        // TODO reconsider name
        if let Val::Coll(xs, d) = self {
            Val::Coll(xs.bake_some(|i| f(Val::from(i as i64))),
                      HashMap::from(d.keys().filter_map(|k| f(k.clone()).map(|x| (k.clone(), x)))
                      .collect::<Vec<_>>()))
        } else {
            panic!();
        }
    }

    pub fn repr(&self) -> String {
        match self {
            Val::Str(s) => format!("\"{}\"", s), // TODO escaping
            Val::Int(x) => format!("{}", x),
            Val::Coll(xs, d) => {
                let mut elements = vec![];
                let mut expected_i = 0;
                for (i, v) in xs.entries() {
                    if i != expected_i {
                        elements.push(format!("{}:", i));
                    }
                    elements.push(v.repr());
                    expected_i = i + 1;
                }
                for (k, v) in d {
                    elements.push(format!("{}:", k.naked_repr()));
                    elements.push(v.repr());
                }
                let body = elements.join(" ");
                if xs.count() > 0 {
                    format!("[{}]", body)
                } else {
                    format!("{{{}}}", body)
                }
            },
            Val::Fn(_) => format!("<Fn>"),
        }
    }

    pub fn pretty(&self) -> Vec<String> {
        match self {
            Val::Coll(xs, d) => {
                let mut elements = vec![];
                let mut expected_i = 0;
                for (i, v) in xs.entries() {
                    if i != expected_i {
                        elements.push(format!("{}:", i));
                    }
                    elements.append(&mut v.pretty());
                    expected_i = i + 1;
                }
                for (k, v) in d {
                    let mut vp = v.pretty();
                    if vp.len() == 1 {
                        elements.push(format!("{}: {}", k.naked_repr(), vp[0]));
                    } else {
                        elements.push(format!("{}:", k.naked_repr()));
                        elements.append(&mut vp);
                    }
                }
                let (open, close) = if xs.count() > 0 {
                    ('[', ']')
                } else {
                    ('{', '}')
                };
                elements.get_mut(0).unwrap().insert(0, open);
                elements.last_mut().unwrap().push(close);
                let mut iter = elements.into_iter();
                elements = vec![iter.next().unwrap()];
                elements.extend(iter.map(|mut s| {s.insert(0, ' '); s}));
                if elements.iter().map(String::len).sum::<usize>() < 40 {
                    vec![elements.join("")]
                } else {
                    elements
                }
            }
            _ => vec![self.repr()]
        }
    }

    pub fn naked_repr(&self) -> String {
        if let Val::Str(s) = self {
            s.clone()
        } else {
            self.repr()
        }
    }

    pub fn naked_pretty(&self) -> Vec<String> {
        if let Val::Str(s) = self {
            vec![s.clone()]
        } else {
            self.pretty()
        }
    }
}

impl std::ops::Index<usize> for Val {
    type Output = Val;

    fn index(&self, index: usize) -> &Val {
        if let Val::Coll(xs, _) = self {
            &xs[index.try_into().unwrap()]
        } else {
            panic!("Not a list");
        }
    }
}

impl TryFrom<Val> for i64 {
    type Error = &'static str;

    fn try_from(v: Val) -> Result<Self, Self::Error> {
        if let Val::Int(i) = v {
            Ok(i)
        } else {
            dbg!(&v);
            Err("Not a Val::Int")
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
        if let Val::Coll(xs, d) = v {
            if d.len() != 0 {
                return Err("Collection has non-integer keys");
            }
            xs.try_into()
        } else {
            Err("Not a Val::Coll")
        }
    }
}

impl TryFrom<Val> for im::HashMap<Val, Val> {
    type Error = &'static str;

    fn try_from(v: Val) -> Result<Self, Self::Error> {
        if let Val::Coll(xs, mut d) = v {
            d.extend(xs.entries().into_iter().map(|(i, v)| (Val::from(i), v)));
            Ok(d)
        } else {
            Err("Not a Val::Coll")
        }
    }
}

// TODO is this a good idea?
impl From<&Val> for Val {
    fn from(v: &Val) -> Self {
        v.clone()
    }
}

impl From<i32> for Val {
    fn from(x: i32) -> Self {
        Val::Int(x as i64)
    }
}

impl From<i64> for Val {
    fn from(x: i64) -> Self {
        Val::Int(x)
    }
}

impl From<usize> for Val {
    fn from(x: usize) -> Self {
        Val::Int(x as i64)
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
        Val::Coll(xs.into_iter().map(Self::from).collect::<Vec<_>>().into(), HashMap::new())
    }
}

impl<K, V> From<HashMap<K, V>> for Val where Val: From<K> + From<V>, K: Clone, V: Clone {
    fn from(m: HashMap<K, V>) -> Self {
        // TODO put integer keys into sparseVec
        Val::Coll(SparseVec::new(), m.iter().map(|(k, v)| (Self::from(k.clone()), Val::from(v.clone()))).collect())
    }
}
