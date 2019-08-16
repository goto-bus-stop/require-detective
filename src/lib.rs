use resast::prelude::*;
use ressa::Parser;
#[cfg(feature = "npm")]
use serde_derive::Serialize;
#[cfg(feature = "npm")]
use wasm_bindgen::prelude::*;

pub use ressa::Error;

#[cfg_attr(feature = "npm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct Options {
    word: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            word: "require".to_string(),
        }
    }
}

#[cfg_attr(feature = "npm", wasm_bindgen)]
impl Options {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn word(mut self, word: &str) -> Self {
        self.word = word.to_string();
        self
    }
}

#[cfg_attr(feature = "npm", derive(Debug, Default, Clone, Serialize))]
#[cfg_attr(not(feature = "npm"), derive(Debug, Default, Clone))]
pub struct Found {
    strings: Vec<String>,
    expressions: Vec<String>,
}

#[derive(Debug)]
struct Detective<'a> {
    options: &'a Options,
    found: Found,
}

impl<'a> Detective<'a> {
    fn new(options: &'a Options) -> Self {
        Self {
            options,
            found: Default::default(),
        }
    }

    fn check(&mut self, callee: &Expr<'_>, args: &[Expr<'_>]) -> bool {
        if let Expr::Ident(ref ident) = callee {
            if ident.name == self.options.word {
                match args.get(0) {
                    Some(Expr::Lit(Lit::String(string))) => {
                        self.found.strings.push(string.clone_inner().to_string());
                        return true;
                    }
                    Some(Expr::Lit(Lit::Template(template))) if template.expressions.is_empty() => {
                        self.found
                            .strings
                            .push(template.quasis[0].cooked.to_string());
                        return true;
                    }
                    Some(_expr) => {
                        // somehow get the text for this
                        // self.found.expressions.push()
                        return true;
                    }
                    _ => (),
                }
            }
        }
        false
    }

    fn oncall(&mut self, call: &CallExpr<'_>) {
        if self.check(&*call.callee, &call.arguments) {
            return;
        }

        self.onexpr(&*call.callee);
        for arg in &call.arguments {
            self.onexpr(arg);
        }
    }

    fn ontemplate(&mut self, tpl: &TemplateLit<'_>) {
        for expr in &tpl.expressions {
            self.onexpr(expr);
        }
    }

    fn onprop(&mut self, prop: &Prop) {
        match &prop.key {
            PropKey::Expr(expr) => self.onexpr(expr),
            PropKey::Pat(pat) => self.onpat(&pat),
            _ => (),
        }
        match &prop.value {
            PropValue::Expr(expr) => self.onexpr(expr),
            PropValue::Pat(pat) => self.onpat(&pat),
            _ => (),
        }
    }

    fn onparams(&mut self, params: &[FuncArg<'_>]) {
        for param in params {
            match param {
                FuncArg::Expr(expr) => self.onexpr(expr),
                FuncArg::Pat(pat) => self.onpat(&pat),
            }
        }
    }

    fn onclass(&mut self, class: &Class<'_>) {
        if let Some(super_class) = &class.super_class {
            self.onexpr(&*super_class);
        }
        for prop in class.body.0.iter() {
            self.onprop(prop);
        }
    }

    fn onexpr(&mut self, expr: &Expr<'_>) {
        match expr {
            Expr::Array(elements) => {
                for element in elements {
                    if let Some(el) = element {
                        self.onexpr(el);
                    }
                }
            }
            Expr::ArrowFunc(arrow) => {
                self.onparams(&arrow.params);
                match &arrow.body {
                    ArrowFuncBody::FuncBody(body) => self.onbody(&body.0),
                    ArrowFuncBody::Expr(expr) => self.onexpr(&*expr),
                }
            }
            Expr::Assign(assign) => {
                match &assign.left {
                    AssignLeft::Pat(pat) => self.onpat(&pat),
                    AssignLeft::Expr(expr) => self.onexpr(&*expr),
                }
                self.onexpr(&*assign.right);
            }
            Expr::Await(expr) => self.onexpr(&*expr),
            Expr::Binary(binary) => {
                self.onexpr(&*binary.left);
                self.onexpr(&*binary.right);
            }
            Expr::Class(class) => self.onclass(class),
            Expr::Call(call) => self.oncall(call),
            Expr::Conditional(cond) => {
                self.onexpr(&*cond.test);
                self.onexpr(&*cond.consequent);
                self.onexpr(&*cond.alternate);
            }
            Expr::Func(func) => {
                self.onparams(&func.params);
                self.onbody(&func.body.0);
            }
            Expr::Logical(op) => {
                self.onexpr(&*op.left);
                self.onexpr(&*op.right);
            }
            Expr::Member(member) => {
                self.onexpr(&*member.object);
                self.onexpr(&*member.property);
            }
            Expr::New(new) => {
                if self.check(&*new.callee, &new.arguments) {
                    return;
                }
                self.onexpr(&*new.callee);
                for arg in &new.arguments {
                    self.onexpr(arg);
                }
            }
            Expr::Obj(obj) => {
                for prop in obj.iter() {
                    match prop {
                        ObjProp::Prop(prop) => self.onprop(prop),
                        ObjProp::Spread(expr) => self.onexpr(expr),
                    }
                }
            }
            Expr::Sequence(seq) => {
                for expr in seq {
                    self.onexpr(expr);
                }
            }
            Expr::Spread(expr) => self.onexpr(&*expr),
            Expr::TaggedTemplate(template) => {
                self.onexpr(&*template.tag);
                self.ontemplate(&template.quasi);
            }
            Expr::Unary(expr) => self.onexpr(&*expr.argument),
            Expr::Update(expr) => self.onexpr(&*expr.argument),
            Expr::Yield(expr) if expr.argument.is_some() => {
                self.onexpr(&*expr.argument.as_ref().unwrap())
            }
            _ => (),
        }
    }

    fn onpat(&mut self, pat: &Pat<'_>) {
        match pat {
            Pat::Obj(obj) => {
                for part in obj.iter() {
                    match part {
                        ObjPatPart::Assign(prop) => self.onprop(prop),
                        ObjPatPart::Rest(pat) => self.onpat(&*pat),
                    }
                }
            }
            Pat::Array(array) => {
                for part in array.iter() {
                    match part {
                        Some(ArrayPatPart::Pat(pat)) => self.onpat(pat),
                        Some(ArrayPatPart::Expr(expr)) => self.onexpr(expr),
                        _ => (),
                    }
                }
            }
            Pat::RestElement(rest) => self.onpat(&*rest),
            Pat::Assign(assign) => {
                self.onpat(&*assign.left);
                self.onexpr(&*assign.right);
            }
            _ => (),
        }
    }

    fn onvar(&mut self, decls: &[VarDecl]) {
        for decl in decls.iter() {
            self.onpat(&decl.id);
            if let Some(init) = &decl.init {
                self.onexpr(init);
            }
        }
    }

    fn ondecl(&mut self, decl: &Decl) {
        match decl {
            Decl::Var(_, decls) => self.onvar(&decls),
            Decl::Func(func) => {
                self.onparams(&func.params);
                self.onbody(&func.body.0);
            }
            Decl::Class(class) => self.onclass(class),
            Decl::Import(_) => (),
            Decl::Export(export) => match &**export {
                ModExport::Default(DefaultExportDecl::Decl(decl)) => self.ondecl(decl),
                ModExport::Default(DefaultExportDecl::Expr(expr)) => self.onexpr(expr),
                ModExport::Named(NamedExportDecl::Decl(decl)) => self.ondecl(decl),
                _ => (),
            },
        };
    }

    fn onloopleft(&mut self, left: &LoopLeft<'_>) {
        match &left {
            LoopLeft::Expr(expr) => self.onexpr(expr),
            LoopLeft::Variable(_, decl) => {
                self.onpat(&decl.id);
                if let Some(init) = &decl.init {
                    self.onexpr(init);
                }
            }
            LoopLeft::Pat(pat) => self.onpat(pat),
        }
    }

    fn onstmt(&mut self, stmt: &Stmt<'_>) {
        match stmt {
            Stmt::Expr(expr) => self.onexpr(expr),
            Stmt::Block(block) => self.onbody(&block.0),
            Stmt::With(with) => {
                self.onexpr(&with.object);
                self.onstmt(&*with.body);
            }
            Stmt::Return(Some(expr)) => self.onexpr(expr),
            Stmt::Labeled(label) => self.onstmt(&*label.body),
            Stmt::If(stmt) => {
                self.onexpr(&stmt.test);
                self.onstmt(&*stmt.consequent);
                if let Some(alternate) = &stmt.alternate {
                    self.onstmt(&*alternate);
                }
            }
            Stmt::Switch(switch) => {
                self.onexpr(&switch.discriminant);
                for case in &switch.cases {
                    if let Some(expr) = &case.test {
                        self.onexpr(expr);
                    }
                    self.onbody(&case.consequent);
                }
            }
            Stmt::Throw(err) => self.onexpr(err),
            Stmt::Try(stmt) => {
                self.onbody(&stmt.block.0);
                if let Some(catch) = &stmt.handler {
                    if let Some(pat) = &catch.param {
                        self.onpat(pat);
                    }
                    self.onbody(&catch.body.0);
                }
                if let Some(finalizer) = &stmt.finalizer {
                    self.onbody(&finalizer.0);
                }
            }
            Stmt::While(stmt) => {
                self.onexpr(&stmt.test);
                self.onstmt(&*stmt.body);
            }
            Stmt::DoWhile(stmt) => {
                self.onstmt(&*stmt.body);
                self.onexpr(&stmt.test);
            }
            Stmt::For(stmt) => {
                match &stmt.init {
                    Some(LoopInit::Variable(_, decls)) => self.onvar(&decls),
                    Some(LoopInit::Expr(expr)) => self.onexpr(expr),
                    _ => (),
                }
                if let Some(test) = &stmt.test {
                    self.onexpr(test);
                }
                if let Some(update) = &stmt.update {
                    self.onexpr(update);
                }
                self.onstmt(&*stmt.body);
            }
            Stmt::ForIn(stmt) => {
                self.onloopleft(&stmt.left);
                self.onexpr(&stmt.right);
                self.onstmt(&*stmt.body)
            }
            Stmt::ForOf(stmt) => {
                self.onloopleft(&stmt.left);
                self.onexpr(&stmt.right);
                self.onstmt(&*stmt.body)
            }
            Stmt::Var(var) => self.onvar(&var),
            _ => (),
        }
    }

    fn onbody(&mut self, body: &[ProgramPart]) {
        for part in body {
            match part {
                ProgramPart::Decl(decl) => self.ondecl(decl),
                ProgramPart::Stmt(stmt) => self.onstmt(stmt),
                _ => (),
            }
        }
    }

    fn find(mut self, source: &str) -> Result<Found, Error> {
        if !source.contains(&self.options.word) {
            return Ok(self.found);
        }

        let parser = Parser::new(source)?;
        for part in parser {
            match &part? {
                ProgramPart::Decl(decl) => self.ondecl(decl),
                ProgramPart::Stmt(stmt) => self.onstmt(stmt),
                _ => (),
            }
        }

        Ok(self.found)
    }
}

