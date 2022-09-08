use io::Write;
use std::{env, fs, io};

#[macro_use]
extern crate maplit;

mod ast;
mod environment;
mod error;
mod interp_error;
mod interpreter;
mod parser;
mod resolver;
mod scanner;
mod test_utils;
mod token;
mod value;

use interpreter::Interpreter;
use parser::Parser;
use resolver::Resolver;
use scanner::Scanner;

fn run(source: String, interpreter: &mut Interpreter) {
    let mut scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();
    println!("{:?}", tokens);
    let mut parser = Parser::new(tokens);

    if let Ok(mut ast) = parser.parse() {
        println!("Parsed successfully.");
        println!("{:?}", ast);
        let mut resolver = Resolver::new();
        match resolver.run(&mut ast) {
            Ok(()) => {
                if let Err(err) = interpreter.run(ast) {
                    println!("{:?}", err);
                }
            }
            Err(err) => println!("{:?}", err),
        }
    } else {
        println!("Error while parsing.");
    }
}

fn run_file(file: &String) {
    let contents = fs::read_to_string(file).expect("Expected file.");
    println!("{}", contents);
}

fn run_prompt() {
    println!("interactive lox");
    let mut interpreter = Interpreter::new();
    loop {
        print!(">");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .expect("Failed to read line");
        run(line, &mut interpreter);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match &args[..] {
        [_] => run_prompt(),
        [_, file] => run_file(file),
        _ => println!("Usage: lox [script]"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter::test_utils::test_interpret;
    use test_utils::*;
    use value::Value;

    #[test]
    fn test_arithmetic() {
        let c = test_interpret("var a = 1; var b = 2; var c = a + b;", "c");
        assert!(matches!(c, Value::Number(n) if n == 3.0));
    }

    #[test]
    fn test_block() {
        let s = "
        var a = 1;
        {
            var a = 2;
        }";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_if() {
        let a = test_interpret(
            "
            var a = 1;
            if (1 == 1)
                a = 2;",
            "a",
        );

        assert_eq!(a, Value::Number(2.0));
    }

    #[test]
    fn test_while() {
        let s = "var a = 1;
        while (a < 3)
            a = a + 1;";
        let a = test_interpret(s, "a");
        assert_eq!(a, Value::Number(3.0));
    }

    #[test]
    fn test_for() {
        let s = "
        var j = 0;
        for (var i = 0;
        i < 4;
        i = i + 1)
            j = j + i;";
        let j = test_interpret(s, "j");
        assert_eq!(j, Value::Number(6.0));
    }

    #[test]
    fn test_logical_and() {
        let s = "
        var a = 0;
        if (false and true)
            a = 1;";
        let a = test_interpret(s, "a");
        assert_eq!(a, Value::Number(0.0));
    }

    #[test]
    fn test_logical_or() {
        let s = "
        var a = 0;
        if (false or true)
            a = 1;";
        let a = test_interpret(s, "a");
        assert_eq!(a, Value::Number(1.0));
    }

    #[test]
    fn test_call() {
        let s = "
        var a = clock();
        var c = 0;
        for (var i = 0; i < 10000; i = i + 1) {
           c = c + 1; 
        }
        var b = clock() - a;";
        let b = test_interpret(s, "b");
        assert!(matches!(b, Value::Number(n) if n > 0.0));
    }

    #[test]
    fn test_nested_call() {
        let s = "
        var a = 1;
        fun foo() {
            fun bar() {
                a = 2;
            }
            bar();
        }
        fun bar() {
        }
        foo();";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 2.0));
    }

    #[test]
    fn test_duplicate_vars() {
        let s = "
        var a = 1;
        var a = 1;";
        let _ = test_interpret(s, "a");
    }

    #[test]
    fn test_nesting_function() {
        let s = "
        var a = 1;
        fun make_a() {
            var a = 2;
            {
                var a = 3;
                return;
            }
        }";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }
    
    #[test]
    fn test_nesting_function2() {
        let s = "
        fun calc_b() {
            var a = 2;
            {
                var a = 3;
                return a;
            }
        }

        var b = calc_b();";
        let b = test_interpret(s, "b");
        assert!(matches!(b, Value::Number(n) if n == 2.0));
    }

    #[test]
    #[should_panic(expected = "Parse failed")]
    fn test_var_in_loop() {
        let s = "
        var a = 1;
        for (var i = 0; i < 2; i = i + 1) var a = 2;";
        let _ = test_interpret(s, "a");

    }

    #[test]
    fn test_simple_class() {
        let s = "
        class Foo {
            method() {
            }
        }

        var foo = Foo();
        foo.method();";
        let _ = test_run(s);
    }

    #[test]
    fn test_class() {
        let s = "
        class Foo {
            bar() {
                class Foo {
                    bar2() {
                    }
                }
                var foo = Foo();
                foo.bar2();

            }
        }

        var foo = Foo();
        foo.bar();";
        let _ = test_run(s);
    }


    #[test]
    fn test_method() {
        let s = "
        class Foo {
            bar() {
                return 1;
            }
        }
        var foo = Foo();
        var a = foo.bar();";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_object_assign() {
        let s = "
        class Foo {
        }

        var foo = Foo();
        foo.bar = 1;";
        let _ = test_run(s);
    }
    
    #[test]
    fn test_object_assign2() {
        let s = "
        class Foo {
        }

        var foo = Foo();
        foo.bar = 1;
        var a = foo.bar;";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_this() {
        let s = "
        class Foo {
            f() {
                this.bar = 1;
                return this.bar;
            }
        }
        var foo = Foo();
        var a = foo.f();";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_this2() {
        let s = "
        class Foo {
            init() {
                this.a = 1;
            }

            do_thing() {
                return this.a;
            }
        }

        var foo1 = Foo();
        var foo2 = Foo();
        foo2.a = 2;
        foo2.do_thing = foo1.do_thing;
        var a = foo2.do_thing();";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }
    
    #[test]
    fn test_this3() {
        let s = "
        class Foo {
            do_thing() {
                return this.a;
            }
        }

        var foo = Foo();
        foo.a = 1;
        var a = foo.do_thing();";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_closure() {
        let s = "
        fun create_closure() {
            var a = 1;
            fun closure() {
                return a;
            }
            return closure;
        }
        var my_closure = create_closure();
        var a = my_closure();";
        let a = test_interpret(s, "a");
        assert!(matches!(a, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_init1() {
        let s = "
        class A {
            init() {
                this.field = 1;
            }
        }
        var a = A();
        var b = a.field;";
        let b = test_interpret(s, "b");
        assert!(matches!(b, Value::Number(n) if n == 1.0));
    }

    #[test]
    fn test_init2() {
        let s = "
        class A {
            init() {
                this.field = 1;
            }
        }
        var a = A();
        var object = a.init();
        var b = a.field + object.field;";
        let b = test_interpret(s, "b");
        assert!(matches!(b, Value::Number(n) if n == 2.0));

    }

    #[test]
    fn test_super_class() {
        let s = "
        class B {
            do_thing() {
                return 5;
            }
        }
        class A < B {}

        var a = A();
        var c = a.do_thing();";
        let c = test_interpret(s, "c");
        assert!(matches!(c, Value::Number(n) if n == 5.0));
    }

    #[test]
    fn test_super_call() {
        let s = "
        class B {
            do_thing() {
                return 5;
            }
        }
        class A < B {
            do_thing() {
                return super.do_thing() + 1;
            }
        }
        var a = A();
        var c = a.do_thing();";
        let c = test_interpret(s, "c");
        assert!(matches!(c, Value::Number(n) if n == 6.0));
    }
}
