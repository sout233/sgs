// tests/integration_test.rs
use sgs::ast::*;
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
        if let Stmt::Let { name, value } = &func1.statements[0] {
            assert_eq!(name, "lambda_print");
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
