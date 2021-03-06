use std::clone::Clone;
use syntax::ast;
use syntax::ast::Ident;
use syntax::codemap::{Span, Spanned};
use syntax::ext::base::{ExtCtxt, MacResult, MacEager, DummyResult};
use syntax::ext::build::AstBuilder;
use syntax::parse::PResult;
use syntax::parse::parser::Parser;
use syntax::ptr::P;
use syntax::symbol::Symbol;
use syntax::util::small_vector::SmallVector;
use syntax::tokenstream::TokenTree;

use util;
use parser::{Value, Field, Message, MacroParser, RHSParser, ParserExt};

use aster::stmt::StmtBuilder;

struct IdentParser;
impl RHSParser for IdentParser {
    type RHS = Spanned<ast::Ident>;
    fn parse<'a>(&mut self, parser: &mut Parser<'a>) -> PResult<'a, Self::RHS> {
        parser.parse_spanned_ident()
    }
}

fn parse_protobuf<'a>(cx: &mut ExtCtxt<'a>, tts: &[TokenTree]) -> PResult<'a, (P<ast::Expr>, Message<Spanned<ast::Ident>>)> {
    let mut parser = cx.new_parser_from_tts(&tts.to_vec());
    MacroParser::new(&mut parser, IdentParser).parse_macro()
}

fn emit_field(cx: &mut ExtCtxt, sp: Span, field: Field<Spanned<ast::Ident>>, parent: P<ast::Expr>) -> (P<ast::Pat>, P<ast::Expr>) {
    let Field(key, value) = field;

    match value {
        Value::SingleValue(ident) => {
            let pat = cx.pat_ident(ident.span, ident.node);

            let e = util::field_get(parent, &key, false);

            (pat, e)
        },
        Value::MessageValue(msg) => {
            let i_msg = Ident::with_empty_ctxt(Symbol::gensym("msg"));
            let e_msg = cx.expr_ident(sp, i_msg);

            let e = util::field_get(parent, &key, false);

            let (pat, value) = emit_message(cx, sp, msg, e_msg);

            let stmts = vec![
                cx.stmt_let(sp, false, i_msg, e),
                cx.stmt_expr(value),
            ];

            let block = cx.block(sp, stmts);

            (pat, cx.expr_block(block))
        },
        Value::RepeatedValue(_) => {
            panic!("protobuf_bind! does not support repeated fields");
        }
    }
}

fn emit_message(cx: &mut ExtCtxt, sp: Span, msg: Message<Spanned<ast::Ident>>, expr: P<ast::Expr>) -> (P<ast::Pat>, P<ast::Expr>) {
    let mut pats = Vec::new();
    let mut values = Vec::new();

    let Message(fields) = msg;
    for f in fields {
        let (pat, value) = emit_field(cx, sp, f, expr.clone());
        pats.push(pat);
        values.push(value);
    }

    (cx.pat_tuple(sp, pats), cx.expr_tuple(sp, values))
}

pub fn macro_protobuf_bind<'a>(cx: &'a mut ExtCtxt,
                               sp: Span,
                               tts: &[TokenTree]) -> Box<MacResult+'a> {
    match parse_protobuf(cx, tts) {
        Ok((expr, msg)) => {
            let (pat, value) = emit_message(cx, sp, msg, expr);
            MacEager::stmts(SmallVector::one(
                StmtBuilder::new().build_let(pat, None, Some(value), vec![])
            ))
        }
        Err(_) => {
            DummyResult::any(sp)
        }
    }
}


