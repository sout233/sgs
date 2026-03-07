use sgs::ast::*;
use sgs::interpreter::{Interpreter, Value};
use sgs::parse_program;

#[test]
fn test_parse_player_entity() {
    let script = r#"
        @type Entity;
        @name Player;

        mount Transition;
        mount Health;
        mount Attack;
    "#;

    let ast = parse_program(script).unwrap();
    assert_eq!(ast.len(), 1);

    if let SgsNode::EntityDef(entity) = &ast[0] {
        assert_eq!(entity.name, "Player");
        assert_eq!(entity.components, vec!["Transition", "Health", "Attack"]);
    } else {
        panic!("Expected EntityDef");
    }
}

#[test]
fn test_parse_transition_component() {
    let script = r#"
        // transition.sgs
        @type Component;
        @name Transition;

        param position : Vector2;
    "#;

    let ast = parse_program(script).unwrap();
    assert_eq!(ast.len(), 1);

    if let SgsNode::ComponentDef(comp) = &ast[0] {
        assert_eq!(comp.name, "Transition");
        assert_eq!(comp.params.len(), 1);
        assert_eq!(comp.params[0].name, "position");
        assert_eq!(comp.params[0].ty, "Vector2");
    } else {
        panic!("Expected ComponentDef");
    }
}

#[test]
fn test_parse_movement_system() {
    let script = r#"
        // movement.sgs
        @type System;
        @name Movement;

        require mut Transition;

        fn _process() -> void {
            Transition.position.x += 1;
        }
    "#;

    let ast = parse_program(script).unwrap();
    assert_eq!(ast.len(), 1);

    if let SgsNode::SystemDef(sys) = &ast[0] {
        assert_eq!(sys.name, "Movement");

        assert_eq!(sys.required_components.len(), 1);
        assert!(sys.required_components[0].is_mut);
        assert_eq!(sys.required_components[0].name, "Transition");

        assert_eq!(sys.functions.len(), 1);
        let func = &sys.functions[0];
        assert_eq!(func.name, "_process");

        assert_eq!(func.statements.len(), 1);
        assert_eq!(
            func.statements[0],
            Stmt::Assign(AssignStmt {
                target_path: vec!["Transition", "position", "x"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                op: "+=".into(),
                value: Expr::Number(1.0)
            })
        );
    } else {
        panic!("Expected SystemDef");
    }
}

#[test]
fn test_fn() {
    let script = r#"
        @type System;
        @name Logic;

        // fn.sgs
        fn _process() -> void {
            let lambda_print = |msg: string| {
                print(msg);
            };
            lambda_print("hello world");
            idk(lambda_print);
        }

        fn idk(some_func: func(string) -> void) {
            some_func("hello world from idk");
            some_func("hello world from idk again");
        }
    "#;

    let ast = parse_program(script).unwrap();
    assert_eq!(ast.len(), 1);

    if let SgsNode::SystemDef(sys) = &ast[0] {
        assert_eq!(sys.functions.len(), 2);

        // _process fn
        let func1 = &sys.functions[0];
        assert_eq!(func1.name, "_process");
        assert_eq!(func1.params.len(), 0);
        assert_eq!(func1.return_ty.as_deref(), Some("void"));
        assert_eq!(func1.statements.len(), 3);

        // let 闭包
        if let Stmt::Let {
            is_mut,
            name,
            value,
        } = &func1.statements[0]
        {
            assert_eq!(name, "lambda_print");
            assert!(!is_mut);
            if let Expr::Closure { params, body } = value {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "msg");
                assert_eq!(params[0].ty, "string");
                assert_eq!(body.len(), 1);
            } else {
                panic!("Expected Closure in Let statement");
            }
        } else {
            panic!("Expected Let statement");
        }

        // lambda_print("hello world");
        if let Stmt::Expr(Expr::Call { target, args }) = &func1.statements[1] {
            assert_eq!(**target, Expr::Path(vec!["lambda_print".to_string()]));
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Expr::StringLit("hello world".to_string()));
        } else {
            panic!("Expected Call expression statement");
        }

        // 验证 idk fn
        let func2 = &sys.functions[1];
        assert_eq!(func2.name, "idk");
        assert_eq!(func2.params.len(), 1);
        assert_eq!(func2.params[0].name, "some_func");
        assert_eq!(func2.params[0].ty, "func(string)->void"); // 空格没了
        assert_eq!(func2.statements.len(), 2);

        // some_func("hello world from idk");
        if let Stmt::Expr(Expr::Call { target, args }) = &func2.statements[0] {
            assert_eq!(**target, Expr::Path(vec!["some_func".to_string()]));
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Expr::StringLit("hello world from idk".to_string()));
        }
    } else {
        panic!("Expected SystemDef");
    }
}

