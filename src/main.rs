/// 可能是运行时，可以运行一个sgs文件
// main.rs
use sgs::interpreter::{Interpreter, Value};
use sgs::parse_program;
use sgs::{analyzer::Analyzer, ast::SgsNode};
use std::fs;

use ariadne::{Color, Config, Label, Report, ReportKind, Source};
use pest::error::InputLocation;

fn main() {
    let filename = "test.sgs";

    let source = fs::read_to_string(filename).unwrap();

    println!("Parsing {} ...", filename);

    let ast = match parse_program(&source) {
        Ok(nodes) => nodes,
        Err(e) => {
            let mut span = match e.location {
                InputLocation::Pos(pos) => pos..pos,
                InputLocation::Span((start, end)) => start..end,
            };

            let expected_msg = match e.variant {
                pest::error::ErrorVariant::ParsingError { positives, .. } => {
                    let expected: Vec<String> =
                        positives.iter().map(|rule| format!("{:?}", rule)).collect();
                    format!("语法错误: 这里应当是 {}", expected.join(" 或 "))
                }
                pest::error::ErrorVariant::CustomError { message } => message,
            };

            if expected_msg.contains("semicolon") {
                let mut prefix = &source[..span.start];

                loop {
                    let original_len = prefix.len();
                    prefix = prefix.trim_end();

                    if let Some(idx) = prefix.rfind("//")
                        && !prefix[idx..].contains('\n')
                    {
                        prefix = prefix[..idx].trim_end();
                    }

                    if prefix.ends_with("*/")
                        && let Some(idx) = prefix.rfind("/*")
                    {
                        prefix = prefix[..idx].trim_end();
                    }

                    if prefix.len() == original_len {
                        break;
                    }
                }

                if !prefix.is_empty() {
                    let last_char = prefix.chars().last().unwrap();
                    let last_pos = prefix.len() - last_char.len_utf8();
                    span = last_pos..(last_pos + last_char.len_utf8());
                }
            }

            let char_start = source[..span.start].chars().count();
            let char_end = source[..span.end].chars().count();
            let char_span = char_start..char_end;

            Report::build(ReportKind::Error, (filename, span.clone()))
                .with_message("Parse Error")
                .with_config(Config::default().with_compact(false))
                .with_label(
                    Label::new((filename, char_span))
                        .with_message(expected_msg)
                        .with_color(Color::Red),
                )
                .with_note("是否漏写了分号 ';'")
                .finish()
                .print((filename, Source::from(&source)))
                .unwrap();

            return;
        }
    };

    let mut analyzer = Analyzer::new();
    println!("Running Static Analysis...");

    for node in &ast {
        match node {
            SgsNode::ExternFunctionDef(ext_def) => analyzer.register_extern_function(ext_def),
            SgsNode::StructDef(s_def) => {
                analyzer.register_struct(s_def);
            }
            SgsNode::SystemDef(sys) => {
                analyzer.register_functions(sys);
            }
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

    if !analyzer.warnings.is_empty() {
        for warn in analyzer.warnings {
            let title = warn.title;
            let msg = warn.message;
            let byte_span = warn.span;

            if byte_span.start != 0 {
                let char_start = source[..byte_span.start].chars().count();
                let char_end = source[..byte_span.end].chars().count();
                let char_span = char_start..char_end;

                Report::build(ReportKind::Warning, (filename, char_span.clone()))
                    .with_message(title)
                    .with_config(Config::default().with_compact(false))
                    .with_label(
                        Label::new((filename, char_span))
                            .with_message(msg)
                            .with_color(Color::Yellow),
                    )
                    .finish()
                    .print((filename, Source::from(&source)))
                    .unwrap();
            } else {
                println!("\x1b[33mWarning: {}\x1b[0m\n  --> {}", title, msg);
            }
        }
    }

    if !analyzer.errors.is_empty() {
        for err in analyzer.errors {
            let title = err.title;
            let msg = err.message;
            let byte_span = err.span;

            if byte_span.start != 0 {
                let char_start = source[..byte_span.start].chars().count();
                let char_end = source[..byte_span.end].chars().count();
                let char_span = char_start..char_end;

                let mut labels = Vec::new();

                if let Some((note_msg, note_byte_span)) = err.note {
                    if note_byte_span.start != 0 {
                        let n_start = source[..note_byte_span.start].chars().count();
                        let n_end = source[..note_byte_span.end].chars().count();
                        let n_span = n_start..n_end;

                        let primary_label = Label::new((filename, char_span.clone()))
                            .with_message(msg)
                            .with_color(Color::Red);

                        let note_label = Label::new((filename, n_span.clone()))
                            .with_message(note_msg)
                            .with_color(Color::Blue);

                        if n_start < char_span.start {
                            labels.push(note_label);
                            labels.push(primary_label);
                        } else {
                            labels.push(primary_label);
                            labels.push(note_label);
                        }
                    } else {
                        labels.push(
                            Label::new((filename, char_span.clone()))
                                .with_message(msg)
                                .with_color(Color::Red),
                        );
                    }
                } else {
                    labels.push(
                        Label::new((filename, char_span.clone()))
                            .with_message(msg)
                            .with_color(Color::Red),
                    );
                }

                Report::build(ReportKind::Error, (filename, char_span.clone()))
                    .with_message(title)
                    .with_config(Config::default().with_compact(false))
                    .with_labels(labels)
                    .finish()
                    .print((filename, Source::from(&source)))
                    .unwrap();
            } else {
                eprintln!("Static check err: {}", msg);
            }
        }
        return;
    }

    println!("Static check passed");

    let mut vm = Interpreter::new();
    let mut executed = false;

    vm.register_native_fn("get_time", |_args| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        Ok(Value::Number(t))
    });

    vm.register_native_fn("debug_log", |args| {
        if let Some(Value::String(msg)) = args.get(0) {
            println!("\x1b[32m[SGS] {}\x1b[0m", msg);
            Ok(Value::Void)
        } else {
            Err("debug_log 需要一个 string 参数".to_string())
        }
    });

    for node in ast {
        if let SgsNode::SystemDef(sys) = node {
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

            for func in sys.functions {
                if func.name == "main" {
                    println!("Running {}\n", sys.name);

                    if let Err((msg, span)) = vm.execute_function(&func) {
                        let char_start = source[..span.start].chars().count();
                        let char_end = source[..span.end].chars().count();
                        let char_span = char_start..char_end;

                        Report::build(ReportKind::Error, (filename, char_span.clone()))
                            .with_message("Runtime Error")
                            .with_config(Config::default().with_compact(false))
                            .with_label(
                                Label::new((filename, char_span.clone()))
                                    .with_message(msg)
                                    .with_color(Color::Yellow),
                            )
                            .finish()
                            .print((filename, Source::from(&source)))
                            .unwrap();
                    } else {
                        println!("\n--- EOF ---");
                    }

                    executed = true;
                }
            }
        }
    }

    if !executed {
        println!("未找到可执行的 main() 函数。");
    }
}
