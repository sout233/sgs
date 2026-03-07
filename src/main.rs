use sgs::ast::SgsNode;
use sgs::interpreter::Interpreter;
use sgs::parse_program;
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
                // 截取报错位置之前的全部源代码
                let prefix = &source[..span.start];
                // 倒序查找最后一个不是空白/换行符的字符位置
                if let Some(last_char_pos) = prefix.rfind(|c: char| !c.is_whitespace()) {
                    let c_len = prefix[last_char_pos..].chars().next().unwrap().len_utf8();
                    span = last_char_pos..(last_char_pos + c_len);
                }
            }

            Report::build(ReportKind::Error, (filename, span.clone()))
                .with_message("Parse Error")
                .with_config(Config::default().with_compact(false))
                .with_label(
                    Label::new((filename, span))
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

    let mut vm = Interpreter::new();
    let mut executed = false;
    for node in ast {
        if let SgsNode::SystemDef(sys) = node {
            for func in sys.functions {
                if func.name == "main" {
                    println!("Running {}", sys.name);

                    if let Err((msg, span)) = vm.execute_function(&func) {
                        Report::build(ReportKind::Error, (filename, span.clone()))
                            .with_message("Runtime Error")
                            .with_config(Config::default().with_compact(false))
                            .with_label(
                                Label::new((filename, span))
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
