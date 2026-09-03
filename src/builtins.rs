use std::rc::Rc;
use std::cell::OnceCell;
use std::io::{self, Write};

use im;

use crate::sparsevec::SparseVec;

use crate::val::{Val, AFn};
use crate::create;

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
                _ => panic!("Coll only expects one arg when called")
            }
        },
        Val::Str(s) => {
            let i: i64 = arity1(args);
            s.get(i as usize..i as usize + 1).map(|s1| Val::Str(s1.to_string()))
                .ok_or(Val::from(i))
        },
        _ => panic!("Value is not callable") // TODO give more information about `x`
    }
}

fn eq(xs: Vec<Val>) -> Result<Val, Val> {
    match xs.as_slice() {
        [first] => Ok(first.clone()),
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

fn range(args: Val) -> YRes {
    let (from, to): (i64, i64) = arity2(args);
    Ok(Val::Coll(SparseVec::range(from as usize, to as usize), im::HashMap::new()))
}

fn concat(xs: Vec<Val>) -> YRes {
    let mut res = SparseVec::new();
    for x in xs {
        match x {
            Val::Coll(ys, d) => {
                assert_eq!(d.len(), 0);
                res.append(&ys);
            }
            _ => panic!()
        }
    }
    Ok(Val::Coll(res, im::HashMap::new()))
}

fn start_index(args: Val) -> YRes {
    let coll: Val = arity1(args);
    Ok(coll.start_index().into())
}

fn first_index(args: Val) -> YRes {
    let coll: Val = arity1(args);
    Ok(coll.first_index().into())
}

fn last_index(args: Val) -> YRes {
    let coll: Val = arity1(args);
    coll.last_index().map(Val::from).ok_or(Val::from("no last index"))
}

fn unsparse(args: Val) -> YRes {
    let coll: Val = arity1(args);
    Ok(coll.unsparse())
}

fn bake(args: Val) -> YRes {
    let (f, coll): (Val, Val) = arity2(args);
    coll.bake(|k| call(&f, Val::from(vec![k])))
}

fn bake_some(args: Val) -> YRes {
    let (f, coll): (Val, Val) = arity2(args);
    Ok(coll.bake_some(|k| call(&f, Val::from(vec![k])).ok()))
}

fn reduce(args: Val) -> YRes {
    let (coll, f): (Val, Val) = arity2(args);
    Ok(coll.values().iter().cloned().reduce(|a, b| call(&f, Val::from(vec![a, b])).unwrap()).unwrap())
}

fn union(xs: Vec<Val>) -> YRes {
    Ok(xs.into_iter().reduce(|a, b| a.union(b)).unwrap())
}

fn retain(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [d, predicate] => {
            Ok(d.clone().retain(|k| call(predicate, Val::from(vec![k])).is_ok()))
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

fn rand_choice(xs: Vec<Val>) -> YRes {
    match xs.as_slice() {
        [coll] => {
            use rand;
            use rand::seq::IndexedRandom;
            coll.values().into_iter().collect::<Vec<_>>().choose(&mut rand::rng()).cloned().ok_or("Empty coll".into())
        },
        _ => panic!()
    }
}

fn str(xs: Vec<Val>) -> YRes {
    Ok(xs.iter().map(Val::naked_repr).collect::<Vec<_>>().join("").into())
}

fn split(args: Val) -> YRes {
    let (s, sep): (String, String) = arity2(args);
    Ok(s.split(&sep).collect::<Vec<_>>().into())
}

fn print(xs: Vec<Val>) {
    print!("{}", xs.iter().map(|x| x.naked_pretty().join("\n")).collect::<Vec<_>>().join(" "));
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

fn read_file(args: Val) -> YRes {
    let path: String = arity1(args);
    use std::fs::read_to_string;
    read_to_string(path).map(Val::from).map_err(|e| format!("{}", e).into())
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

fn name_to_lit(inst: Val) -> Val {
    if String::try_from(inst.get("op").unwrap()).unwrap() == "deref" {
        create::lit(inst.get("name").unwrap().clone())
    } else {
        inst
    }
}

fn get_op(form: &Val) -> Option<String> {
    form.get("op").map(|op| String::try_from(op).unwrap())
}

fn op_op(args: Val) -> YRes {
    let (x, xs) = arity1andmore(args);
    match get_op(&x).as_deref() {
        Some("deref") => {
            Ok(create::inst(&String::try_from(x.get("name").unwrap()).unwrap(),
                            xs))
        }
        Some("bind") => {
            let mut ys = vec![x];
            ys.extend(xs);
            Ok(create::inst("dict", ys))
        }
        _ => panic!()
    }
}

fn op_call_coll(args: Val) -> YRes {
    let xs = Vec::try_from(args).unwrap();
    Ok(create::inst("call", vec![xs[0].clone(),
                              create::inst("list", xs[1..].to_vec())]))
}

fn op_dot(xs: Vec<Val>) -> YRes {
    let mut iter = xs.into_iter();
    let mut res = iter.next().unwrap();
    for foo in iter {
        res = create::inst("call", vec![res,
                                        create::inst("list", vec![name_to_lit(foo)])]);
    }
    Ok(res)
}

       fn bind_op_list(args: Val) -> YRes {
           let args: Vec<Val> = Vec::try_from(args).unwrap();
           let list_name = Val::from(gensym("list"));
           Ok(Val::from(im::HashMap::from(vec![
               ("bind", list_name.clone()),
               ("ops", args.into_iter().enumerate().map(|(i, arg)| {
                   create::inst("bind", vec![arg, create::inst("call-coll", vec![create::deref(list_name.clone()),
                                                                                 create::lit(Val::from(i as i64))])])
               })
               .collect::<Vec<_>>().into())
           ])))
       }

fn wrap_list_arg(f: &'static fn(Vec<Val>) -> YRes) -> AFn {
    AFn(Rc::new(|arg: Val| {
        f(arg.try_into().unwrap())
    }))
}

fn wrapf(f: &'static fn(Val) -> YRes) -> AFn {
    AFn(Rc::new(f))
}

use std::fmt::Debug;

fn arity1andmore(v: Val) -> (Val, Vec<Val>) {
    let xs = Vec::try_from(v).unwrap();
    match xs.as_slice() {
        [a, more@..] => (a.clone(), more.to_vec()),
        _ => panic!("Wrong arity, expected at least 1 got {}", xs.len())
    }
}

fn arity1<A>(v: Val) -> A where
    A: TryFrom<Val>, <A as TryFrom<Val>>::Error: Debug {
    let xs = Vec::try_from(v).unwrap();
    match xs.as_slice() {
        [a] => a.clone().try_into().expect(&format!("{}", a.repr())),
        _ => panic!("Wrong arity, expected 1 got {}", xs.len())
    }
}

fn arity2<A, B>(v: Val) -> (A, B) where
    A: TryFrom<Val>, <A as TryFrom<Val>>::Error: Debug,
    B: TryFrom<Val>, <B as TryFrom<Val>>::Error: Debug {
    let xs = Vec::try_from(v).unwrap();
    match xs.as_slice() {
        [a, b] => (a.clone().try_into().unwrap(), b.clone().try_into().unwrap()),
        _ => panic!("Wrong arity, expected 2 got {}", xs.len())
    }
}

pub fn op_op_env() -> Env {
   let mut res: Env = im::HashMap::new();
    res.extend([
        ("op-op", op_op as fn(Val) -> YRes),
    ].iter().map(|(name, f)| (Val::from(*name), Val::Fn(wrapf(f)))));
    res
}

pub fn get() -> Env {
    let mut res: Env = [
        ("=", eq as fn(Vec<Val>) -> YRes),
        ("<", lt as fn(Vec<Val>) -> YRes),
        ("+", plus as fn(Vec<Val>) -> YRes),
        ("-", minus as fn(Vec<Val>) -> YRes),
        ("/", div as fn(Vec<Val>) -> YRes),
        ("++", concat as fn(Vec<Val>) -> YRes),
        ("union", union as fn(Vec<Val>) -> YRes),
        ("retain", retain as fn(Vec<Val>) -> YRes),
        ("negate", negate as fn(Vec<Val>) -> YRes),
        ("rand-choice", rand_choice as fn(Vec<Val>) -> YRes),
        ("str", str as fn(Vec<Val>) -> YRes),
        ("say", say as fn(Vec<Val>) -> YRes),
        ("ask", ask as fn(Vec<Val>) -> YRes),
        ("placeholder-fn", placeholder_fn as fn(Vec<Val>) -> YRes),
        ("gensym", gensym2 as fn(Vec<Val>) -> YRes),
        ("op-dot", op_dot as fn(Vec<Val>) -> YRes),
    ].iter().map(|(name, f)| ((*name).into(), Val::Fn(wrap_list_arg(f)))).collect();
    res.extend([
        ("op-op", op_op as fn(Val) -> YRes),
        ("op-call-coll", op_call_coll as fn(Val) -> YRes),
        ("bind-op-list", bind_op_list as fn(Val) -> YRes),
        ("range", range as fn(Val) -> YRes),
        ("first-index", first_index as fn(Val) -> YRes),
        ("start-index", start_index as fn(Val) -> YRes),
        ("last-index", last_index as fn(Val) -> YRes),
        ("unsparse", unsparse as fn(Val) -> YRes),
        ("bake", bake as fn(Val) -> YRes),
        ("bake-some", bake_some as fn(Val) -> YRes),
        ("reduce", reduce as fn(Val) -> YRes),
        ("split", split as fn(Val) -> YRes),
        ("read-file", read_file as fn(Val) -> YRes),
    ].iter().map(|(name, f)| (Val::from(*name), Val::Fn(wrapf(f)))));
    res
}
