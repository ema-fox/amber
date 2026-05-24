use std::rc::Rc;
use std::collections::HashMap;

use nom::{IResult, Parser};
use nom::branch::{alt};
use nom::character::complete::{char, one_of, alpha1, alphanumeric1, digit1, multispace0};
use nom::combinator::{recognize, map};
use nom::multi::{many0_count, many0};
use nom::sequence::{delimited, preceded};

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

#[derive(Debug, Clone)]
enum Val {
    Int(i64),
    List(Vec<Val>),
    Fn(AFn)
}

#[derive(Debug, Clone)]
enum Inst {
    Lit(Val),
    Deref(String),
    Bind(String, Box<Inst>),
    List(Vec<Inst>),
    Call(Box<Inst>, Box<Inst>),
    If(Box<Inst>, Box<Inst>, Box<Inst>),
    Fn(String, Vec<Inst>, Box<Inst>)
}

fn sym_char(inp: &str) -> IResult<&str, &str> {
    recognize(one_of("<+")).parse(inp)
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
    alt((pbind, plit, pderef, pbraceinst, plistinst, pcallinst)).parse(inp)
}

fn pinsts(inp: &str) -> IResult<&str, Vec<Inst>> {
    many0(preceded(multispace0, pinst)).parse(inp)
}

fn eval(inst: &Inst, env: &HashMap<String, Val>) -> Result<Val, Val> {
    match inst {
        Inst::Lit(x) => Ok(x.clone()),
        Inst::Deref(x) => Ok(env.get(x).expect(&format!("no {} in env", x)).clone()),
        Inst::List(xs) => Ok(Val::List(xs.iter().map(|x| eval(x, env).unwrap()).collect())),
        Inst::Call(finst, arginst) => {
            let f = eval(finst, env).unwrap();
            let arg = eval(arginst, env).unwrap();
            match f {
                Val::Fn(f2) => {
                    f2.0(arg)
                },
                _ => panic!()
            }
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
                env2.insert(par_name.to_string(), arg);
                for inner_inst in &body {
                    if let Inst::Bind(binding_name, inner_inner_inst) = inner_inst {
                        env2.insert(binding_name.to_string(), eval(&inner_inner_inst, &env2).unwrap());
                    } else {
                        eval(&inner_inst, &env2);
                    }
                }
                eval(&tail, &env2)
            }))))
        },
        Inst::Bind(_, _) => panic!()
    }
}

fn plus(arg: Val) -> Result<Val, Val> {
    match arg {
        Val::List(xs) => {
            Ok(Val::Int(xs.iter().map(|x| {
                match x {
                    Val::Int(n) => n,
                    _ => panic!()
                }
            }).sum()))
        },
        _ => panic!()
    }
}

fn lt(arg: Val) -> Result<Val, Val> {
    match arg {
        Val::List(xs) => {
            match (xs[0].clone(), xs[1].clone()) {
                (Val::Int(first), Val::Int(second)) => {
                    if first < second {
                        Ok(xs[0].clone())
                    } else {
                        Err(xs[0].clone())
                    }
                },
                _ => panic!()
            }
        },
        _ => panic!()
    }
}

fn nth(arg: Val) -> Result<Val, Val> {
    match arg {
        Val::List(xs) => {
            match (xs[0].clone(), xs[1].clone()) {
                (Val::List(ys), Val::Int(i)) => {
                    Ok(ys[i as usize].clone())
                },
                _ => panic!()
            }
        },
        _ => panic!()
    }
}

fn main() {
    let glob: HashMap<String, Val> = [
        ("<".to_string(), Val::Fn(AFn(Rc::new(lt)))),
        ("+".to_string(), Val::Fn(AFn(Rc::new(plus)))),
        ("nth".to_string(), Val::Fn(AFn(Rc::new(nth))))
    ].into();
    //dbg!(eval(&pinst("{if (< 4 3) 0 (+ 90 9)}").unwrap().1, &glob));
    dbg!(eval(&pinst("({fn [a b] (+ a b)} 1 8)").unwrap().1, &glob));
    dbg!(eval(&pinst("({fn [a [[b1 b2] c]] (+ a b1 b2 c)} 1 [[8 5] 5])").unwrap().1, &glob));
}
