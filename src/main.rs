use std::rc::Rc;
use std::sync::Arc;
use std::fs::read_to_string;
use std::env;

use im;
use im::hashmap;

use panic_context::panic_context;

mod sparsevec;

mod val;
use val::{Val, Ref, Res, AFn, refe, ok};

mod create;
mod parse;

mod builtins;
use builtins::{Env, call, op_op_env};

#[derive(Debug, Clone)]
enum Inst {
    Lit(Val),
    Deref(String),
    Bind(Box<Inst>, Box<Inst>),
    List(Vec<Inst>),
    Dict(Vec<Inst>),
    Call(Box<Inst>, Box<Inst>),
    AsResult(Box<Inst>),
    If(Box<Inst>, Box<Inst>, Box<Inst>),
    Fn(String, Vec<Inst>, Box<Inst>),
    Module(Vec<Val>),
    Quasiquote(Val)
}

fn macro_expand1(form: Val, env: &Env) -> Val {
    if let Some(op) = form.get("op") {
        let op_str: String = format!("op-{}", String::try_from(op).unwrap());
        if let Some(mac) = env.get(&refe(op_str)) {
           (*call(&mac, Arc::new(form.get("args").unwrap().clone())).unwrap()).clone()
        } else {form}
    } else {form}
}

fn macro_expand(form: Val, env: &Env) -> Val {
    if let Some(op) = form.get("op") {
        let op_str: String = format!("op-{}", String::try_from(op).unwrap());
        if op_str == "op-module" || op_str == "op-qq" {
            // don't recurse into modules and qq here, module and qqinstruction does macro expansion itself.
            // TODO check if we can do this withouth this special case
            form
        } else if op_str == "op-fn" {
            let (bind, ops) = dest_macro_expand(form.get("args").unwrap().get(0).unwrap().clone(), env);
            let args = Vec::try_from(form.get("args").unwrap().clone()).unwrap();
            let mut args2 = vec![create::deref(bind)];
            args2.extend(ops);
            args2.extend(args[1..].iter().map(|arg: &Val| macro_expand(arg.clone(), env)));
            let mut form2 = form.clone();
            form2.insert("args", Val::from(args2));
            form2
        } else if let Some(mac) = env.get(&refe(op_str)) {
            macro_expand((*call(&mac, Arc::new(form.get("args").unwrap().clone())).unwrap()).clone(), env)
        } else {
            if let Some(args) = form.get("args").map(|v| Vec::try_from(v.clone()).unwrap()) {
                let mut form2 = form.clone();
                form2.insert("args", args.iter().map(|arg: &Val| macro_expand(arg.clone(), env)).collect::<Vec<_>>());
                form2
            } else {form}
        }
    } else {form}
}

fn dest_macro_expand(dest: Val, env: &Env) -> (Val, Vec<Val>) {
    let dest = macro_expand1(dest, &op_op_env());
    let dest_op_str = format!("bind-op-{}", String::try_from(dest.get("op").unwrap()).unwrap());
    if dest_op_str == "bind-op-deref" {
        (dest.get("name").unwrap().clone(), vec![])
    } else if let Some(mac) = env.get(&refe(dest_op_str.clone())) {
        let foo = call(&mac, Arc::new(dest.get("args").unwrap().clone())).unwrap();
        (foo.get("bind").unwrap().clone(),
         Vec::try_from(foo.get("ops").unwrap().clone()).unwrap()
         .into_iter().flat_map(|form| bind_macro_expand(form, env)).collect())
    } else {
        panic!("{} not defined", dest_op_str);
    }
}

fn bind_macro_expand(form: Val, env: &Env) -> Vec<Val> {
    match form.get("op") {
        Some(Val::Str(op)) if op == "bind" => {
            let (bind, ops) = dest_macro_expand(form.get("args").unwrap()[0].clone(), env);
            let mut res = vec![
                create::inst("bind", vec![
                    create::deref(bind),
                    macro_expand(form.get("args").unwrap()[1].clone(), env)
                ])
            ];
            res.extend(ops);
            res
        }
        _ => vec![form]
    }
}

