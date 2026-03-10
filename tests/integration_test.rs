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
            func.statements[0].node,
            Stmt::Assign(AssignStmt {
                target_path: vec!["Transition", "position", "x"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                op: "+=".into(),
                value: Expr::Number(1.0),
                index: None
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
        } = &func1.statements[0].node
        {
            assert_eq!(name, "lambda_print");
            assert!(!*is_mut);
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
        if let Stmt::Expr(Expr::Call { target, args }) = &func1.statements[1].node {
            assert_eq!(**target, Expr::Path(vec!["lambda_print".to_string()]));
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Expr::StringLit("hello world".to_string()));
        } else {
            panic!("Expected Call expression statement");
        }

        // idk fn
        let func2 = &sys.functions[1];
        assert_eq!(func2.name, "idk");
        assert_eq!(func2.params.len(), 1);
        assert_eq!(func2.params[0].name, "some_func");
        assert_eq!(func2.params[0].ty, "func(string)->void");
        assert_eq!(func2.statements.len(), 2);

        // some_func("hello world from idk");
        if let Stmt::Expr(Expr::Call { target, args }) = &func2.statements[0].node {
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
        } = &func.statements[0].node
        {
            assert_eq!(name, "base_val");
            assert!(!*is_mut);
            assert_eq!(*value, Expr::Number(100.0));
        } else {
            panic!("应是let");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
            index,
        }) = &func.statements[1].node
        {
            assert_eq!(target_path, &vec!["Player", "hp"]);
            assert_eq!(op, "+=");
            assert_eq!(*value, Expr::Number(50.0));
            assert!(index.is_none());
        } else {
            panic!("应该是+=");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
            index,
        }) = &func.statements[2].node
        {
            assert_eq!(op, "-=");
            assert_eq!(*value, Expr::Number(20.0));
            assert!(index.is_none());
        } else {
            panic!("应当是-=");
        }

        if let Stmt::Assign(AssignStmt {
            target_path,
            op,
            value,
            index,
        }) = &func.statements[3].node
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
            index,
        }) = &func.statements[4].node
        {
            assert_eq!(op, "/=");
            assert_eq!(*value, Expr::Number(1.5));
            assert!(index.is_none());
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
        } = &func.statements[0].node
        {
            assert_eq!(name, "result");
            assert!(!*is_mut);

            if let Expr::BinaryOp { left, op, right } = value {
                assert_eq!(op, "+");
                assert_eq!(**left, Expr::Number(10.0));

                if let Expr::BinaryOp {
                    left: mul_left,
                    op: mul_op,
                    right: mul_right,
                } = &**right
                {
                    assert_eq!(mul_op, "*");
                    assert_eq!(**mul_left, Expr::Number(5.0));

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

        fn main() -> void {
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
            assert!(
                result.is_ok(),
                "计算测试已经爆炸，亿万代码必须重写: {:?}",
                result.err()
            );
        }

        assert_eq!(
            vm.env.get_val("base").unwrap(),
            sgs::interpreter::Value::Number(10.0)
        );
        assert_eq!(
            vm.env.get_val("offset").unwrap(),
            sgs::interpreter::Value::Number(6.0)
        );
        assert_eq!(
            vm.env.get_val("total").unwrap(),
            sgs::interpreter::Value::Number(50.0)
        );

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
        assert_eq!(
            err.unwrap_err(),
            "不可变变量 'constant_val' 无法被重新赋值，请使用 let mut 声明"
        );

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
        assert_eq!(result.unwrap_err().0, "不可变变量 'pi' 无法被重新赋值");
    } else {
        panic!("应该是 SystemDef");
    }
}

#[test]
fn test_advanced_expressions_and_interpolation() {
    let script = r#"
        @type System;
        @name AdvancedSyntax;

        fn get_32() -> number {
            // 隐式返回 (无分号)
            32
        }

        fn get_10() -> number {
            // 显式返回
            return 10;
        }

        fn main() -> void {
            // 验证跨行多行表达式，注意优先级：33/11 先算
            let a =
                32 + 33 / 11 + 10
                + get_32()
                - get_10();

            // 验证字符串内插，并且在花括号里做运算！
            let msg = $"Result a is {a}, next is {a + 1}";
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = sgs::interpreter::Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        for func in &sys.functions {
            let params = func
                .params
                .iter()
                .map(|p| (p.is_mut, p.name.clone()))
                .collect();

            let closure_val = sgs::interpreter::Value::Closure {
                params,
                body: func.statements.clone(),
                captured_env: vm.env.scopes.clone(),
            };

            vm.env.define(func.name.clone(), closure_val, false);
        }

        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();

        vm.env.push_scope();
        for stmt in &main_func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "执行失败: {:?}", res.err());
        }

        assert_eq!(
            vm.env.get_val("a").unwrap(),
            sgs::interpreter::Value::Number(67.0)
        );

        assert_eq!(
            vm.env.get_val("msg").unwrap(),
            sgs::interpreter::Value::String("Result a is 67, next is 68".to_string())
        );

        vm.env.pop_scope();
    } else {
        panic!("预期应该是 SystemDef");
    }
}

