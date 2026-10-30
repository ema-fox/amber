use std::sync::Arc;

const BRANCHBITS: usize = 5;
const BRANCH: usize = 1 << BRANCHBITS;

#[derive(Debug, Hash, PartialEq, Eq)]
enum Node<T> {
    Leaf(Arc<T>),
    Branch([Option<Arc<Node<T>>>; BRANCH])
}

impl<T> Clone for Node<T> {
    fn clone(&self) -> Self {
        match self {
            Leaf(x) => Leaf(x.clone()),
            Branch(xs) => Branch(xs.clone())
        }
    }
}

struct ValuesIterator<'a, T: 'a> {
    stack: Vec<core::slice::Iter<'a, Option<Arc<Node<T>>>>>
}

impl<'a, T: 'a> Iterator for ValuesIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if let Some(r) = self.stack.last_mut()?.next() {
            if let Some(node) = r {
                match &**node {
                    Leaf(x) => Some(x),
                    Branch(xs) => {
                        self.stack.push(xs.iter());
                        self.next()
                    }
                }
            } else {
                self.next()
            }
        } else {
            self.stack.pop();
            self.next()
        }
    }
}

struct EntriesIterator<'a, T> {
    idxs: Vec<usize>,
    branches: Vec<&'a [Option<Arc<Node<T>>>; BRANCH]>
}

impl<T> EntriesIterator<'_, T> {
    fn fix(&mut self) -> Option<()> {
        let i = self.idxs.last()?;
        if *i == BRANCH {
            // Leave
            self.idxs.pop();
            self.branches.pop();
            *self.idxs.last_mut()? += 1;
            self.fix()
        } else {
            let xs = self.branches.last()?;
            if let Some(node) = &xs[*i] {
                match &**node {
                    Leaf(_) => Some(()), // Nothing to fix
                    Branch(ys) => {
                        // Enter
                        self.idxs.push(0);
                        self.branches.push(&ys);
                        self.fix()
                    }
                }
            } else {
                // Skip
                *self.idxs.last_mut()? += 1;
                self.fix()
            }
        }
    }
}


impl<T> Iterator for EntriesIterator<'_, T> {
    type Item = (usize, Arc<T>);
    fn next(&mut self) -> Option<Self::Item> {
        self.fix()?;
        let res = self.branches.last()?[*self.idxs.last()?].as_ref().unwrap();
        match &**res {
            Leaf(x) => {
                let i = self.idxs.iter().rev().enumerate()
                    .map(|(lvl, j)| j << (lvl * BRANCHBITS))
                    .sum();
                *self.idxs.last_mut()? += 1;
                Some((i, x.clone()))
            }
            _ => panic!()
        }
    }
}

impl<T> Node<T> {
    fn count (&self) -> usize {
        match self {
            Leaf(_) => 1,
            Branch(xs) => xs.iter().flatten().map(|xref| xref.count()).sum()
        }
    }

    fn entries_arc(&self, j: usize) -> Vec<(usize, Arc<T>)> {
        match self {
            Leaf(x) => vec![(j, x.clone())],
            Branch(xs) => xs.iter().enumerate()
                .filter_map(|(i, x)| x.as_ref().map(|xref| (i, xref)))
                .flat_map(|(i, xref)| xref.entries_arc((j << BRANCHBITS) + i))
                .collect()
        }
    }

    fn values_i(&self) ->  Vec<core::slice::Iter<'_, Option<Arc<Node<T>>>>> {
        match self {
            Leaf(_) => vec![],
            Branch(xs) => vec![xs.iter()]
        }
    }

    fn entries_i(&self) -> EntriesIterator<T> {
        match self {
            Leaf(_) => todo!(),
            Branch(xs) => EntriesIterator {
                idxs: vec![0],
                branches: vec![&xs]
            }
        }
    }

    fn entries(&self, j: usize) -> Vec<(usize, &T)> {
        match self {
            Leaf(x) => vec![(j, &x)],
            Branch(xs) => xs.iter().enumerate()
                .filter_map(|(i, x)| x.as_ref().map(|xref| (i, xref)))
                .flat_map(|(i, xref)| xref.entries((j << BRANCHBITS) + i))
                .collect()
        }
    }

    fn merge(&mut self, other: Self) {
        match (self, other) {
            (Leaf(x), Leaf(y)) => {
                *x = y;
            }
            (Branch(xs), Branch(ys)) => {
                for (i, y) in ys.into_iter().enumerate() {
                    if let Some(mut yref) = y {
                        if let Some(xref) = &mut xs[i] {
                            let xx = Arc::make_mut(xref);
                            Arc::make_mut(&mut yref);
                            xx.merge(Arc::into_inner(yref).unwrap());
                        } else {
                            xs[i] = Some(yref);
                        }
                    }
                }
            }
            _ => panic!()
        }
    }
}

