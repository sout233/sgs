use sgs::ast::*;
use sgs::parse_program;

#[test]
fn test_parse_player_entity() {
    let script = r#"
        @type Entity
        @name Player

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
        @type Component
        @name Transition

        param position : Vector2
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
        @type System
        @name Movement

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
        assert_eq!(func.statements[0].op, "+=");

        let path = &func.statements[0].target_path;
        assert_eq!(path, &vec!["Transition", "position", "x"]);

        if let Expr::Number(val) = func.statements[0].value {
            assert_eq!(val, 1.0);
        } else {
            panic!("Expected Number expr");
        }
    } else {
        panic!("Expected SystemDef");
    }
}