#[test]
#[allow(unused)]
fn test_calculation_parsing() {
    let script = r#"
        @type System;
        @name MathTest;

        fn calculate() -> void {
            let base_val = 100;

            Player.hp += 50;
            Player.hp -= 20;
            Player.multiplier *= 2;
            Player.defense /= 1.5;
        }
    "#;

    let ast = parse_program(script).unwrap();
    assert_eq!(ast.len(), 1);

    if let SgsNode::SystemDef(sys) = &ast[0] {
        assert_eq!(sys.functions.len(), 1);
        let func = &sys.functions[0];

        assert_eq!(func.statements.len(), 5);

        if let Stmt::Let {
            is_mut,
            name,
            value,
        } = &func.statements[0]
        {
            assert_eq!(name, "base_val");
            assert!(!is_mut);
            assert_eq!(*value, Expr::Number(100.0));
        } else {
            panic!("应是let");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
        }) = &func.statements[1]
        {
            assert_eq!(target_path, &vec!["Player", "hp"]);
            assert_eq!(op, "+=");
            assert_eq!(*value, Expr::Number(50.0));
        } else {
            panic!("应该是+=");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
        }) = &func.statements[2]
        {
            assert_eq!(op, "-=");
            assert_eq!(*value, Expr::Number(20.0));
        } else {
            panic!("应当是-=");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
        }) = &func.statements[3]
        {
            assert_eq!(op, "*=");
            assert_eq!(*value, Expr::Number(2.0));
        } else {
            panic!("应该是 *=");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
        }) = &func.statements[4]
        {
            assert_eq!(op, "/=");
            assert_eq!(*value, Expr::Number(1.5));
        } else {
            panic!("应该是 /=");
        }
    } else {
        panic!("应该是 SystemDef 节点");
    }
}

#[test]
fn test_complex_math_expression() {
    let script = r#"
        @type System;
        @name MathTest;

        fn calc() -> void {
            let result = 10 + 5 * (2 - Player.defense);
        }
    "#;

    let ast = parse_program(script).unwrap();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let func = &sys.functions[0];

        if let Stmt::Let {
            is_mut,
            name,
            value,
        } = &func.statements[0]
        {
            assert_eq!(name, "result");
            assert!(!is_mut);

            // 外层应当是加法
            if let Expr::BinaryOp { left, op, right } = value {
                assert_eq!(op, "+");
                assert_eq!(**left, Expr::Number(10.0));

                // 右侧是乘法
                if let Expr::BinaryOp {
                    left: mul_left,
                    op: mul_op,
                    right: mul_right,
                } = &**right
                {
                    assert_eq!(mul_op, "*");
                    assert_eq!(**mul_left, Expr::Number(5.0));

                    // 括号内是减法
                    if let Expr::BinaryOp {
                        left: sub_left,
                        op: sub_op,
                        right: sub_right,
                    } = &**mul_right
                    {
                        assert_eq!(sub_op, "-");
                        assert_eq!(**sub_left, Expr::Number(2.0));
                        assert_eq!(
                            **sub_right,
                            Expr::Path(vec!["Player".to_string(), "defense".to_string()])
                        );
                    } else {
                        panic!("预期括号内为减法节点");
                    }
                } else {
                    panic!("预期右侧为乘法节点");
                }
            } else {
                panic!("预期最外层为加法节点");
            }
        } else {
            panic!("Expected Let stmt");
        }
    } else {
        panic!("Expected SystemDef");
    }
}

#[test]
fn test_interpreter_math_execution() {
    let script = r#"
        @type System;
        @name EngineMath;

        fn _process() -> void {
            let base = 10;
            let offset = 2 * 3;            // 6
            let mut total = base + offset; // 16

            total += 4;                    // 20
            total *= (2 + 3);              // 100
            total /= 2;                    // 50
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let func = &sys.functions[0];

        let mut vm = sgs::interpreter::Interpreter::new();

        vm.env.push_scope();

        for stmt in &func.statements {
            let result = vm.eval_stmt(stmt);
            assert!(result.is_ok(), "计算测试已经爆炸，亿万代码必须重写: {:?}", result.err());
        }

        assert_eq!(vm.env.get_val("base").unwrap(), sgs::interpreter::Value::Number(10.0));
        assert_eq!(vm.env.get_val("offset").unwrap(), sgs::interpreter::Value::Number(6.0));
        assert_eq!(vm.env.get_val("total").unwrap(), sgs::interpreter::Value::Number(50.0));

        vm.env.pop_scope();

        assert_eq!(vm.env.get_val("total"), None);

    } else {
        panic!("预期应该是SystemDef");
    }
}

#[test]
fn test_mutability_and_closure_scope() {
    let script = r#"
        @type System;
        @name ScopeTest;

        fn test_scope() -> void {
            let mut x = 10;
            x += 5; // x to 15

            let constant_val = 100;

            let modifier = |step: number| {
                let local_var = 2;
                x += step * local_var;
            };

            // x += 3 * 2 => x 变成 21
            modifier(3);
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let func = &sys.functions[0];

        vm.env.push_scope();

        for stmt in &func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "语句执行失败: {:?}", res.err());
        }

        assert_eq!(vm.env.get_val("x").unwrap(), Value::Number(21.0));

        let err = vm.env.set("constant_val", Value::Number(999.0));
        assert!(err.is_err());
        assert_eq!(err.unwrap_err(), "不可变变量 'constant_val' 无法被重新赋值，请使用 let mut 声明");

        assert_eq!(vm.env.get_val("local_var"), None);

        vm.env.pop_scope();

        assert_eq!(vm.env.get_val("x"), None);
        assert_eq!(vm.env.get_val("constant_val"), None);

    } else {
        panic!("应该是 SystemDef");
    }
}

#[test]
fn test_immutable_assignment_error() {
    let script = r#"
        @type System;
        @name MutErrorTest;

        fn test_err() -> void {
            let pi = 3.14;
            // err here
            pi = 3.14159;
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let func = &sys.functions[0];

        let result = vm.execute_function(func);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "不可变变量 'pi' 无法被重新赋值，请使用 let mut 声明");
    } else {
        panic!("应该是 SystemDef");
    }
}
