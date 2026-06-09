use std::rc::Rc;
use std::collections::HashMap;
use std::cell::OnceCell;

use im;

use nom::{IResult, Parser};
use nom::branch::{alt};
use nom::character::complete::{char, one_of, alpha1, alphanumeric1, digit1, multispace0};
use nom::combinator::{recognize, map};
use nom::multi::{many0_count, many0};
use nom::sequence::{delimited, preceded};
use nom::bytes::{take_till};

static mut COUNTER: usize = 0;

fn get_uniq_number() -> usize {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

#[derive(Clone)]
struct AFn(Rc<dyn Fn (Val) -> Result<Val, Val>>);

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
enum Val {
    Int(i64),
    Str(String),
    List(Vec<Val>),
    Dict(im::HashMap<Val, Val>),
    Fn(AFn)
}

#[derive(Debug, Clone)]
enum Inst {
    Lit(Val),
    Deref(String),
    Bind(String, Box<Inst>),
    List(Vec<Inst>),
    Dict(Vec<Inst>),
    Call(Box<Inst>, Box<Inst>),
    If(Box<Inst>, Box<Inst>, Box<Inst>),
    Fn(String, Vec<Inst>, Box<Inst>)
}

type Env = HashMap<String, Rc<OnceCell<Val>>>;
type YRes = Result<Val, Val>;

fn sym_char(inp: &str) -> IResult<&str, &str> {
    recognize(one_of("<+-")).parse(inp)
}

fn psym(inp: &str) -> IResult<&str, &str> {
    recognize((alt((alpha1, sym_char)), many0_count(alt((alphanumeric1, sym_char))))).parse(inp)
}

fn pnum(inp: &str) -> IResult<&str, &str> {
    digit1.parse(inp)
}

fn plit(inp: &str) -> IResult<&str, Inst> {
    map(pnum, |v: &str| Inst::Lit(Val::Int(i64::from_str_radix(v, 10).unwrap()))).parse(inp)
}

fn pstrlit(inp: &str) -> IResult<&str, Inst> {
    map(delimited(char('"'),  take_till(|c| c == '"'), char('"')),
        |v: &str| Inst::Lit(Val::Str(v.to_string()))
    ).parse(inp)
}

fn pderef(inp: &str) -> IResult<&str, Inst> {
    map(psym, |v: &str| Inst::Deref(v.to_string())).parse(inp)
}

fn pbind(inp: &str) -> IResult<&str, Inst> {
    map((psym, (char(':'), multispace0), pinst),
        |(name, _, body)| Inst::Bind(name.to_string(), Box::new(body))).parse(inp)
}

fn analyze_par(par: &Inst) -> (String, Vec<Inst>) {
    match par {
        Inst::Deref(par_name) => (par_name.to_string(), vec![]),
        Inst::List(xs) => {
            let par_name = format!("list{}", get_uniq_number());
            let mut insts = vec![];
            for (i, x) in xs.iter().enumerate() {
                let (entry_name, mut entry_insts) = analyze_par(x);
                insts.push(Inst::Bind(entry_name.to_string(),
                                   Box::new(Inst::Call(Box::new(Inst::Deref("nth".to_string())),
                                                       Box::new(Inst::List(vec![Inst::Deref(par_name.clone()),
                                                                                Inst::Lit(Val::Int(i as i64))]))))));
                insts.append(&mut entry_insts);
            }
            (par_name, insts)
        }
        _ => todo!()
    }
}

fn pbraceinst(inp: &str) -> IResult<&str, Inst> {
    map(delimited(char('{'), (psym, pinsts), char('}')),
        |(op, args): (&str, Vec<Inst>)| match op {
            "list" => Inst::List(args),
            "dict" => Inst::Dict(args),
            "call" => Inst::Call(Box::new(args[0].clone()),
                                 Box::new(args[1].clone())),
            "if" => Inst::If(Box::new(args[0].clone()),
                             Box::new(args[1].clone()),
                             Box::new(args[2].clone())),
            "fn" => {
                if let [par, body @ .., tail] = args.as_slice() {
                    let mut body_vec = vec![];
                    let (par_name, mut destructuring_body) = analyze_par(par);
                    body_vec.append(&mut destructuring_body);
                    body_vec.append(&mut body.into());
                    Inst::Fn(par_name.to_string(), body_vec, Box::new(tail.clone()))
                } else {
                    panic!();
                }
            },
            _ => todo!()
        }).parse(inp)
}

fn plistinst(inp: &str) -> IResult<&str, Inst> {
    map(delimited(char('['), pinsts, char(']')),
        |entries: Vec<Inst>| Inst::List(entries)
    ).parse(inp)
}

fn pcallinst(inp: &str) -> IResult<&str, Inst> {
    map(delimited(char('('), (pinst, pinsts), char(')')),
        |(f, args): (Inst, Vec<Inst>)| Inst::Call(Box::new(f), Box::new(Inst::List(args)))
    ).parse(inp)
}

fn pinst(inp: &str) -> IResult<&str, Inst> {
    alt((pbind, plit, pstrlit, pderef, pbraceinst, plistinst, pcallinst)).parse(inp)
}

fn pinsts(inp: &str) -> IResult<&str, Vec<Inst>> {
    many0(preceded(multispace0, pinst)).parse(inp)
}

fn eval_body(insts: &Vec<Inst>, env: &mut Env) {
    for inst in insts {
        if let Inst::Bind(binding_name, inner_inst) = inst {
            env.insert(binding_name.to_string(), Rc::new(OnceCell::new()));
            env.get(binding_name).unwrap().set(eval(&inner_inst, &env).unwrap());
        } else {
            eval(&inst, &env);
        }
    }
}

fn eval_dict(insts: &Vec<Inst>, env: &Env) -> im::HashMap<Val, Val> {
    let mut dict = im::HashMap::new();
    for inst in insts {
        if let Inst::Bind(binding_name, inner_inst) = inst {
            dict.insert(Val::Str(binding_name.to_string()),
                        eval(&inner_inst, &env).unwrap());
        } else {
            panic!();
        }
    }
    dict
}

fn call(x: &Val, args: Val) -> YRes {
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

fn eval(inst: &Inst, env: &Env) -> Result<Val, Val> {
    match inst {
        Inst::Lit(x) => Ok(x.clone()),
        Inst::Deref(x) => Ok(env.get(x).expect(&format!("no {} in env", x)).get()
                             .expect(&format!("{} not initialized", x)).clone()),
        Inst::List(xs) => Ok(Val::List(xs.iter().map(|x| eval(x, env).unwrap()).collect())),
        Inst::Dict(xs) => Ok(Val::Dict(eval_dict(xs, env))),
        Inst::Call(finst, arginst) => {
            call(&eval(finst, env).unwrap(),
                 eval(arginst, env).unwrap())
        },
        Inst::If(cond_inst, then_inst, else_inst) => {
            match eval(cond_inst, env) {
                Ok(_) => eval(then_inst, env),
                Err(_) => eval(else_inst, env)
            }
        }
        Inst::Fn(par_name, body, tail) => {
            let env = env.clone();
            let body = body.clone();
            let tail = tail.clone();
            let par_name = par_name.clone();
            Ok(Val::Fn(AFn(Rc::new(move |arg: Val| {
                let mut env2 = env.clone();
                env2.insert(par_name.to_string(), Rc::new(arg.into()));
                eval_body(&body, &mut env2);
                eval(&tail, &env2)
            }))))
        },
        Inst::Bind(_, _) => panic!()
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

fn get(xs: Vec<Val>) -> Result<Val, Val> {
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

fn wrap_list_arg(f: &'static fn(Vec<Val>) -> YRes) -> AFn {
    AFn(Rc::new(|arg: Val| {
        match arg {
            Val::List(xs) => {
                f(xs)
            },
            _ => panic!()
        }
    }))
}

fn eval_str(code: &str, env: &Env) -> Result<Val, Val> {
    eval(&pinst(code).unwrap().1, &env)
}

fn eval_body_str(code: &str, env: &mut Env) {
    eval_body(&pinsts(code).unwrap().1, env);
}

fn main() {
    let mut glob: Env = [
        ("<", lt as fn(Vec<Val>) -> YRes),
        ("+", plus as fn(Vec<Val>) -> YRes),
        ("-", minus as fn(Vec<Val>) -> YRes),
        ("nth", nth as fn(Vec<Val>) -> YRes),
        ("++", concat as fn(Vec<Val>) -> YRes),
        ("get", get as fn(Vec<Val>) -> YRes),
        ("merge-with", merge_with as fn(Vec<Val>) -> YRes),
        ("retain", retain as fn(Vec<Val>) -> YRes),
        ("negate", negate as fn(Vec<Val>) -> YRes),
    ].iter().map(|(name, f)| (name.to_string(), Rc::new(OnceCell::from(Val::Fn(wrap_list_arg(f)))))).collect();
    eval_body_str("
inc: {fn [x] (+ x 1)}
merge: {fn [d0 d1] (merge-with {fn [a b] b} d0 d1)}
fibonacci: {fn [x] {if (< x 2) x (+ (fibonacci (- x 1)) (fibonacci (- x 2)))}}
", &mut glob);
    //dbg!(eval(&pinst("{if (< 4 3) 0 (+ 90 9)}").unwrap().1, &glob));
    //dbg!(eval_str("({fn [a b] (+ a b)} 1 8)", &glob));
    //dbg!(eval_str("({fn [a [[b1 b2] c]] (+ a b1 b2 c)} 1 [[8 5] 5])", &glob));
    dbg!(eval_str("(fibonacci 6)", &glob));
    dbg!(eval_str("\"this is a string inside of a string\"", &glob));
    dbg!(eval_str("(get {dict a: 4 b: 5} \"c\")", &glob));
    dbg!(eval_str("(merge {dict a: 4 b: 5} {dict a: 2 c: 3})", &glob));
    dbg!(eval_str("(++ [1 2 3] [4] [5 6])", &glob));
    dbg!(eval_str("(retain {dict a: 4 b: 5} {dict a: 1})", &glob));
    dbg!(eval_str("(retain {dict a: 4 b: 5} (negate {dict a: 1}))", &glob));
}
