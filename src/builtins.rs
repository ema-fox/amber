use std::rc::Rc;
use std::collections::HashMap;
use std::cell::OnceCell;
use std::io::{self, Write};

use im;

use crate::val::{Val, AFn};

// TODO reconsider where to define these types
pub type Env = HashMap<String, Rc<OnceCell<Val>>>;
pub type YRes = Result<Val, Val>;

pub fn call(x: &Val, args: Val) -> YRes {
    match x {
        Val::Fn(f) => {
            f.0(args)
        },
        Val::Dict(d) => {
            match args {
                Val::List(ys) => {
                    match ys.as_slice() {
                        [y] => get_inner(&d, y),
                        _ => panic!()
                    }
                },
                _ => panic!()
            }
        },
        _ => panic!()
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

fn nth(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [Val::List(ys), Val::Int(i)] => {
            Ok(ys[*i as usize].clone())
        },
        _ => panic!()
    }
}

fn concat(xs: Vec<Val>) -> YRes {
    let mut res = vec![];
    for x in xs {
        match x {
            // I don't understand why I can make ys mut here
            // might fail in unexpected ways
            Val::List(mut ys) => {
                res.append(&mut ys);
            },
        _ => panic!()
        }
    }
    Ok(Val::List(res))
}

fn get_inner(dict: &im::HashMap<Val, Val>, key: &Val) -> YRes {
    if let Some(val) = dict.get(key) {
        Ok(val.clone())
    } else {
        Err(key.clone())
    }
}

fn aget(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [Val::Dict(dict), key] => get_inner(dict, key),
        _ => panic!()
    }
}

fn merge_with(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [Val::Fn(AFn(f)), Val::Dict(d0), Val::Dict(d1)] => {
            Ok(Val::Dict(d0.clone().union_with(d1.clone(), |a, b| f(Val::List(vec![a, b])).unwrap())))
        },
        _ => panic!()
    }
}

fn retain(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [Val::Dict(d), predicate] => {
            let mut res = d.clone();
            res.retain(|k, _v| call(predicate, Val::List(vec![k.clone()])).is_ok());
            Ok(Val::Dict(res))
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


fn say(xs: Vec<Val>) -> YRes {
    for x in &xs {
        match x {
            Val::Str(s) => print!("{}", s),
            _ => panic!()
        }
    }
    println!("");
    Ok(xs[0].clone()) // TODO think about return value
}

fn ask(xs: Vec<Val>) -> YRes {
    for x in &xs {
        match x {
            Val::Str(s) => print!("{}", s),
            _ => panic!()
        }
    }
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    let mut res = "".to_string();
    stdin.read_line(&mut res).unwrap();
    Ok(Val::Str(res.trim_end_matches(&['\r', '\n'][..]).to_string()))
}

fn wrap_list_arg(f: &'static fn(Vec<Val>) -> YRes) -> AFn {
    AFn(Rc::new(|arg: Val| {
        match arg {
            Val::List(xs) => {
                f(xs)
            },
            _ => panic!("{:?} is not a list", arg)
        }
    }))
}

pub fn get() -> Env {
    [
        ("=", eq as fn(Vec<Val>) -> YRes),
        ("<", lt as fn(Vec<Val>) -> YRes),
        ("+", plus as fn(Vec<Val>) -> YRes),
        ("-", minus as fn(Vec<Val>) -> YRes),
        ("/", div as fn(Vec<Val>) -> YRes),
        ("nth", nth as fn(Vec<Val>) -> YRes),
        ("++", concat as fn(Vec<Val>) -> YRes),
        ("get", aget as fn(Vec<Val>) -> YRes),
        ("merge-with", merge_with as fn(Vec<Val>) -> YRes),
        ("retain", retain as fn(Vec<Val>) -> YRes),
        ("negate", negate as fn(Vec<Val>) -> YRes),
        ("say", say as fn(Vec<Val>) -> YRes),
        ("ask", ask as fn(Vec<Val>) -> YRes),
    ].iter().map(|(name, f)| (name.to_string(), Rc::new(OnceCell::from(Val::Fn(wrap_list_arg(f)))))).collect()
}
