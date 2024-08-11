use std::collections::HashMap;

fn main() {
    for line in std::io::stdin().lines().flatten() {
        parse(&line);
    }
}

fn eval<'src>(code: Value<'src>, vm: &mut Vm<'src>) {
    match code {
        Value::Op(op) => match op {
            "+" => add(&mut vm.stack),
            "-" => sub(&mut vm.stack),
            "*" => mul(&mut vm.stack),
            "/" => div(&mut vm.stack),
            "<" => lt(&mut vm.stack),
            "if" => op_if(vm),
            "def" => op_def(vm),
            _ => {
                let val = vm
                    .vars
                    .get(op)
                    .expect(&format!("{op:?} is not a define operation"));
                vm.stack.push(val.clone());
                // vm.stack.push(*val); でもよい？
            }
        },
        _ => vm.stack.push(code.clone()),
    }
}

fn parse<'a>(line: &'a str) -> Vec<Value> {
    let mut vm = Vm::new();
    let input: Vec<_> = line.split(" ").collect();
    let mut words = &input[..];
    while let Some((&word, mut rest)) = words.split_first() {
        if word.is_empty() {
            break;
        }
        if word == "{" {
            let value;
            (value, rest) = parse_block(rest);
            vm.stack.push(value);
        } else {
            let code = if let Ok(num) = word.parse::<i32>() {
                Value::Num(num)
            } else if word.starts_with("/") {
                Value::Sym(&word[1..]) // 先頭の`/`を除外する
            } else {
                Value::Op(word)
            };
            eval(code, &mut vm);
        }

        words = rest;
    }
    println!("stack: {:?}", vm.stack);
    vm.stack
}

fn parse_block<'src, 'a>(input: &'a [&'src str]) -> (Value<'src>, &'a [&'src str]) {
    let mut tokens = vec![];
    let mut words = input;

    while let Some((&word, mut rest)) = words.split_first() {
        if word.is_empty() {
            break;
        }
        if word == "{" {
            let value;
            (value, rest) = parse_block(rest);
            tokens.push(value);
        } else if word == "}" {
            return (Value::Block(tokens), rest);
        } else if let Ok(value) = word.parse::<i32>() {
            tokens.push(Value::Num(value))
        } else {
            tokens.push(Value::Op(word));
        }

        words = rest;
    }

    (Value::Block(tokens), words)
}

macro_rules! impl_op {
    {$name:ident, $op:tt} => {
        fn $name(stack: &mut Vec<Value>) {
            let rhs = stack.pop().unwrap().as_num();
            let lhs = stack.pop().unwrap().as_num();
            stack.push(Value::Num((lhs $op rhs) as i32));
        }
    }
}

impl_op!(add, +);
impl_op!(sub, -);
impl_op!(mul, *);
impl_op!(div, /);
impl_op!(lt, <);

fn op_if<'src>(vm: &mut Vm<'src>) {
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
    let sym = vm.stack.pop().unwrap().as_sym();

    vm.vars.insert(sym, value);
}

// stackに加えvarsという状態を扱いたいが、引数に複数の状態を持たせるのが煩雑　→　Vmという状態にまとめる
struct Vm<'src> {
    stack: Vec<Value<'src>>,
    vars: HashMap<&'src str, Value<'src>>,
}

impl<'src> Vm<'src> {
    fn new() -> Self {
        Self {
            stack: vec![],
            vars: HashMap::new(),
        }
    }
}
#[derive(Debug, PartialEq, Eq, Clone)]
enum Value<'src> {
    Num(i32),
    Op(&'src str),
    Sym(&'src str),
    Block(Vec<Value<'src>>),
}

impl<'src> Value<'src> {
    fn as_num(&self) -> i32 {
        match self {
            Self::Num(val) => *val,
            _ => panic!("Value is not a number"),
        }
    }

    fn as_sym(&self) -> &'src str {
        if let Self::Sym(sym) = self {
            *sym
        } else {
            panic!("Value is not a symbol");
        }
    }

    fn to_block(self) -> Vec<Value<'src>> {
        match self {
            Self::Block(val) => val,
            _ => panic!("Value is not a block"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::{parse, Value::*};
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
}