fn analyze_par(par: &Inst) -> (String, Vec<Inst>) {
    match par {
        Inst::Deref(par_name) => (par_name.to_string(), vec![]),
        _ => panic!("expected deref got {:?}", par)
    }
}

fn val_to_inst(y: &Val) -> Inst {
    // TODO performance y is always dbg formatted, not just when necessary
    let op: String = y.get("op").expect(&format!("expected an instruction instead got {}", y.repr())).clone().try_into().unwrap();
    let op2: &str = &op;
    match op2 {
        "lit" => {
            Inst::Lit(y.get("val").unwrap().clone())
        },
        "bind" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            panic_context!("bind: {}", y.repr());
            Inst::Bind(Box::new(match get_op(&args[0]).as_deref() {
                Some("deref") => Inst::Lit(
                    args[0].get("name").expect("deref op should have name").try_into().expect("name should be a string")
                ),
                _ => val_to_inst(&args[0])
            }),
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
        "as-result" => {
            let args: Vec<Val> = y.get("args").unwrap().clone().try_into().unwrap();
            Inst::AsResult(Box::new(val_to_inst(&args[0])))
        }
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
                panic!("{:?}", args);
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
            env.insert(eval(&binding_name, &env).unwrap(), eval(&inner_inst, &env).unwrap());
        } else {
            eval(&inst, &env).unwrap();
        }
    }
}

fn eval_val(vinst: &Val, env: &Env) -> Res {
    let inst = val_to_inst(&macro_expand(vinst.clone(), env));
    eval(&inst, &env)
}

fn eval_vals(vinsts: &Vec<Val>, env: &Env) -> Env {
    let mut env2 = env.clone();
    let mut result_env = im::HashMap::new();
    for vinst in vinsts {
        panic_context!("{}", vinst.repr());
        for vinst2 in bind_macro_expand(macro_expand(vinst.clone(), &env2), &env2) {
            let inst = val_to_inst(&vinst2);
            if let Inst::Bind(binding_name, inner_inst) = inst {
                let name = eval(&binding_name, &env2).unwrap();
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
            dict.insert((*eval(&binding_name, &env).unwrap()).clone(),
                        (*eval(&inner_inst, &env).unwrap()).clone());
        } else {
            panic!();
        }
    }
    dict
}

fn get_op(form: &Val) -> Option<String> {
    form.get("op").map(|op| String::try_from(op).unwrap())
}

fn quasiquote(form: &Val, env: &Env) -> Val {
    // TODO performance quasiquote goes over the entire form but only the parts which contain unqotes need to be visited
    // TODO cleaned_form is an ugly hack, need a better way to deal with this
    // perhaps we go
    // {op: "macro" find macro and call it
    // {op: "inst" macro-expand args and convert to final-inst
    // {op: "final-inst" leave as is
    // macros which do not want their result to be macro-expanded upon can go straight to final-inst
    let cleaned_form = macro_expand1(form.clone(), &op_op_env());
    match cleaned_form.get("op") {
        Some(Val::Str(op)) if op == "uq" => {
            // TODO performance: eval_val does macro expansion
            (*eval_val(&cleaned_form.get("args").unwrap()[0], env).unwrap()).clone()
        }
        _ => match form {
            Val::Coll(xs, d) => {
                Val::Coll(
                    sparsevec::SparseVec::from_entries(xs.entries().into_iter().map(|(i, x)| {
                        (i, quasiquote(&x, env))
                    }).collect()),
                    d.iter().map(|(k, v)| {
                        (Arc::new(quasiquote(k, env)), Arc::new(quasiquote(v, env)))
                    }).collect()
                )
            },
            _ => form.clone()
        }
    }
}

fn eval(inst: &Inst, env: &Env) -> Res {
    match inst {
        Inst::Lit(x) => ok(x.clone()),
        Inst::Deref(x) => env.get(&refe(x.clone())).cloned().ok_or_else(|| Arc::new(format!("no {} in env", x).into())),
        Inst::List(xs) => ok(xs.iter().map(|x| (*eval(x, env).unwrap()).clone()).collect::<Vec<_>>()),
        Inst::Dict(xs) => ok(eval_dict(xs, env)),
        Inst::Call(finst, arginst) => {
            call(&eval(finst, env).unwrap(),
                 eval(arginst, env).unwrap())
        },
        Inst::AsResult(cond_inst) => {
            match eval(cond_inst, env) {
                Ok(v) => ok(hashmap!{
                    Val::from("ok") => (*v).clone()
                }),
                Err(v) => ok(hashmap!{
                    Val::from("err") => (*v).clone()
                })
            }
        }
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
            ok(Val::Fn(AFn(Rc::new(move |arg: Ref| {
                let mut env2 = env.clone();
                env2.insert(refe(par_name.clone()), arg);
                eval_body(&body, &mut env2);
                eval(&tail, &env2)
            }))))
        },
        Inst::Module(body) => {
            ok(eval_vals(&body, &env))
        }
        Inst::Quasiquote(form) => ok(macro_expand(quasiquote(form, env), env)),
        Inst::Bind(_, _) => panic!()
    }
}

