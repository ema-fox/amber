use std::sync::Arc;
use std::env;

use im;

use libamber::{eval, eval_file, eval_str, val_to_inst};
use libamber::builtins;
use libamber::builtins::Env;
use libamber::val::ok;

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
