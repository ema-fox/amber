use std::rc::Rc;
use std::fs::read_to_string;
use std::env;

use im;

mod val;
use val::{Val, AFn};

mod create;
mod parse;

mod builtins;
use builtins::{Env, YRes, call, gensym};

#[derive(Debug, Clone)]
enum Inst {
    Lit(Val),
    Deref(String),
    Bind(String, Box<Inst>),
    List(Vec<Inst>),
    Dict(Vec<Inst>),
    Call(Box<Inst>, Box<Inst>),
    If(Box<Inst>, Box<Inst>, Box<Inst>),
    Fn(String, Vec<Inst>, Box<Inst>),
    Module(Vec<Val>),
    Quasiquote(Val)
}

fn macro_expand(form: Val, env: &Env) -> Val {
    if let Some(op) = form.get("op") {
        let op_str: String = format!("op-{}", String::try_from(op).unwrap());
        if let Some(mac) = env.get(&op_str.into()) {
            macro_expand(call(&mac, form.get("args").unwrap().clone()).unwrap(), env)
        } else {
            if let Some(args) = form.get("args").map(|v| Vec::try_from(v.clone()).unwrap()) {
                let mut form2 = form.clone();
                form2.insert("args", args.iter().map(|arg: &Val| macro_expand(arg.clone(), env)).collect::<Vec<_>>());
                form2
            } else {form}
        }
    } else {form}
}

fn bind_macro_expand(form: Val, env: &Env) -> Vec<Val> {
    match form.get("op") {
        Some(Val::Str(op)) if op == "bind" => {
            let dest = &form.get("args").unwrap()[0];
            let dest_op_str = format!("bind-op-{}", String::try_from(dest.get("op").unwrap()).unwrap());
            if dest_op_str == "bind-op-deref" {
                vec![form]
            } else if let Some(mac) = env.get(&dest_op_str.clone().into()) {
                let foo = call(&mac, dest.get("args").unwrap().clone()).unwrap();
                let mut res = vec![
                    create::inst("bind", vec![
                        create::deref(foo.get("bind").unwrap().clone()),
                        form.get("args").unwrap()[1].clone()
                    ])
                ];
                // TODO recursively apply `bind_macro_expand`
                res.extend(Vec::try_from(foo.get("ops").unwrap().clone()).unwrap());
                res
            } else {
                panic!("{} not defined", dest_op_str);
            }
        }
        _ => vec![form]
    }
}

fn analyze_par(par: &Inst) -> (String, Vec<Inst>) {
    match par {
        Inst::Deref(par_name) => (par_name.to_string(), vec![]),
        Inst::List(xs) => {
            let par_name = gensym("list");
            let mut insts = vec![];
            for (i, x) in xs.iter().enumerate() {
                let (entry_name, mut entry_insts) = analyze_par(x);
                insts.push(Inst::Bind(entry_name.to_string(),
                                   Box::new(Inst::Call(Box::new(Inst::Deref(par_name.clone())),
                                                       Box::new(Inst::List(vec![Inst::Lit(Val::Int(i as i64))]))))));
                insts.append(&mut entry_insts);
            }
            (par_name, insts)
        }
        _ => todo!()
    }
}

fn val_to_inst(y: &Val) -> Inst {
    // TODO performance y is always dbg formatted, not just when necessary
    let op: String = y.get("op").expect(&format!("expected an instruction instead got {:?}", y)).clone().try_into().unwrap();
    let op2: &str = &op;
    match op2 {
        "lit" => {
            Inst::Lit(y.get("val").unwrap().clone())
        },
        "bind" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::Bind(args[0].get("name").expect("bind op should have name").try_into().expect("name should be a string"),
                       Box::new(val_to_inst(&args[1])))
        },
        "list" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::List(args.iter().map(val_to_inst).collect())
        },
        "dict" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::Dict(args.iter().map(val_to_inst).collect())
        },
        "call" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::Call(Box::new(val_to_inst(&args[0])), Box::new(val_to_inst(&args[1])))
        },
        "deref" => {
            Inst::Deref(y.get("name").unwrap().clone().try_into().unwrap())
        },
        "if" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::If(Box::new(val_to_inst(&args[0])),
                     Box::new(val_to_inst(&args[1])),
                     Box::new(val_to_inst(&args[2])))
        },
        "fn" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            if let [par, body @ .., tail] = args.iter().map(val_to_inst).collect::<Vec<_>>().as_slice() {
                let mut body_vec = vec![];
                let (par_name, mut destructuring_body) = analyze_par(par);
                body_vec.append(&mut destructuring_body);
                body_vec.append(&mut body.into());
                Inst::Fn(par_name.to_string(), body_vec, Box::new(tail.clone()))
            } else {
                panic!();
            }
        },
        "module" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::Module(args)
        },
        "qq" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::Quasiquote(args[0].clone())
        },
        _ => panic!("Unknown op: {}", op2)
    }
}