use Node::*;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SparseVec<T> {
    root: Arc<Node<T>>,
    level: usize
}

fn find_level(mut i: usize) -> usize {
    let mut lvl = 0;
    while i != 0 {
        i >>= BRANCHBITS;
        lvl += 1;
    }
    lvl
}

fn level_bits(i: usize, lvl: usize) -> usize {
    (i >> (lvl * BRANCHBITS)) & const {(1 << BRANCHBITS) - 1}
}

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        SparseVec {
            root: Arc::new(Branch([const {None}; BRANCH])),
            level: 0
        }
    }

    pub fn unit_arc(i: usize, v: Arc<T>) -> Self {
        let mut child = Leaf(v);
        for lvl in 0..(find_level(i) + 1) {
            let mut xs = [const {None}; BRANCH];
            xs[level_bits(i, lvl)] = Some(Arc::new(child));
            child = Branch(xs);
        }
        SparseVec {
            root: Arc::new(child),
            level: find_level(i)
        }
    }

    pub fn unit(i: usize, v: T) -> Self {
        let mut child = Leaf(Arc::new(v));
        for lvl in 0..(find_level(i) + 1) {
            let mut xs = [const {None}; BRANCH];
            xs[level_bits(i, lvl)] = Some(Arc::new(child));
            child = Branch(xs);
        }
        SparseVec {
            root: Arc::new(child),
            level: find_level(i)
        }
    }

    pub fn from_entries_arc(xs: Vec<(usize, Arc<T>)>) -> Self {
        let mut res = Self::new();
        for (i, x) in xs {
            res.merge(Self::unit_arc(i, x));
        }
        res
    }

    pub fn from_entries(xs: Vec<(usize, T)>) -> Self {
        let mut res = Self::new();
        for (i, x) in xs {
            res.merge(Self::unit(i, x));
        }
        res
    }

    pub fn count(&self) -> usize {
        self.root.count()
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        if find_level(i) > self.level {
            return None;
        }

        let mut node = &*self.root;
        let mut lvl = self.level;

        loop {
            match node {
                Leaf(v) => {
                    debug_assert_eq!(lvl, 0);
                    return Some(v);
                }
                Branch(xs) => {
                    if let Some(node2) = &xs[level_bits(i, lvl)] {
                        node = &*node2;
                        if let Branch(_) = &node {
                            lvl -= 1;
                        }
                    } else { return None }
                }
            }
        }
    }

    pub fn entries_arc(&self) -> Vec<(usize, Arc<T>)> {
        return self.entries_i().collect();
    }

    pub fn entries(&self) -> Vec<(usize, &T)> {
        self.root.entries(0)
    }

    pub fn values_i(&self) -> impl Iterator<Item = &T> {
        ValuesIterator {stack: self.root.values_i()}
    }

    fn entries_i(&self) -> EntriesIterator<T> {
        self.root.entries_i()
    }

    pub fn first_index(&self) -> Option<usize> {
        Some(self.entries_i().next()?.0)
    }

    pub fn last_index(&self) -> Option<usize> {
        self.entries().last().map(|(i, _)| *i)
    }

    pub fn start_index(&self) -> usize {
        self.first_index().unwrap_or(0)
    }

    pub fn next_index(&self) -> usize {
        self.last_index().map(|i| i + 1).unwrap_or(0)
    }

    pub fn unsparse(&self) -> Vec<&T> {
        self.entries().into_iter().map(|(_, x)| x).collect()
    }

    pub fn retain(&self, f: impl Fn(usize) -> bool) -> Self {
         SparseVec::from_entries_arc(self.entries_i().filter_map(|(i, x)| {
             if f(i) {
                 Some((i, x))
             } else {
                 None
             }
        }).collect())
    }

    pub fn bake<D, E>(&self, f: impl Fn(usize) -> Result<Arc<D>, E>) -> Result<SparseVec<D>, E> {
        Ok(SparseVec::from_entries_arc(self.entries_i().map(|(i, _)| {
            f(i).map(|x| (i, x))
        }).collect::<Result<_, _>>()?))
    }

    pub fn bake_some<D>(&self, f: impl Fn(usize) -> Option<D>) -> SparseVec<D> {
        SparseVec::from_entries(self.entries_i().filter_map(|(i, _)| {
            f(i).map(|x| (i, x))
        }).collect())
    }

    fn add_level(&mut self) {
        let mut branch = [const {None}; BRANCH];
        branch[0] = Some(self.root.clone());
        self.root = Arc::new(Branch(branch));

        self.level += 1;
    }

    pub fn merge(&mut self, mut other: Self) {
        while self.level < other.level { self.add_level() }
        while other.level < self.level { other.add_level() }

        Arc::make_mut(&mut other.root);
        Arc::make_mut(&mut self.root).merge(Arc::into_inner(other.root).unwrap());
    }

    pub fn insert(&mut self, i: usize, v: T) {
        self.insert_arc(i, Arc::new(v));
    }

    pub fn insert_arc(&mut self, i: usize, v: Arc<T>) {
        while find_level(i) > self.level { self.add_level(); }

        let mut node = Arc::make_mut(&mut self.root);
        let mut lvl = self.level;

        loop {
            match node {
                Leaf(_) => {
                    unreachable!();
                }
                Branch(xs) => {
                    match &mut xs[level_bits(i, lvl)] {
                        &mut Some(ref mut node2) => {
                            node = Arc::make_mut(node2);
                            if lvl == 0 {
                                *node = Leaf(v);
                                return;
                            }
                            if let Branch(_) = &node {
                                lvl -= 1;
                            }
                        }
                        z => {
                            if lvl == 0 {
                                *z = Some(Arc::new(Leaf(v)));
                                return;
                            } else {
                                *z = Some(Arc::new(Branch([const {None}; BRANCH])));
                                // loop again over this place
                                node = Arc::make_mut(z.as_mut().unwrap());
                                lvl -= 1;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn append(&mut self, other: &Self) {
        let offset: i64 = self.next_index() as i64 - other.start_index() as i64;
        for (i, x) in other.entries_i() {
            self.insert_arc((i as i64 + offset) as usize, x);
        }
    }
}

impl<T: From<usize>> SparseVec<T> {
    pub fn range(from: usize, to: usize) -> Self {
        Self::from_entries((from..to).map(|i| (i, i.into())).collect())
    }
}

impl<T> std::ops::Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).expect("Index out of bounds")
    }
}

impl<T> From<Vec<T>> for SparseVec<T> {
    fn from(xs: Vec<T>) -> Self {
        Self::from_entries(xs.into_iter().enumerate().collect())
    }
}

impl<T> From<Vec<Arc<T>>> for SparseVec<T> {
    fn from(xs: Vec<Arc<T>>) -> Self {
        Self::from_entries_arc(xs.into_iter().enumerate().collect())
    }
}

impl<T: Clone> TryFrom<SparseVec<T>> for Vec<T> {
    type Error = &'static str;

    fn try_from(xs: SparseVec<T>) -> Result<Self, Self::Error> {
        if xs.start_index() != 0 {
            return Err("Not starting at 0");
        }
        if xs.next_index() != xs.count() {
            dbg!(xs.next_index());
            dbg!(xs.count());
            return Err("Not contiguous");
        }
        Ok(xs.unsparse().into_iter().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entries_i() {
        let ys = vec![3, 4, 5, 1, 3];
        let xs: SparseVec<isize> = ys.clone().into();
        assert_eq!(xs.entries_i().map(|(a, b)| (a, *b)).collect::<Vec<_>>(),
                   ys.into_iter().enumerate().collect::<Vec<_>>());
    }
}
