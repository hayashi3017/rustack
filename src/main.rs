use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
};

fn main() {
    if let Some(f) = std::env::args()
        .nth(1)
        .and_then(|f| std::fs::File::open(f).ok())
    {
        parse_batch(BufReader::new(f));
    } else {
        parse_interactive();
    }
}

fn eval(code: Value, vm: &mut Vm) {
    if let Some(top_block) = vm.blocks.last_mut() {
        top_block.push(code);
        return;
    }
    if let Value::Op(ref op) = code {
        let val = vm
            .find_var(op)
            .expect(&format!("{op:?} is not a defined operation"))
            .clone();
        match val {
            Value::Block(block) => {
                vm.vars.push(HashMap::new());
                for code in block {
                    eval(code, vm);
                }
                vm.vars.pop();
            }
            Value::Native(op) => (op.0)(vm),
            _ => vm.stack.push(val),
        }
    } else {
        vm.stack.push(code.clone());
    }
}

fn parse_batch(source: impl BufRead) -> Vec<Value> {
    let mut vm = Vm::new();
    for line in source.lines().flatten() {
        for word in line.split(" ") {
            parse_word(word, &mut vm);
        }
    }
    vm.stack
}

fn parse_interactive() {
    let mut vm = Vm::new();
    for line in std::io::stdin().lines().flatten() {
        for word in line.split(" ") {
            parse_word(word, &mut vm);
        }
        println!("stack: {:?}", vm.stack);
    }
}

fn parse_word<'a>(word: &'a str, vm: &mut Vm) {
    if word.is_empty() {
        return;
    }
    if word == "{" {
        vm.blocks.push(vec![]);
    } else if word == "}" {
        let top_block = vm.blocks.pop().expect("Block stack underrun!");
        eval(Value::Block(top_block), vm);
    } else {
        let code = if let Ok(num) = word.parse::<i32>() {
            Value::Num(num)
        } else if word.starts_with("/") {
            Value::Sym(word[1..].to_string()) // 先頭の`/`を除外する
        } else {
            Value::Op(word.to_string())
        };
        eval(code, vm);
    }
}

macro_rules! impl_op {
    {$name:ident, $op:tt} => {
        fn $name(vm: &mut Vm) {
            let rhs = vm.stack.pop().unwrap().as_num();
            let lhs = vm.stack.pop().unwrap().as_num();
            vm.stack.push(Value::Num((lhs $op rhs) as i32));
        }
    }
}

impl_op!(add, +);
impl_op!(sub, -);
impl_op!(mul, *);
impl_op!(div, /);
impl_op!(lt, <);

fn op_if(vm: &mut Vm) {
    let false_branch = vm.stack.pop().unwrap().to_block();
    let true_branch = vm.stack.pop().unwrap().to_block();
    let cond = vm.stack.pop().unwrap().to_block();

    for code in cond {
        eval(code, vm);
    }

    let cond_result = vm.stack.pop().unwrap().as_num();

    if cond_result != 0 {
        for code in true_branch {
            eval(code, vm);
        }
    } else {
        for code in false_branch {
            eval(code, vm);
        }
    }
}

fn op_def(vm: &mut Vm) {
    let value = vm.stack.pop().unwrap();
    eval(value, vm);
    let value = vm.stack.pop().unwrap();
    let sym = vm.stack.pop().unwrap().as_sym().to_string();

    vm.vars.last_mut().unwrap().insert(sym, value);
}

fn puts(vm: &mut Vm) {
    let value = vm.stack.pop().unwrap();
    println!("{}", value.to_string());
}

fn dp(vm: &mut Vm) {
    let value = vm.stack.last().unwrap();
    vm.stack.push(value.clone());
}

fn pop(vm: &mut Vm) {
    vm.stack.pop().unwrap();
}

fn dup(vm: &mut Vm) {
    let value = vm.stack.last().unwrap();
    vm.stack.push(value.clone());
}

fn exch(vm: &mut Vm) {
    let last = vm.stack.pop().unwrap();
    let second = vm.stack.pop().unwrap();
    vm.stack.push(last);
    vm.stack.push(second);
}

fn index(vm: &mut Vm) {
    let index = vm.stack.pop().unwrap().as_num() as usize;
    let value = vm.stack[vm.stack.len() - index - 1].clone();
    vm.stack.push(value);
}

