use std::rc::Rc;

use im::{HashMap, Vector};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SparseVec<T: Clone>(Vector<(i64, Vector<T>)>);

impl<T: Clone> From<Vec<T>> for SparseVec<T> {
    fn from(xs: Vec<T>) -> Self {
        Self(vec![(0, xs.into())].into())
    }
}

impl<T: Clone> TryFrom<SparseVec<T>> for Vec<T> {
    type Error = &'static str;

    fn try_from(v: SparseVec<T>) -> Result<Self, Self::Error> {
        let chunks = v.0;
        match chunks.len() {
            0 => Ok(vec![]),
            1 => match &chunks[0] {
                (0, xs) => Ok(xs.iter().map(Clone::clone).collect()),
                _ => Err("Not starting at 0")
            },
            _ => Err("Not contigous")
        }
    }
}


impl<T: Clone> std::ops::Index<i64> for SparseVec<T> {
    type Output = T;

    fn index(&self, index: i64) -> &T {
        for (offset, chunk) in self.0.iter() {
            if *offset <= index && index < offset + chunk.len() as i64 {
                return &chunk[(index - offset) as usize];
            }
        }
        panic!("Index out of bounds");
    }
}

impl<T: Clone> std::ops::Add for SparseVec<T> {
    type Output = SparseVec<T>;

    fn add(mut self, mut other: Self) -> Self {
        if let Some(a) = self.0.pop_back() {
            if let Some(b) = other.0.pop_front() {
                let offset_move = a.0 + a.1.len() as i64 - b.0;
                let ab = (a.0, a.1 + b.1);
                self.0.push_back(ab);
                self.0.extend(other.0.iter().map(|(offset, chunk)| {
                    (offset + offset_move, chunk.clone())
                }));
                self
            } else {
                self.0.push_back(a);
                self
            }
        } else {
            other
        }
    }
}

impl<T: Clone> SparseVec<T> {
    pub fn new() -> Self {
        Self(Vector::new())
    }

    pub fn count(self) -> usize {
        self.0.iter().map(|(_, chunk)| chunk.len()).sum()
    }

    pub fn map_indexed<D: Clone>(&self, f: impl Fn(i64, T) -> D) -> SparseVec<D> {
        SparseVec(self.0.iter().map(|(offset, chunk)| {
            (*offset, chunk.iter().enumerate().map(|(i, entry)| {
                f(offset + i as i64, entry.clone())
            }).collect())
        }).collect())
    }
}

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
                Some(&xs[i])
            } else {
                d.get(&k2)
            }
            _ =>{
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
                todo!();
            } else {
                d.insert(k2, v.into());
            }
        } else {
            panic!();
        }
    }

    pub fn map_indexed(self, f: &dyn Fn(i64, Val) -> Val) -> Self {
        if let Val::Coll(xs, d) = self {
            // TODO also map over d
            assert_eq!(d.len(), 0);
            Val::Coll(xs.map_indexed(f), HashMap::new())
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
                for (offset, chunk) in &xs.0 {
                    if *offset != 0 {
                        elements.push(format!("{}:", offset));
                    }
                    for entry in chunk {
                        elements.push(entry.repr());
                    }
                }
                for (k, v) in d {
                    elements.push(format!("{}:", k.naked_repr()));
                    elements.push(v.repr());
                }
                format!("[{}]", elements.join(" "))
            },
            Val::Fn(_) => format!("<Fn>"),
        }
    }

    pub fn naked_repr(&self) -> String {
        if let Val::Str(s) = self {
            s.clone()
        } else {
            self.repr()
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
        if let Val::Coll(xs, d) = v {
            if xs.count() != 0 {
                todo!();
            }
            Ok(d)
        } else {
            Err("Not a Val::Coll")
        }
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
        Val::Coll(SparseVec::new(), m.iter().map(|(k, v)| (Self::from(k.clone()), Val::from(v.clone()))).collect())
    }
}