fn eval_str(code: &str, env: &Env) -> Res {
    eval_val(&parse::inst(code).unwrap().1, &env)
}

fn eval_body_str(code: &str, env: &Env) -> Env {
    let code2 = parse::module(code).unwrap().1 ;
    im::HashMap::try_from((*eval_val(&code2, &env).unwrap()).clone()).unwrap()
}

fn eval_file(path: &str, env: &Env) -> Env {
    eval_body_str(&read_to_string(path).unwrap(), env)
}

fn main() {
    let mut glob: Env = builtins::get();
    glob.extend(eval_file("prelude.br", &glob));
    assert_eq!(eval_str("{if (< 4 3) {do 0} (+ 90 9)}", &glob), ok(99));
    assert_eq!(eval_str("({fn [a b] (+ a b)} 1 8)", &glob), ok(9));
    assert_eq!(eval_str("({fn [a [[b1 b2] c]] (+ a b1 b2 c)} 1 [[8 5] 5])", &glob), ok(19));
    assert_eq!(eval_str("(fibonacci 6)", &glob), ok(8));
    assert_eq!(
        eval_str("\"this is a string inside of a string\"", &glob),
        ok("this is a string inside of a string")
    );
    assert_eq!(eval_str("({a: 4 b: 5} \"c\")", &glob), Err(Arc::new("c".into())));
    assert_eq!(
        eval_str("(merge {a: 4 b: 5} {a: 2 c: 3})", &glob),
        ok(im::HashMap::from(vec![("c", 3), ("b", 5), ("a", 2)]))
    );
    assert_eq!(
        eval_str("(++ [1 2 3] [4] [5 6])", &glob),
        ok(vec![1, 2, 3, 4, 5, 6])
    );
    assert_eq!(
        eval_str("(retain {a: 4 b: 5} {a: 1})", &glob),
        ok(im::HashMap::from(vec![("a", 4)]))
    );
    assert_eq!(
        eval_str("(retain {a: 4 b: 5} (negate {a: 1}))", &glob),
        ok(im::HashMap::from(vec![("b", 5)]))
    );
    assert_eq!(
        eval(&val_to_inst(&eval_str("{op: \"call\" args: [{op: \"deref\" name: \"inc\"}
{op: \"list\" args: [{op: \"lit\" val: 5}]}]}", &glob).unwrap()),
             &glob),
        ok(6)
    );
    assert_eq!(
        eval_str("(kv-map [4 4 4] +)", &glob),
        ok(vec![4, 5, 6])
    );
    assert_eq!(
        eval_str("(map [1 2 3] + 2)", &glob),
        ok(vec![3, 4, 5])
    );
    assert_eq!(
        eval_str("(zip + [1 2 3] [30 20 10])", &glob),
        ok(vec![31, 22, 13])
    );

    let args: Vec<String> = env::args().collect();
    if let [_, path] = args.as_slice() {
        eval_file(path, &glob);
    }
}
