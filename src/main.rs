use std::rc::Rc;
use std::cell::OnceCell;
use std::fs::read_to_string;
use std::env;

use im;

mod val;
use val::{Val, AFn};

mod parse;

mod builtins;
use builtins::{Env, YRes, call};

static mut COUNTER: usize = 0;

fn get_uniq_number() -> usize {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
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

fn macro_expand(form: Val, env: &Env) -> Val {
    if let Some(op) = form.try_get("op") {
        let op_str: String = format!("op-{}", String::try_from(op).unwrap());
        if let Some(mac) = deref(env, &op_str) {
            call(&mac, form.try_get("args").unwrap().clone()).unwrap()
        } else {
            if let Some(Val::List(args)) = form.try_get("args") {
                let mut form2 = form.clone();
                form2.insert("args", args.iter().map(|arg| macro_expand(arg.clone(), env)).collect::<Vec<_>>());
                form2
            } else {form}
        }
    } else {form}
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

fn val_to_inst(x: &Val) -> Inst {
    match x {
        Val::Dict(y) => {
            let op: String = y.get(&"op".into()).unwrap().clone().try_into().unwrap();
            let op2: &str = &op;
            match op2 {
                "lit" => {
                    Inst::Lit(y.get(&"val".into()).unwrap().clone())
                },
                "bind" => {
                    let args: Vec<Val> = y.get(&"args".into()).unwrap().clone().try_into().unwrap();
                    Inst::Bind(args[0].get("name").try_into().unwrap(),
                               Box::new(val_to_inst(&args[1])))
                },
                "list" => {
                    let args: Vec<Val> = y.get(&"args".into()).unwrap().clone().try_into().unwrap();
                    Inst::List(args.iter().map(val_to_inst).collect())
                },
                "dict" => {
                    let args: Vec<Val> = y.get(&"args".into()).unwrap().clone().try_into().unwrap();
                    Inst::Dict(args.iter().map(val_to_inst).collect())
                },
                "call" => {
                    let args: Vec<Val> = y.get(&"args".into()).unwrap().clone().try_into().unwrap();
                    Inst::Call(Box::new(val_to_inst(&args[0])), Box::new(val_to_inst(&args[1])))
                },
                "deref" => {
                    Inst::Deref(y.get(&"name".into()).unwrap().clone().try_into().unwrap())
                },
                "if" => {
                    let args: Vec<Val> = y.get(&"args".into()).unwrap().clone().try_into().unwrap();
                    Inst::If(Box::new(val_to_inst(&args[0])),
                             Box::new(val_to_inst(&args[1])),
                             Box::new(val_to_inst(&args[2])))
                },
                "fn" => {
                    let args: Vec<Val> = y.get(&"args".into()).unwrap().clone().try_into().unwrap();
                    if let [par, body @ .., tail] = args.iter().map(val_to_inst).collect::<Vec<_>>().as_slice() {
                        let mut body_vec = vec![];
                        let (par_name, mut destructuring_body) = analyze_par(par);
                        body_vec.append(&mut destructuring_body);
                        body_vec.append(&mut body.into());
                        Inst::Fn(par_name.to_string(), body_vec, Box::new(tail.clone()))
                    } else {
                        panic!();
                    }
                }
                _ => panic!("Unknown op: {}", op2)
            }
        },
        _ => panic!()
    }
}

fn eval_body(insts: &Vec<Inst>, env: &mut Env) {
    for inst in insts {
        if let Inst::Bind(binding_name, inner_inst) = inst {
            env.insert(binding_name.to_string(), Rc::new(OnceCell::new()));
            env.get(binding_name).unwrap().set(eval(&inner_inst, &env).unwrap()).unwrap();
        } else {
            eval(&inst, &env).unwrap();
        }
    }
}

fn eval_val(vinst: &Val, env: &Env) -> YRes {
    let inst = val_to_inst(&macro_expand(vinst.clone(), env));
    eval(&inst, &env)
}

fn eval_vals(vinsts: &Vec<Val>, env: &mut Env) {
    for vinst in vinsts {
        let inst = val_to_inst(&macro_expand(vinst.clone(), env));
        if let Inst::Bind(binding_name, inner_inst) = inst {
            env.insert(binding_name.to_string(), Rc::new(OnceCell::new()));
            env.get(&binding_name).unwrap().set(eval(&inner_inst, &env).unwrap()).unwrap();
        } else {
            eval(&inst, &env).unwrap();
        }
    }
}

fn deref(env: &Env, x: &str) -> Option<Val> {
        env.get(x).map(|y| {
            y.get().expect(&format!("{} not initialized", x)).clone()
        })
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

fn eval(inst: &Inst, env: &Env) -> YRes {
    match inst {
        Inst::Lit(x) => Ok(x.clone()),
        Inst::Deref(x) => Ok(deref(env, x).expect(&format!("no {} in env", x))),
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

fn eval_str(code: &str, env: &Env) -> YRes {
    eval_val(&parse::inst(code).unwrap().1, &env)
}

fn eval_body_str(code: &str, env: &mut Env) {
    eval_vals(&parse::insts(code).unwrap().1, env);
}

fn eval_file(path: &str, env: &mut Env) {
    eval_body_str(&read_to_string(path).unwrap(), env);
}

fn main() {
    let mut glob: Env = builtins::get();
    eval_file("prelude.br", &mut glob);
    assert_eq!(eval_str("{if (< 4 3) {do 0} (+ 90 9)}", &glob), Ok(99.into()));
    assert_eq!(eval_str("({fn [a b] (+ a b)} 1 8)", &glob), Ok(9.into()));
    assert_eq!(eval_str("({fn [a [[b1 b2] c]] (+ a b1 b2 c)} 1 [[8 5] 5])", &glob), Ok(19.into()));
    assert_eq!(eval_str("(fibonacci 6)", &glob), Ok(8.into()));
    assert_eq!(
        eval_str("\"this is a string inside of a string\"", &glob),
        Ok("this is a string inside of a string".into())
    );
    assert_eq!(eval_str("(get {dict a: 4 b: 5} \"c\")", &glob), Err("c".into()));
    assert_eq!(
        eval_str("(merge {dict a: 4 b: 5} {dict a: 2 c: 3})", &glob),
        Ok(im::HashMap::from(vec![("c", 3), ("b", 5), ("a", 2)]).into())
    );
    assert_eq!(
        eval_str("(++ [1 2 3] [4] [5 6])", &glob),
        Ok(vec![1, 2, 3, 4, 5, 6].into())
    );
    assert_eq!(
        eval_str("(retain {dict a: 4 b: 5} {dict a: 1})", &glob),
        Ok(im::HashMap::from(vec![("a", 4)]).into())
    );
    assert_eq!(
        eval_str("(retain {dict a: 4 b: 5} (negate {dict a: 1}))", &glob),
        Ok(im::HashMap::from(vec![("b", 5)]).into())
    );
    assert_eq!(
        eval(&val_to_inst(&eval_str("{dict op: \"call\" args: [{dict op: \"deref\" name: \"inc\"}
{dict op: \"list\" args: [{dict op: \"lit\" val: 5}]}]}", &glob).unwrap()),
             &glob),
        Ok(6.into())
    );

    let args: Vec<String> = env::args().collect();
    if let [_, path] = args.as_slice() {
        eval_file(path, &mut glob);
    }
}
