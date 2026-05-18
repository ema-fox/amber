use std::rc::Rc;

use nom::{IResult, Parser};
use nom::branch::{alt};
use nom::character::complete::{char, alpha1, alphanumeric1, digit1, multispace0};
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
    List(Vec<Inst>),
    Call(Box<Inst>, Box<Inst>),
    If(Box<Inst>, Box<Inst>, Box<Inst>)
}

fn psym(inp: &str) -> IResult<&str, &str> {
    recognize((alpha1, many0_count(alphanumeric1))).parse(inp)
}

fn pnum(inp: &str) -> IResult<&str, &str> {
    digit1.parse(inp)
}

fn plit(inp: &str) -> IResult<&str, Inst> {
    map(pnum, |v: &str| Inst::Lit(Val::Int(i64::from_str_radix(v, 10).unwrap()))).parse(inp)
}

fn pbraceinst(inp: &str) -> IResult<&str, Inst> {
    map(delimited(char('{'), (psym, pinsts), char('}')),
        |(op, args): (&str, Vec<Inst>)| match op {
            "if" => Inst::If(Box::new(args[0].clone()),
                             Box::new(args[1].clone()),
                             Box::new(args[2].clone())),
            _ => todo!()
        }).parse(inp)
}

fn pinst(inp: &str) -> IResult<&str, Inst> {
    alt((plit, pbraceinst)).parse(inp)
}

fn pinsts(inp: &str) -> IResult<&str, Vec<Inst>> {
    many0(preceded(multispace0, pinst)).parse(inp)
}

fn eval(inst: &Inst) -> Result<Val, Val> {
    match inst {
        Inst::Lit(x) => Ok(x.clone()),
        Inst::List(xs) => Ok(Val::List(xs.iter().map(|x| eval(x).unwrap()).collect())),
        Inst::Call(finst, arginst) => {
            let f = eval(finst).unwrap();
            let arg = eval(arginst).unwrap();
            match f {
                Val::Fn(f2) => {
                    f2.0(arg)
                },
                _ => panic!()
            }
        },
        Inst::If(cond_inst, then_inst, else_inst) => {
            match eval(cond_inst) {
                Ok(_) => eval(then_inst),
                Err(_) => eval(else_inst)
            }
        }
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

fn main() {
    // {if (< 2 3) 0 99}
    let code = Inst::If(Box::new(Inst::Call(Box::new(Inst::Lit(Val::Fn(AFn(Rc::new(lt))))),
                                            Box::new(Inst::List(vec![Inst::Lit(Val::Int(2)), Inst::Lit(Val::Int(3))])))),
                        Box::new(Inst::Lit(Val::Int(0))),
                        Box::new(Inst::Lit(Val::Int(99))));
    dbg!(eval(&code));

    dbg!(pinst("{if 2 3 4}"));
    //dbg!(pnum("123"));
}