pub fn find(source: &str, options: &Options) -> Result<Found, Error> {
    Detective::new(options).find(source)
}

pub fn detective(source: &str, options: &Options) -> Result<Vec<String>, Error> {
    Detective::new(options).find(source).map(|res| res.strings)
}

#[cfg(garget_arch = "wasm32")]
pub use wasm::*;

#[cfg(feature = "npm")]
mod wasm {
    // use wasm_bindgen::prelude::*;
    use super::*;

    fn convert_err(err: Error) -> JsValue {
        JsValue::from_str(&format!("{}", err))
    }

    #[wasm_bindgen(js_name = "find")]
    pub fn js_find(source: &str, options: Options) -> Result<JsValue, JsValue> {
        find(source, &options)
            .map(|found| JsValue::from_serde(&found).unwrap())
            .map_err(convert_err)
    }

    #[wasm_bindgen(js_name = "detective")]
    pub fn js_detective(source: &str, options: Options) -> Result<JsValue, JsValue> {
        detective(source, &options)
            .map(|list| JsValue::from_serde(&list).unwrap())
            .map_err(convert_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both() {
        let found = find(
            r#"
            require('a');
            require('b');
            require('c' + x);
            var moo = require('d' + y).moo;
        "#,
            &Default::default(),
        )
        .unwrap();

        assert_eq!(found.strings, vec!["a", "b"]);
        assert_eq!(found.expressions, vec!["'c' + x", "'d' + y"]);
    }

    #[test]
    fn chained() {
        let found = find(
            r#"
            require('c').hello().goodbye()
            require('b').hello()
            require('a')
        "#,
            &Default::default(),
        )
        .unwrap();

        assert_eq!(found.strings, vec!["c", "b", "a"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn complicated() {
        let sources = [
            "require(\"a\")",
            "require('a')",
            "require(`a`)",
            ";require(\"a\")",
            " require(\"a\")",
            "void require(\"a\")",
            "+require(\"a\")",
            "!require(\"a\")",
            "/*comments*/require(\"a\")",
            "(require(\"a\"))",
            "require/*comments*/(\"a\")",
            ";require/*comments*/(\"a\")",
            " require/*comments*/(\"a\")",
            "void require/*comments*/(\"a\")",
            "+require/*comments*/(\"a\")",
            "!require/*comments*/(\"a\")",
            "/*comments*/require/*comments*/(\"a\")",
            "(require/*comments*/(\"a\"))",
            "require /*comments*/ (\"a\")",
            ";require /*comments*/ (\"a\")",
            " require /*comments*/ (\"a\")",
            "void require /*comments*/ (\"a\")",
            "+require /*comments*/ (\"a\")",
            "!require /*comments*/ (\"a\")",
            " /*comments*/ require /*comments*/ (\"a\")",
            "(require /*comments*/ (\"a\"))",
            "require /*comments*/ /*more comments*/ (\"a\")",
            ";require /*comments*/ /*more comments*/ (\"a\")",
            " require /*comments*/ /*more comments*/ (\"a\")",
            "void require /*comments*/ /*more comments*/ (\"a\")",
            "+require /*comments*/ /*more comments*/ (\"a\")",
            "!require /*comments*/ /*more comments*/ (\"a\")",
            " /*comments*/ /*more comments*/ require /*comments*/ /*more comments*/ (\"a\")",
            "(require /*comments*/ /*more comments*/ (\"a\"))",
            "require//comments\n(\"a\")",
            ";require//comments\n(\"a\")",
            " require//comments\n(\"a\")",
            "void require//comments\n(\"a\")",
            "+require//comments\n(\"a\")",
            "!require//comments\n(\"a\")",
            "  require//comments\n(\"a\")",
            "(require//comments\n(\"a\"))",
        ];

        for source in sources.iter() {
            let found = find(source, &Default::default()).unwrap();
            assert_eq!(found.strings, vec!["a"]);
            assert!(found.expressions.is_empty());
        }
    }

    #[test]
    fn for_await() {
        let found = find(
            r#"
            async function main () {
                for await (const _ of (async function* () {})()) {
                    require(_)
                }
            }
        "#,
            &Default::default(),
        )
        .unwrap();
        assert!(found.strings.is_empty());
        assert_eq!(found.expressions, vec!["_"]);
    }

    #[test]
    fn optional_catch() {
        let found = find(
            r#"
            try {
                require;
            } catch {
            }
        "#,
            &Default::default(),
        )
        .unwrap();
        assert!(found.strings.is_empty());
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn es_module() {
        let found = find(
            r#"
            var a = require('a');

            export default function () {
                var b = require('b');
            }
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "b"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn generators() {
        let found = find(
            r#"
            var a = require('a');

            function *gen() {
              yield require('b');
            }
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "b"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn nested() {
        let found = find(
            r#"
            if (true) {
                (function () {
                    require('a');
                })();
            }
            if (false) {
                (function () {
                    var x = 10;
                    switch (x) {
                        case 1 : require('b'); break;
                        default : break;
                    }
                })()
            }

            function qqq () {
                require
                    (
                    "c"
                );
            }
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "b", "c"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn rest_spread() {
        let found = find(
            r#"
            var a = require('a');
            var b = require('b');
            var c = require('c');


            var obj = { foo: 'bar', bee: 'bop' }
            var spread = { ...obj }
            var { foo, ...rest } = obj
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "b", "c"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn top_level_return() {
        let found = find(
            r#"
            var a = require('a');

            return
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn set_in_object_pat() {
        let found = find(
            r#"
            var a = load('a');
            var b = load('b');
            var c = load('c');
            var abc = a.b(c);

            function load2({set = 'hello'}) {
                return load('tt');
            }

            var loadUse = load2();
        "#,
            &Options::new().word("load"),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "b", "c", "tt"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn shebang() {
        let found = find(
            r#"
            #!/usr/bin/env node
            var a = require('a');
            var b = require('b');
            var c = require('c');
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "b", "c"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn sparse_array() {
        let found = find(
            r#"
            var o = [,,,,]

            require('./foo')
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["./foo"]);
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn strings() {
        let found = find(
            r#"
            var a = require('a');
            var b = require('b');
            var c = require('c');
            var abc = a.b(c);

            var EventEmitter = require('events').EventEmitter;

            var x = require('doom')(5,6,7);
            x(8,9);
            c.require('notthis');
            var y = require('y') * 100;

            var EventEmitter2 = require('events2').EventEmitter();
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(
            found.strings,
            vec!["a", "b", "c", "events", "doom", "y", "events2"]
        );
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn word() {
        let found = find(
            r#"
            var a = load('a');
            var b = load('b');
            var c = load('c');
            var abc = a.b(c);

            var EventEmitter = load('events').EventEmitter;

            var x = load('doom')(5,6,7);
            x(8,9);
            c.load('notthis');
            var y = load('y') * 100;

            var EventEmitter2 = load('events2').EventEmitter();
        "#,
            &Options::new().word("load"),
        )
        .unwrap();
        assert_eq!(
            found.strings,
            vec!["a", "b", "c", "events", "doom", "y", "events2"]
        );
        assert!(found.expressions.is_empty());
    }

    #[test]
    fn yield_() {
        let found = find(
            r#"
            (function * () {
                var a = require('a');
                var b = yield require('c')(a);
            })();
        "#,
            &Default::default(),
        )
        .unwrap();
        assert_eq!(found.strings, vec!["a", "c"]);
        assert!(found.expressions.is_empty());
    }
}
