use std::rc::Rc;
use std::sync::Arc;
use std::cell::OnceCell;
use std::io::{self, Write};

use im;

use crate::sparsevec::SparseVec;

use crate::val::{Val, Ref, Res, AFn, refe, ok};
use crate::create;

// TODO reconsider where to define these types
pub type Env = im::HashMap<Val, Val>;

pub fn call(x: &Val, args: Ref) -> Res {
    match x {
        Val::Fn(f) => {
            f.0(args)
        },
        Val::Coll(_, _) => {
            let ys: Vec<_> = (*args).clone().try_into().unwrap();
            match ys.as_slice() {
                [key] => x.get(key.clone()).cloned().map(Arc::new).ok_or_else(|| Arc::new(key.clone())),
                _ => panic!("Coll only expects one arg when called")
            }
        },
        Val::Str(s) => {
            let i: i64 = arity1(args);
            s.get(i as usize..i as usize + 1).map(|s1| Arc::new(Val::Str(s1.to_string())))
                .ok_or_else(|| Arc::new(Val::from(i)))
        },
        _ => panic!("Value is not callable") // TODO give more information about `x`
    }
}

fn eq(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [first] => ok(first.clone()),
        [first, second] => {
            if first == second {
                ok(xs[0].clone())
            } else {
                Err(Arc::new(xs[0].clone()))
            }
        },
        _ => panic!()
    }
}

fn lt(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [Val::Int(first), Val::Int(second)] => {
            if first < second {
                ok(xs[0].clone())
            } else {
                Err(Arc::new(xs[0].clone()))
            }
        },
        _ => panic!()
    }
}

fn plus(xs: Vec<Val>) -> Res {
    ok(Val::Int(xs.iter().map(|x| {
        match x {
            Val::Int(n) => n,
            _ => panic!()
        }
    }).sum()))
}

fn minus(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [Val::Int(x), xs @ ..] =>
            ok(Val::Int(x - xs.iter().map(|x| {
                match x {
                    Val::Int(n) => n,
                    _ => panic!()
                }
            }).sum::<i64>())),
        _ => panic!()
    }
}

fn div(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [Val::Int(x), xs @ ..] =>
            ok(Val::Int(x / xs.iter().map(|x| {
                match x {
                    Val::Int(n) => n,
                    _ => panic!()
                }
            }).product::<i64>())),
        _ => panic!()
    }
}

fn range(args: Ref) -> Res {
    let (from, to): (i64, i64) = arity2(args);
    ok(Val::Coll(SparseVec::range(from as usize, to as usize), im::HashMap::new()))
}

fn concat(xs: Vec<Val>) -> Res {
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
    ok(Val::Coll(res, im::HashMap::new()))
}

fn start_index(args: Ref) -> Res {
    let coll: Val = arity1(args);
    ok(coll.start_index())
}

fn first_index(args: Ref) -> Res {
    let coll: Val = arity1(args);
    ok(coll.first_index())
}

fn last_index(args: Ref) -> Res {
    let coll: Val = arity1(args);
    coll.last_index().map(refe).ok_or_else(|| Arc::new(Val::from("no last index")))
}

fn unsparse(args: Ref) -> Res {
    let coll: Val = arity1(args);
    ok(coll.unsparse())
}

fn bake(args: Ref) -> Res {
    let (f, coll): (Val, Val) = arity2(args);
    coll.bake(|k| call(&f, refe(vec![k])).map(|x| (*x).clone())).map(Arc::new)
}

fn bake_some(args: Ref) -> Res {
    let (f, coll): (Val, Val) = arity2(args);
    ok(coll.bake_some(|k| call(&f, refe(vec![k])).map(|x| (*x).clone()).ok()))
}

fn reduce(args: Ref) -> Res {
    let (coll, f): (Val, Val) = arity2(args);
    ok(coll.values().iter().cloned().reduce(|a, b| (*call(&f, refe(vec![a, b])).unwrap()).clone()).unwrap())
}

fn union(xs: Vec<Val>) -> Res {
    ok(xs.into_iter().reduce(|a, b| a.union(b)).unwrap())
}

fn retain(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [d, predicate] => {
            ok(d.clone().retain(|k| call(predicate, refe(vec![k])).is_ok()))
        },
        _ => panic!()
    }
}

