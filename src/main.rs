use std::rc::Rc;
use std::collections::HashMap;

use nom::{IResult, Parser};
use nom::branch::{alt};
use nom::character::complete::{char, one_of, alpha1, alphanumeric1, digit1, multispace0};
use nom::combinator::{recognize, map};
use nom::multi::{many0_count, many0};
use nom::sequence::{delimited, preceded};

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
    Fn(String, Box<Inst>)
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
                if let Inst::Deref(par_name) = &args[0] {
                    Inst::Fn(par_name.to_string(), Box::new(args[1].clone()))
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
        Inst::Deref(x) => Ok(env.get(x).unwrap().clone()),
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
        Inst::Fn(par_name, body) => {
            let env = env.clone();
            let body = body.clone();
            let par_name = par_name.clone();
            Ok(Val::Fn(AFn(Rc::new(move |arg: Val| {
                let mut env2 = env.clone();
                env2.insert(par_name.to_string(), arg);
                eval(&body, &env2)
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
    // ({fn [a b] (+ a b)} 2 3)
    // ({fn args a: (nth args 0) b: (nth args 1) (+ a b)} 2 3)
    // ({fn args (+ (nth args 0) (nth args 1))} 2 3)

    let glob: HashMap<String, Val> = [
        ("<".to_string(), Val::Fn(AFn(Rc::new(lt)))),
        ("+".to_string(), Val::Fn(AFn(Rc::new(plus)))),
        ("nth".to_string(), Val::Fn(AFn(Rc::new(nth))))
    ].into();
    //dbg!(eval(&pinst("{if (< 4 3) 0 (+ 90 9)}").unwrap().1, &glob));
    dbg!(eval(&pinst("({fn arg (+ (nth arg 0) (nth arg 1))} 1 2)").unwrap().1, &glob));
    dbg!(pinst("{fn arg foo: (nth arg 0) foo}"));
}