fn eval_body(insts: &Vec<Inst>, env: &mut Env) {
    for inst in insts {
        if let Inst::Bind(binding_name, inner_inst) = inst {
            env.insert(binding_name.clone().into(), eval(&inner_inst, &env).unwrap());
        } else {
            eval(&inst, &env).unwrap();
        }
    }
}

fn eval_val(vinst: &Val, env: &Env) -> YRes {
    let inst = val_to_inst(&macro_expand(vinst.clone(), env));
    eval(&inst, &env)
}

fn eval_vals(vinsts: &Vec<Val>, env: &Env) -> Env {
    let mut env2 = env.clone();
    let mut result_env = im::HashMap::new();
    for vinst in vinsts {
        for vinst2 in bind_macro_expand(macro_expand(vinst.clone(), &env2), &env2) {
            let inst = val_to_inst(&vinst2);
            if let Inst::Bind(binding_name, inner_inst) = inst {
                let name = Val::from(binding_name.clone());
                let result = eval(&inner_inst, &env2).unwrap();
                env2.insert(name.clone(), result.clone());
                result_env.insert(name, result);
            } else {
                eval(&inst, &env2).unwrap();
            }
        }
    }
    result_env
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

fn quasiquote(form: &Val, env: &Env) -> Val {
    // TODO performance quasiquote goes over the entire form but only the parts which contain unqotes need to be visited
    match form.get("op") {
        Some(Val::Str(op)) if op == "uq" => {
            // TODO performance: eval_val does macro expansion
            eval_val(&form.get("args").unwrap()[0], env).unwrap()
        }
        _ => match form {
            Val::Coll(xs, d) => {
                Val::Coll(xs.map_indexed(|_, x| quasiquote(&x, env)),
                          d.iter().map(|(k, v)| {
                              (quasiquote(k, env), quasiquote(v, env))
                          }).collect())
            },
            _ => form.clone()
        }
    }
}

fn eval(inst: &Inst, env: &Env) -> YRes {
    match inst {
        Inst::Lit(x) => Ok(x.clone()),
        Inst::Deref(x) => env.get(&x.clone().into()).cloned().ok_or(format!("no {} in env", x).into()),
        Inst::List(xs) => Ok(xs.iter().map(|x| eval(x, env).unwrap()).collect::<Vec<_>>().into()),
        Inst::Dict(xs) => Ok(Val::from(eval_dict(xs, env))),
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
                env2.insert(par_name.clone().into(), arg);
                eval_body(&body, &mut env2);
                eval(&tail, &env2)
            }))))
        },
        Inst::Module(body) => {
            Ok(Val::from(eval_vals(&body, &env)))
        }
        Inst::Quasiquote(form) => Ok(quasiquote(form, env)),
        Inst::Bind(_, _) => panic!()
    }
}

fn eval_str(code: &str, env: &Env) -> YRes {
    eval_val(&parse::inst(code).unwrap().1, &env)
}

fn eval_body_str(code: &str, env: &Env) -> Env {
    let code2 = parse::module(code).unwrap().1 ;
    im::HashMap::try_from(eval_val(&code2, &env).unwrap()).unwrap()
}

fn eval_file(path: &str, env: &Env) -> Env {
    eval_body_str(&read_to_string(path).unwrap(), env)
}

fn main() {
    let mut glob: Env = builtins::get();
    glob.extend(eval_file("prelude.br", &glob));
    assert_eq!(eval_str("{if (< 4 3) {do 0} (+ 90 9)}", &glob), Ok(99.into()));
    assert_eq!(eval_str("({fn [a b] (+ a b)} 1 8)", &glob), Ok(9.into()));
    assert_eq!(eval_str("({fn [a [[b1 b2] c]] (+ a b1 b2 c)} 1 [[8 5] 5])", &glob), Ok(19.into()));
    assert_eq!(eval_str("(fibonacci 6)", &glob), Ok(8.into()));
    assert_eq!(
        eval_str("\"this is a string inside of a string\"", &glob),
        Ok("this is a string inside of a string".into())
    );
    assert_eq!(eval_str("({dict a: 4 b: 5} \"c\")", &glob), Err("c".into()));
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
        eval_file(path, &glob);
    }
}