fn negate(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [f] => {
            let f = f.clone();
            ok(Val::Fn(AFn(Rc::new(move |arg: Ref| match call(&f, arg) {
                Ok(x) => Err(x),
                Err(x) => Ok(x)
            }))))
        },
        _ => panic!()
    }
}

fn rand_choice(xs: Vec<Val>) -> Res {
    match xs.as_slice() {
        [coll] => {
            use rand;
            use rand::seq::IndexedRandom;
            coll.values().into_iter().collect::<Vec<_>>().choose(&mut rand::rng()).cloned().map(refe).ok_or_else(|| Arc::new("Empty coll".into()))
        },
        _ => panic!()
    }
}

fn str(xs: Vec<Val>) -> Res {
    ok(xs.iter().map(Val::naked_repr).collect::<Vec<_>>().join(""))
}

fn split(args: Ref) -> Res {
    let (s, sep): (String, String) = arity2(args);
    ok(s.split(&sep).collect::<Vec<_>>())
}

fn print(xs: Vec<Val>) {
    print!("{}", xs.iter().map(|x| x.naked_pretty().join("\n")).collect::<Vec<_>>().join(" "));
}

fn say(xs: Vec<Val>) -> Res {
    print(xs.clone());
    println!("");
    ok(xs[0].clone()) // TODO think about return value
}

fn ask(xs: Vec<Val>) -> Res {
    print(xs);
    io::stdout().flush().unwrap();

    let stdin = io::stdin();
    let mut res = "".to_string();
    stdin.read_line(&mut res).unwrap();
    ok(Val::Str(res.trim_end_matches(&['\r', '\n'][..]).to_string()))
}

fn read_file(args: Ref) -> Res {
    let path: String = arity1(args);
    use std::fs::read_to_string;
    read_to_string(path).map(refe).map_err(|e| Arc::new(format!("{}", e).into()))
}