#[test]
fn test_array_operations() {
    let script = r#"
        @type System;
        @name ArrayTest;

        fn main() -> void {
            let mut arr = [10, 20, 30];

            let first = arr[0];

            arr[1] += 5;   // 25
            arr[2] -= 10;  // 20
            arr[0] = 99;   // 99

            let res0 = arr[0];
            let res1 = arr[1];
            let res2 = arr[2];
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = sgs::interpreter::Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();

        vm.env.push_scope();
        for stmt in &main_func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "数组操作执行失败: {:?}", res.err());
        }

        assert_eq!(
            vm.env.get_val("first").unwrap(),
            sgs::interpreter::Value::Number(10.0)
        );
        assert_eq!(
            vm.env.get_val("res0").unwrap(),
            sgs::interpreter::Value::Number(99.0)
        );
        assert_eq!(
            vm.env.get_val("res1").unwrap(),
            sgs::interpreter::Value::Number(25.0)
        );
        assert_eq!(
            vm.env.get_val("res2").unwrap(),
            sgs::interpreter::Value::Number(20.0)
        );
    }
}

#[test]
fn test_while_loop_with_control_flow() {
    let script = r#"
        @type System;
        @name WhileTest;

        fn main() -> void {
            let mut i = 0;
            let mut sum = 0;

            while i < 10 {
                i += 1;

                if i == 3 {
                    continue;
                }

                sum += i;

                if i == 4 {
                    break; // sum = 1 + 2 + 4 = 7
                }
            }
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = sgs::interpreter::Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();

        vm.env.push_scope();
        for stmt in &main_func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "while 循环执行失败: {:?}", res.err());
        }

        assert_eq!(
            vm.env.get_val("i").unwrap(),
            sgs::interpreter::Value::Number(4.0)
        );
        assert_eq!(
            vm.env.get_val("sum").unwrap(),
            sgs::interpreter::Value::Number(7.0)
        );
    }
}

#[test]
fn test_for_in_loop() {
    let script = r#"
        @type System;
        @name ForTest;

        fn main() -> void {
            let arr = [10, 20, 30, 40, 50];
            let mut total = 0;

            for val in arr {
                if val == 20 {
                    continue;
                }
                if val == 40 {
                    break;
                }
                total += val;
            }
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = sgs::interpreter::Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();

        vm.env.push_scope();
        for stmt in &main_func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "for 循环执行失败: {:?}", res.err());
        }

        assert_eq!(
            vm.env.get_val("total").unwrap(),
            sgs::interpreter::Value::Number(40.0)
        );
        assert_eq!(vm.env.get_val("val"), None);
    }
}

#[test]
fn test_string_concatenation_syntax() {
    let script = r#"
        @type System;
        @name ConcatTest;

        fn main() -> void {
            let mut msg = "Player " ++ "Sout";
            msg ++= " has joined.";
            let final_msg = msg ++ " Welcome!";
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut vm = sgs::interpreter::Interpreter::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();

        vm.env.push_scope();
        for stmt in &main_func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "字符串拼接执行失败: {:?}", res.err());
        }

        assert_eq!(
            vm.env.get_val("msg").unwrap(),
            sgs::interpreter::Value::String("Player Sout has joined.".into())
        );
        assert_eq!(
            vm.env.get_val("final_msg").unwrap(),
            sgs::interpreter::Value::String("Player Sout has joined. Welcome!".into())
        );
    }
}

