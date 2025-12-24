use lalrpop_util::lalrpop_mod;

mod ast;
mod errors;
lalrpop_mod!(pub grammar);

fn main() {
    env_logger::init();

    let source = r#"
p1*(q2/g) + p2*(q1/g)
    "#;

    let error_handler = errors::ErrorHandler::new("dummy.eta", source);
    let lexed = grammar::ExprParser::new()
        .parse(source)
        .map_err(|e| error_handler.lexical_error(e));

    match lexed {
        Ok(prgm) => println!("{:?}", prgm),
        Err(e) => error_handler.show(&e),
    }
}