fn placeholder_fn(_xs: Vec<Val>) -> Res {
    /*
    TODO this is a plumbing function, the porcelain will be something like:
    {recursive
      foo: {fn ...}
      bar: {fn ...}}
    */
    let place: Rc<OnceCell<AFn>> = Rc::new(OnceCell::new());
    let place2 = place.clone();
    ok(vec![
        Val::Fn(AFn(Rc::new(move |arg: Ref| place.get().unwrap().0(arg)))),
        Val::Fn(AFn(Rc::new(move |arg: Ref| {
            let args: Vec<Val> = (*arg).clone().try_into().unwrap();
            if let Val::Fn(f) = args[0].clone() {
                place2.set(f).unwrap();
            } else {
                panic!();
            }
            ok(0) // TODO think about return value
        })))
    ])
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

fn gensym2(xs: Vec<Val>) -> Res {
    ok(gensym(&String::try_from(xs[0].clone()).unwrap()))
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

fn op_op(args: Ref) -> Res {
    let (x, xs) = arity1andmore(args);
    match get_op(&x).as_deref() {
        Some("deref") => {
            ok(create::inst(&String::try_from(x.get("name").unwrap()).unwrap(),
                            xs))
        }
        Some("bind") => {
            let mut ys = vec![x];
            ys.extend(xs);
            ok(create::inst("dict", ys))
        }
        _ => panic!()
    }
}

fn op_call_coll(args: Ref) -> Res {
    let xs = Vec::try_from((*args).clone()).unwrap();
    ok(create::inst("call", vec![xs[0].clone(),
                              create::inst("list", xs[1..].to_vec())]))
}

fn op_dot(xs: Vec<Val>) -> Res {
    let mut iter = xs.into_iter();
    let mut res = iter.next().unwrap();
    for foo in iter {
        res = create::inst("call", vec![res,
                                        create::inst("list", vec![name_to_lit(foo)])]);
    }
    ok(res)
}

fn bind_op_list(args: Ref) -> Res {
    let args: Vec<Val> = Vec::try_from((*args).clone()).unwrap();
    let list_name = Val::from(gensym("list"));
    ok(im::HashMap::from(vec![
        ("bind", list_name.clone()),
        ("ops", args.into_iter().enumerate().map(|(i, arg)| {
            create::inst("bind", vec![arg, create::inst("call-coll", vec![create::deref(list_name.clone()),
                                                                          create::lit(Val::from(i as i64))])])
        })
         .collect::<Vec<_>>().into())
    ]))
}

fn wrap_list_arg(f: &'static fn(Vec<Val>) -> Res) -> AFn {
    AFn(Rc::new(|arg: Ref| {
        f((*arg).clone().try_into().unwrap())
    }))
}

fn wrapf(f: &'static fn(Ref) -> Res) -> AFn {
    AFn(Rc::new(f))
}

use std::fmt::Debug;

fn arity1andmore(v: Ref) -> (Val, Vec<Val>) {
    let xs = Vec::try_from((*v).clone()).unwrap();
    match xs.as_slice() {
        [a, more@..] => (a.clone(), more.to_vec()),
        _ => panic!("Wrong arity, expected at least 1 got {}", xs.len())
    }
}

fn arity1<A>(v: Ref) -> A where
    A: TryFrom<Val>, <A as TryFrom<Val>>::Error: Debug {
    let xs = Vec::try_from((*v).clone()).unwrap();
    match xs.as_slice() {
        [a] => a.clone().try_into().unwrap_or_else(|_| panic!("{}", a.repr())),
        _ => panic!("Wrong arity, expected 1 got {}", xs.len())
    }
}

fn arity2<A, B>(v: Ref) -> (A, B) where
    A: TryFrom<Val>, <A as TryFrom<Val>>::Error: Debug,
    B: TryFrom<Val>, <B as TryFrom<Val>>::Error: Debug {
    let xs = Vec::try_from((*v).clone()).unwrap();
    match xs.as_slice() {
        [a, b] => (a.clone().try_into().unwrap(), b.clone().try_into().unwrap()),
        _ => panic!("Wrong arity, expected 2 got {}", xs.len())
    }
}

pub fn op_op_env() -> Env {
   let mut res: Env = im::HashMap::new();
    res.extend([
        ("op-op", op_op as fn(Ref) -> Res),
    ].iter().map(|(name, f)| (Val::from(*name), Val::Fn(wrapf(f)))));
    res
}

pub fn get() -> Env {
    let mut res: Env = [
        ("=", eq as fn(Vec<Val>) -> Res),
        ("<", lt as fn(Vec<Val>) -> Res),
        ("+", plus as fn(Vec<Val>) -> Res),
        ("-", minus as fn(Vec<Val>) -> Res),
        ("/", div as fn(Vec<Val>) -> Res),
        ("++", concat as fn(Vec<Val>) -> Res),
        ("union", union as fn(Vec<Val>) -> Res),
        ("retain", retain as fn(Vec<Val>) -> Res),
        ("negate", negate as fn(Vec<Val>) -> Res),
        ("rand-choice", rand_choice as fn(Vec<Val>) -> Res),
        ("str", str as fn(Vec<Val>) -> Res),
        ("say", say as fn(Vec<Val>) -> Res),
        ("ask", ask as fn(Vec<Val>) -> Res),
        ("placeholder-fn", placeholder_fn as fn(Vec<Val>) -> Res),
        ("gensym", gensym2 as fn(Vec<Val>) -> Res),
        ("op-dot", op_dot as fn(Vec<Val>) -> Res),
    ].iter().map(|(name, f)| ((*name).into(), Val::Fn(wrap_list_arg(f)))).collect();
    res.extend([
        ("op-op", op_op as fn(Ref) -> Res),
        ("op-call-coll", op_call_coll as fn(Ref) -> Res),
        ("bind-op-list", bind_op_list as fn(Ref) -> Res),
        ("range", range as fn(Ref) -> Res),
        ("first-index", first_index as fn(Ref) -> Res),
        ("start-index", start_index as fn(Ref) -> Res),
        ("last-index", last_index as fn(Ref) -> Res),
        ("unsparse", unsparse as fn(Ref) -> Res),
        ("bake", bake as fn(Ref) -> Res),
        ("bake-some", bake_some as fn(Ref) -> Res),
        ("reduce", reduce as fn(Ref) -> Res),
        ("split", split as fn(Ref) -> Res),
        ("read-file", read_file as fn(Ref) -> Res),
    ].iter().map(|(name, f)| (Val::from(*name), Val::Fn(wrapf(f)))));
    res
}