#[test]
fn test_analyzer_rejects_plus_for_strings() {
    let script = r#"
        @type System;
        @name AnalyzerErrorTest;

        fn main() -> void {
            let bad_concat = "Hello " + "World";
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut analyzer = sgs::analyzer::Analyzer::new();

    if let SgsNode::SystemDef(sys) = &ast[0] {
        analyzer.register_functions(sys);
        for func in &sys.functions {
            analyzer.check_function(func);
        }
    }

    assert_eq!(analyzer.errors.len(), 1);

    let err = &analyzer.errors[0];
    assert_eq!(err.title, "操作符错误");
    assert!(err.message.contains("不能使用 '+' 来操作字符串"));

    let note = err.note.as_ref().unwrap();
    assert!(note.0.contains("SGS 使用 '++'"));
}

#[test]
fn test_struct_instantiation_and_deep_assignment() {
    let script = r#"
        @type System;
        @name StructTest;

        struct Vector2 { x: number, y: number }
        struct Player { pos: Vector2, hp: number }

        fn main() -> void {
            let mut p = Player {
                pos: Vector2 { x: 10, y: 20 },
                hp: 100
            };

            // 深度修改
            p.pos.x += 5;
            p.hp -= 10;

            let final_x = p.pos.x;
            let final_hp = p.hp;
        }
    "#;

    let ast = parse_program(script).unwrap();
    let mut analyzer = sgs::analyzer::Analyzer::new();

    for node in &ast {
        match node {
            SgsNode::StructDef(s) => analyzer.register_struct(s),
            SgsNode::SystemDef(sys) => analyzer.register_functions(sys),
            _ => {}
        }
    }
    for node in &ast {
        if let SgsNode::SystemDef(sys) = node {
            for func in &sys.functions {
                analyzer.check_function(func);
            }
        }
    }
    assert!(analyzer.errors.is_empty(), "静态检查报错: {:?}", analyzer.errors.iter().map(|e| &e.message).collect::<Vec<_>>());

    let mut vm = Interpreter::new();
    if let SgsNode::SystemDef(sys) = ast.iter().find(|n| matches!(n, SgsNode::SystemDef(_))).unwrap() {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();
        vm.env.push_scope();
        for stmt in &main_func.statements {
            let res = vm.eval_stmt(stmt);
            assert!(res.is_ok(), "结构体测试执行失败: {:?}", res.err());
        }

        assert_eq!(vm.env.get_val("final_x").unwrap(), Value::Number(15.0));
        assert_eq!(vm.env.get_val("final_hp").unwrap(), Value::Number(90.0));
    }
}

#[test]
fn test_ufcs_and_mut_parameters() {
    let script = r#"
        @type System;
        @name UFCSTest;

        struct Stats { atk: number }

        fn buff(mut s: Stats, amount: number) -> void {
            s.atk += amount;
        }

        fn main() -> void {
            let mut my_stats = Stats { atk: 10 };

            my_stats.buff(5);
            my_stats.buff(10);

            let final_atk = my_stats.atk;
        }
    "#;

    let ast = parse_program(script).unwrap();
    let mut analyzer = sgs::analyzer::Analyzer::new();
    for node in &ast {
        match node {
            SgsNode::StructDef(s) => analyzer.register_struct(s),
            SgsNode::SystemDef(sys) => analyzer.register_functions(sys),
            _ => {}
        }
    }
    for node in &ast {
        if let SgsNode::SystemDef(sys) = node {
            for func in &sys.functions { analyzer.check_function(func); }
        }
    }
    assert!(analyzer.errors.is_empty());

    let mut vm = Interpreter::new();
    if let SgsNode::SystemDef(sys) = ast.iter().find(|n| matches!(n, SgsNode::SystemDef(_))).unwrap() {
        for func in &sys.functions {
            let params = func.params.iter().map(|p| (p.is_mut, p.name.clone())).collect();
            let closure_val = Value::Closure {
                params, body: func.statements.clone(), captured_env: vm.env.scopes.clone()
            };
            vm.env.define(func.name.clone(), closure_val, false);
        }

        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();
        vm.env.push_scope();
        for stmt in &main_func.statements {
            vm.eval_stmt(stmt).unwrap();
        }

        // 10 + 5 + 10 = 25
        assert_eq!(vm.env.get_val("final_atk").unwrap(), Value::Number(25.0));
    }
}

#[test]
fn test_type_casting_as_keyword() {
    let script = r#"
        @type System;
        @name CastTest;

        fn main() -> void {
            let s = "100" as number;
            let n = s + 50;           // n = 150
            let res_str = n as string; // res_str = "150"
            let is_true = true as number; // is_true = 1
        }
    "#;

    let ast = parse_program(script).unwrap();
    let mut analyzer = sgs::analyzer::Analyzer::new();
    if let SgsNode::SystemDef(sys) = &ast[0] {
        analyzer.register_functions(sys);
        for func in &sys.functions { analyzer.check_function(func); }
    }
    assert!(analyzer.errors.is_empty());

    let mut vm = Interpreter::new();
    if let SgsNode::SystemDef(sys) = &ast[0] {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();
        vm.env.push_scope();
        for stmt in &main_func.statements {
            vm.eval_stmt(stmt).unwrap();
        }

        assert_eq!(vm.env.get_val("s").unwrap(), Value::Number(100.0));
        assert_eq!(vm.env.get_val("n").unwrap(), Value::Number(150.0));
        assert_eq!(vm.env.get_val("res_str").unwrap(), Value::String("150".into()));
        assert_eq!(vm.env.get_val("is_true").unwrap(), Value::Number(1.0));
    }
}

#[test]
fn test_array_builtin_methods() {
    let script = r#"
        @type System;
        @name ArrayMethodsTest;

        fn main() -> void {
            let mut arr = [1, 2, 3];

            arr.push(4);          // [1, 2, 3, 4]
            let l = arr.len();    // 4

            let popped = arr.pop(); // 4, arr = [1, 2, 3]

            arr.remove(0);        // arr = [2, 3]
            let first = arr[0];   // 2
        }
    "#;

    let ast = parse_program(script).unwrap();
    let mut analyzer = sgs::analyzer::Analyzer::new();
    if let SgsNode::SystemDef(sys) = &ast[0] {
        analyzer.register_functions(sys);
        for func in &sys.functions { analyzer.check_function(func); }
    }
    assert!(analyzer.errors.is_empty(), "数组方法静态检查失败");

    let mut vm = Interpreter::new();
    if let SgsNode::SystemDef(sys) = &ast[0] {
        let main_func = sys.functions.iter().find(|f| f.name == "main").unwrap();
        vm.env.push_scope();
        for stmt in &main_func.statements {
            vm.eval_stmt(stmt).unwrap();
        }

        assert_eq!(vm.env.get_val("l").unwrap(), Value::Number(4.0));
        assert_eq!(vm.env.get_val("popped").unwrap(), Value::Number(4.0));
        assert_eq!(vm.env.get_val("first").unwrap(), Value::Number(2.0));
    }
}

#[test]
fn test_ecs_engine_bridge() {
    let script = r#"
        @type System;
        @name MovementSystem;

        fn _process() -> void {
            for mut pos in __query_Position {
                pos.x += 1.0;
                pos.y -= 0.5;
            }
        }
    "#;

    let ast = sgs::parse_program(script).unwrap();
    let mut analyzer = sgs::analyzer::Analyzer::new();

    analyzer.define_var(
        "__query_Position".to_string(),
        false,
        sgs::analyzer::Type::Array(Box::new(sgs::analyzer::Type::Any)), // 宽容类型
        &(0..0)
    );

    if let sgs::ast::SgsNode::SystemDef(sys) = &ast[0] {
        analyzer.register_functions(sys);
        for func in &sys.functions { analyzer.check_function(func); }
    }
    assert!(analyzer.errors.is_empty(), "静态检查失败: {:?}", analyzer.errors.iter().map(|e| &e.message).collect::<Vec<_>>());

    #[derive(Debug, Clone, PartialEq)]
    struct Position { x: f64, y: f64 }

    let mut engine_world = vec![
        Position { x: 0.0, y: 0.0 },
        Position { x: 10.0, y: 20.0 },
    ];

    let mut sgs_array = Vec::new();
    for pos in &engine_world {
        let mut fields = std::collections::HashMap::new();
        fields.insert("x".to_string(), sgs::interpreter::Value::Number(pos.x));
        fields.insert("y".to_string(), sgs::interpreter::Value::Number(pos.y));

        sgs_array.push(sgs::interpreter::Value::Struct {
            name: "Position".to_string(),
            fields: std::rc::Rc::new(std::cell::RefCell::new(fields)),
        });
    }
    let injected_array = sgs::interpreter::Value::Array(std::rc::Rc::new(std::cell::RefCell::new(sgs_array)));

    let mut vm = sgs::interpreter::Interpreter::new();

    vm.env.scopes[0].insert(
        "__query_Position".to_string(),
        std::rc::Rc::new(std::cell::RefCell::new(sgs::interpreter::Variable {
            value: injected_array.clone(),
            is_mut: false,
        }))
    );

    if let sgs::ast::SgsNode::SystemDef(sys) = &ast[0] {
        let process_func = sys.functions.iter().find(|f| f.name == "_process").unwrap();
        vm.env.push_scope();
        for stmt in &process_func.statements {
            vm.eval_stmt(stmt).unwrap();
        }
        vm.env.pop_scope();
    }

    if let sgs::interpreter::Value::Array(arr_rc) = injected_array {
        let arr = arr_rc.borrow();
        for (i, val) in arr.iter().enumerate() {
            if let sgs::interpreter::Value::Struct { fields, .. } = val {
                let map = fields.borrow();
                let new_x = if let sgs::interpreter::Value::Number(n) = map.get("x").unwrap() { *n } else { 0.0 };
                let new_y = if let sgs::interpreter::Value::Number(n) = map.get("y").unwrap() { *n } else { 0.0 };

                engine_world[i].x = new_x;
                engine_world[i].y = new_y;
            }
        }
    }

    assert_eq!(engine_world[0], Position { x: 1.0, y: -0.5 });
    assert_eq!(engine_world[1], Position { x: 11.0, y: 19.5 });

    println!("OK");
}
