use std::rc::Rc;
use std::cell::OnceCell;
use std::io::{self, Write};

use im;

use crate::val::{Val, AFn, SparseVec};

// TODO reconsider where to define these types
pub type Env = im::HashMap<Val, Val>;
pub type YRes = Result<Val, Val>;

pub fn call(x: &Val, args: Val) -> YRes {
    match x {
        Val::Fn(f) => {
            f.0(args)
        },
        Val::Coll(_, _) => {
            let ys: Vec<_> = args.try_into().unwrap();
            match ys.as_slice() {
                [key] => x.get(key.clone()).cloned().ok_or(key.clone()),
                _ => panic!()
            }
        },
        _ => panic!("Value is not callable") // TODO give more information about `x`
    }
}

fn eq(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [first, second] => {
            if first == second {
                Ok(xs[0].clone())
            } else {
                Err(xs[0].clone())
            }
        },
        _ => panic!()
    }
}

fn lt(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [Val::Int(first), Val::Int(second)] => {
            if first < second {
                Ok(xs[0].clone())
            } else {
                Err(xs[0].clone())
            }
        },
        _ => panic!()
    }
}

fn plus(xs: Vec<Val>) -> Result<Val, Val> {
    Ok(Val::Int(xs.iter().map(|x| {
        match x {
            Val::Int(n) => n,
            _ => panic!()
        }
    }).sum()))
}

fn minus(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [Val::Int(x), xs @ ..] =>
            Ok(Val::Int(x - xs.iter().map(|x| {
                match x {
                    Val::Int(n) => n,
                    _ => panic!()
                }
            }).sum::<i64>())),
        _ => panic!()
    }
}

fn div(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [Val::Int(x), xs @ ..] =>
            Ok(Val::Int(x / xs.iter().map(|x| {
                match x {
                    Val::Int(n) => n,
                    _ => panic!()
                }
            }).product::<i64>())),
        _ => panic!()
    }
}

fn concat(xs: Vec<Val>) -> YRes {
    let mut res = SparseVec::new();
    for x in xs {
        match x {
            Val::Coll(ys, d) => {
                assert_eq!(d.len(), 0);
                res = res + ys;
            }
            _ => panic!()
        }
    }
    Ok(Val::Coll(res, im::HashMap::new()))
}

fn map_indexed(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [coll, Val::Fn(AFn(f))] => {
            Ok(coll.clone().map_indexed(&|i: i64, entry: Val| {
                f(Val::from(vec![entry.clone(), i.into()])).unwrap()
            }))
        },
        _ => panic!()
    }
}

fn merge_with(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [Val::Fn(AFn(f)), c0, c1] => {
            let d0: im::HashMap<Val, Val> = c0.clone().try_into().unwrap();
            let d1: im::HashMap<Val, Val> = c1.clone().try_into().unwrap();
            Ok(Val::from(d0.union_with(d1, |a, b| f(Val::from(vec![a, b])).unwrap())))
        },
        _ => panic!()
    }
}

fn retain(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [d, predicate] => {
            let mut res = im::HashMap::try_from(d.clone()).unwrap();
            res.retain(|k, _v| call(predicate, Val::from(vec![k.clone()])).is_ok());
            Ok(Val::from(res))
        },
        _ => panic!()
    }
}

fn negate(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [f] => {
            let f = f.clone();
            Ok(Val::Fn(AFn(Rc::new(move |arg: Val| match call(&f, arg) {
                Ok(x) => Err(x),
                Err(x) => Ok(x)
            }))))
        },
        _ => panic!()
    }
}

fn print(xs: Vec<Val>) {
    print!("{}", xs.iter().map(Val::naked_repr).collect::<Vec<_>>().join(" "));
}

fn say(xs: Vec<Val>) -> YRes {
    print(xs.clone());
    println!("");
    Ok(xs[0].clone()) // TODO think about return value
}

fn ask(xs: Vec<Val>) -> YRes {
    print(xs);
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    let mut res = "".to_string();
    stdin.read_line(&mut res).unwrap();
    Ok(Val::Str(res.trim_end_matches(&['\r', '\n'][..]).to_string()))
}

fn placeholder_fn(_xs: Vec<Val>) -> YRes {
    /*
    TODO this is a plumbing function, the porcelain will be something like:
    {recursive
      foo: {fn ...}
      bar: {fn ...}}
    */
    let place: Rc<OnceCell<AFn>> = Rc::new(OnceCell::new());
    let place2 = place.clone();
    Ok(vec![
        Val::Fn(AFn(Rc::new(move |arg: Val| place.get().unwrap().0(arg)))),
        Val::Fn(AFn(Rc::new(move |arg: Val| {
            let args: Vec<Val> = arg.try_into().unwrap();
            if let Val::Fn(f) = args[0].clone() {
                place2.set(f).unwrap();
            } else {
                panic!();
            }
            Ok(0.into()) // TODO think about return value
        })))
    ].into())
}

static mut COUNTER: usize = 0;

fn get_uniq_number() -> usize {
    // TODO make sure this is thread-safe
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

pub fn gensym(base: &str) -> String {
    format!("{}{}", base, get_uniq_number())
}

fn gensym2(xs: Vec<Val>) -> YRes {
    Ok(gensym(&String::try_from(xs[0].clone()).unwrap()).into())
}

fn wrap_list_arg(f: &'static fn(Vec<Val>) -> YRes) -> AFn {
    AFn(Rc::new(|arg: Val| {
        f(arg.try_into().unwrap())
    }))
}

pub fn get() -> Env {
    [
        ("=", eq as fn(Vec<Val>) -> YRes),
        ("<", lt as fn(Vec<Val>) -> YRes),
        ("+", plus as fn(Vec<Val>) -> YRes),
        ("-", minus as fn(Vec<Val>) -> YRes),
        ("/", div as fn(Vec<Val>) -> YRes),
        ("++", concat as fn(Vec<Val>) -> YRes),
        ("map-indexed", map_indexed as fn(Vec<Val>) -> YRes),
        ("merge-with", merge_with as fn(Vec<Val>) -> YRes),
        ("retain", retain as fn(Vec<Val>) -> YRes),
        ("negate", negate as fn(Vec<Val>) -> YRes),
        ("say", say as fn(Vec<Val>) -> YRes),
        ("ask", ask as fn(Vec<Val>) -> YRes),
        ("placeholder-fn", placeholder_fn as fn(Vec<Val>) -> YRes),
        ("gensym", gensym2 as fn(Vec<Val>) -> YRes),
    ].iter().map(|(name, f)| ((*name).into(), Val::Fn(wrap_list_arg(f)))).collect()
}
