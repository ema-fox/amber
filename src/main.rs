struct AFn(Box<dyn Fn (Val) -> Result<Val, Val>>);

impl std::fmt::Debug for AFn {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("fn")
    }
}

#[derive(Debug)]
enum Val {
    Int(i64),
    List(Vec<Val>),
    Fn(AFn)
}

#[derive(Debug)]
enum Inst {
    Int(i64),
    List(Vec<Inst>),
    Call(Box<Inst>, Box<Inst>)
}

fn eval(inst: &Inst) -> Result<Val, Val> {
    match inst {
        Inst::Int(x) => Ok(Val::Int(*x)),
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
            })).sum()
        },
        _ => panic!()
    }
}


fn main() {
    let code = Inst::List(vec![Inst::Int(2), Inst::Int(3)]);
    dbg!(eval(&code));
}