// stackに加えvarsという状態を扱いたいが、引数に複数の状態を持たせるのが煩雑　→　Vmという状態にまとめる
struct Vm {
    stack: Vec<Value>,
    vars: Vec<HashMap<String, Value>>,
    blocks: Vec<Vec<Value>>,
}

impl Vm {
    fn new() -> Self {
        let functions: [(&str, fn(&mut Vm)); 12] = [
            ("+", add),
            ("-", sub),
            ("*", mul),
            ("/", div),
            ("<", lt),
            ("if", op_if),
            ("def", op_def),
            ("puts", puts),
            ("pop", pop),
            ("dup", dup),
            ("exch", exch),
            ("index", index),
        ];
        Self {
            stack: vec![],
            vars: vec![functions
                .into_iter()
                .map(|(name, fun)| (name.to_owned(), Value::Native(NativeOp(fun))))
                .collect()],
            blocks: vec![],
        }
    }

    // スタックの最上位から順に検索し最初に合致した変数を返す
    fn find_var(&self, name: &str) -> Option<Value> {
        self.vars
            .iter()
            .rev()
            .find_map(|vars| vars.get(name).map(|var| var.to_owned()))
    }
}
#[derive(Debug, PartialEq, Eq, Clone)]
enum Value {
    Num(i32),
    Op(String),
    Sym(String),
    Block(Vec<Value>),
    Native(NativeOp),
}

impl Value {
    fn as_num(&self) -> i32 {
        match self {
            Self::Num(val) => *val,
            _ => panic!("Value is not a number"),
        }
    }

    fn as_sym(&self) -> &str {
        if let Self::Sym(sym) = self {
            sym
        } else {
            panic!("Value is not a symbol");
        }
    }

    fn to_block(self) -> Vec<Value> {
        match self {
            Self::Block(val) => val,
            _ => panic!("Value is not a block"),
        }
    }

    fn to_string(&self) -> String {
        match self {
            Self::Num(i) => i.to_string(),
            // refをつける理由は
            Self::Op(ref s) | Self::Sym(ref s) => s.clone(),
            Self::Block(_) => "<Block>".to_string(),
            Self::Native(_) => "<Native>".to_string(),
        }
    }
}

#[derive(Clone)]
struct NativeOp(fn(&mut Vm));

impl PartialEq for NativeOp {
    fn eq(&self, other: &Self) -> bool {
        //　関数ポインタの値を比較する
        self.0 as *const fn() == other.0 as *const fn()
    }
}

impl Eq for NativeOp {}

impl std::fmt::Debug for NativeOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<NativeOp>")
    }
}

#[cfg(test)]
mod test {
    use super::{Value::*, *};
    use std::io::Cursor;

    fn parse(input: &str) -> Vec<Value> {
        // 文字列リテラルをCursorでラップしてBufReadトレイトを実装
        parse_batch(Cursor::new(input))
    }

    #[test]
    fn test_group() {
        assert_eq!(
            parse("1 2 + { 3 4 }"),
            vec![Num(3), Block(vec![Num(3), Num(4)])]
        )
    }

    #[test]
    fn test_if_false() {
        assert_eq!(parse("{ 1 -1 + } { 100 } { -100 } if"), vec![Num(-100)])
    }

    #[test]
    fn test_if_true() {
        assert_eq!(parse("{ 1 1 + } { 100 } { -100 } if"), vec![Num(100)])
    }

    #[test]
    fn test_var_unit() {
        assert_eq!(parse("/x 10 def x 1 +"), vec![Num(11)])
    }

    #[test]
    fn test_var() {
        assert_eq!(parse("/x 10 def /y 20 def x y *"), vec![Num(200)])
    }

    #[test]
    fn test_var_if() {
        assert_eq!(
            parse("/x 10 def /y 20 def { x y < } { x } { y } if"),
            vec![Num(10)]
        );
    }

    #[test]
    fn test_multiline() {
        assert_eq!(
            parse(
                r#"
/x 10 def
/y 20 def

{ x y < }
{ x }
{ y }
if
"#
            ),
            vec![Num(10)]
        )
    }

    #[test]
    fn test_function() {
        assert_eq!(
            parse(
                r#"
/double { 2 * } def
10 double"#
            ),
            vec![Num(20)]
        )
    }
}
