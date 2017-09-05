use dart::parse;
use enum_primitive::FromPrimitive;
use node::Node;
use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::path::{Path, PathBuf};
use syntax::symbol::Symbol;
use syntax::codemap::Span;

#[derive(Debug)]
pub struct Module {
    pub path: PathBuf,
    pub items: Vec<Node<Item>>,
    pub has_error: bool,
}

impl Module {
    pub fn load(path: &Path) -> Node<Module> {
        thread_local!(static CACHE: RefCell<HashMap<PathBuf, Node<Module>>> =
            RefCell::new(HashMap::new()));

        let path_buf;
        let mut path = path;
        let module = do catch {
            path_buf = path.canonicalize()?;
            path = &path_buf;
            if let Some(module) = CACHE.with(|c| c.borrow().get(path).cloned()) {
                return module;
            }
            parse::Parser::with_file(path, |p| p.dart_module())
        };
        let module = match module {
            Ok(module) => module,
            Err(error) => {
                if let parse::ErrorKind::Io(_) = *error.kind() {
                    println!("{}: {}", path.display(), error);
                } else {
                    println!("{}", error);
                }
                Node::new(Module {
                    path: path.to_path_buf(),
                    items: vec![],
                    has_error: true,
                })
            }
        };
        CACHE.with(|c| {
            c.borrow_mut().insert(module.path.clone(), module.clone())
        });
        module
    }
}

#[derive(Debug)]
pub enum Item {
    LibraryName {
        metadata: Metadata,
        path: Vec<Symbol>,
    },
    Import(Metadata, Import),
    Export(Metadata, StringLiteral, Vec<ImportFilter>),
    Part {
        metadata: Metadata,
        uri: StringLiteral,
        module: Node<Module>,
    },
    PartOf {
        metadata: Metadata,
        path: Vec<Symbol>,
    },
    Class {
        metadata: Metadata,
        abstract_: bool,
        name: Symbol,
        generics: Vec<Node<TypeParameter>>,
        superclass: Option<Node<Qualified>>,
        mixins: Vec<Node<Qualified>>,
        interfaces: Vec<Node<Qualified>>,
        members: Vec<Node<ClassMember>>,
    },
    MixinClass {
        metadata: Metadata,
        abstract_: bool,
        name: Symbol,
        generics: Vec<Node<TypeParameter>>,
        mixins: Vec<Node<Qualified>>,
        interfaces: Vec<Node<Qualified>>,
    },
    Enum {
        metadata: Metadata,
        name: Symbol,
        values: Vec<Symbol>,
    },
    TypeAlias {
        metadata: Metadata,
        name: Symbol,
        generics: Vec<Node<TypeParameter>>,
        ty: Node<Type>,
    },
    Function(Node<Function>),
    Vars(VarType, Vec<Node<VarDef>>),
}

#[derive(Debug)]
pub struct ImportFilter {
    pub hide: bool,
    pub names: Vec<Symbol>,
}

#[derive(Debug)]
pub struct Import {
    pub uri: StringLiteral,
    pub deferred: bool,
    pub as_ident: Option<Symbol>,
    pub filters: Vec<ImportFilter>,
}

#[derive(Debug)]
pub struct Function {
    pub name: FnName,
    pub generics: Vec<Node<TypeParameter>>,
    pub sig: FnSig,
    pub body: Option<FnBody>,
}

#[derive(Debug)]
pub enum ClassMember {
    Redirect {
        metadata: Metadata,
        method_qualifiers: Vec<MethodQualifiers>,
        name: Option<Symbol>,
        sig: FnSig,
        ty: Node<Type>,
    },
    Constructor {
        metadata: Metadata,
        method_qualifiers: Vec<MethodQualifiers>,
        name: Option<Symbol>,
        sig: FnSig,
        initializers: Vec<ConstructorInitializer>,
        function_body: Option<FnBody>,
    },
    Method(Metadata, Vec<MethodQualifiers>, Node<Function>),
    Fields {
        metadata: Metadata,
        static_: bool,
        var_type: VarType,
        initializers: Vec<Node<VarDef>>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MethodQualifiers {
    External,
    Static,
    Const,
    Final,
    Factory,
}

#[derive(Copy, Clone, Debug)]
pub enum FnName {
    Regular(Symbol),
    Getter(Symbol),
    Setter(Symbol),
    Operator(OverloadedOp),
}

#[derive(Copy, Clone, Debug)]
pub enum OverloadedOp {
    BitNot,
    Index,
    IndexAssign,
    Bool(BoolBinOp),
    Value(ValueBinOp),
}

#[derive(Debug)]
pub enum ConstructorInitializer {
    Super(Option<Symbol>, Args),
    This(Option<Symbol>, Args),
    Assert(Args),
    Field(bool, Symbol, Node<Expr>),
}

#[derive(Debug)]
pub struct TypeParameter {
    pub metadata: Metadata,
    pub name: Symbol,
    pub extends: Option<Node<Qualified>>,
}

#[derive(Debug)]
pub struct Qualified {
    pub prefix: Option<Node<Qualified>>,
    pub name: Symbol,
    pub params: Vec<Node<Type>>,
}

impl Qualified {
    pub fn one(name: &str, params: Vec<Node<Type>>) -> Node<Qualified> {
        Node::new(Qualified {
            prefix: None,
            name: Symbol::intern(name),
            params,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
    Await,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
}

enum_from_primitive! {
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BoolBinOp {
    Or,
    And,
    Eq,
    Ne,
    Ge,
    Gt,
    Le,
    Lt,
}
}

impl BoolBinOp {
    pub fn values() -> impl Iterator<Item = Self> {
        (0..)
            .map(Self::from_usize)
            .take_while(|x| x.is_some())
            .map(|x| x.unwrap())
    }
}

enum_from_primitive! {
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum ValueBinOp {
        IfNull,
        Add,
        Sub,
        Mul,
        Div,
        Mod,
        TruncDiv,
        Lsh,
        Rsh,
        BitAnd,
        BitXor,
        BitOr,
    }
}

impl ValueBinOp {
    pub fn values() -> impl Iterator<Item = Self> {
        (0..)
            .map(Self::from_usize)
            .take_while(|x| x.is_some())
            .map(|x| x.unwrap())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BinOp {
    Bool(BoolBinOp),
    Value(ValueBinOp),
    Assign(Option<ValueBinOp>),
}

impl BinOp {
    pub fn values() -> impl Iterator<Item = Self> {
        (BoolBinOp::values().map(BinOp::Bool))
            .chain(ValueBinOp::values().map(BinOp::Value))
            .chain(
                ValueBinOp::values()
                    .map(Some)
                    .chain(iter::once(None))
                    .map(BinOp::Assign),
            )
    }
}

impl BinOp {
    pub fn as_str(self) -> &'static str {
        macro_rules! value_bin_op {
            ($op:expr, $suffix:expr) => (match $op {
                ValueBinOp::IfNull => concat!("??", $suffix),
                ValueBinOp::Add => concat!("+", $suffix),
                ValueBinOp::Sub => concat!("-", $suffix),
                ValueBinOp::Mul => concat!("*", $suffix),
                ValueBinOp::Div => concat!("/", $suffix),
                ValueBinOp::Mod => concat!("%", $suffix),
                ValueBinOp::TruncDiv => concat!("~/", $suffix),
                ValueBinOp::Lsh => concat!("<<", $suffix),
                ValueBinOp::Rsh => concat!(">>", $suffix),
                ValueBinOp::BitAnd => concat!("&", $suffix),
                ValueBinOp::BitXor => concat!("^", $suffix),
                ValueBinOp::BitOr => concat!("|", $suffix),
            })
        }
        match self {
            BinOp::Bool(BoolBinOp::Or) => "||",
            BinOp::Bool(BoolBinOp::And) => "&&",
            BinOp::Bool(BoolBinOp::Eq) => "==",
            BinOp::Bool(BoolBinOp::Ne) => "!=",
            BinOp::Bool(BoolBinOp::Ge) => ">=",
            BinOp::Bool(BoolBinOp::Gt) => ">",
            BinOp::Bool(BoolBinOp::Le) => "<=",
            BinOp::Bool(BoolBinOp::Lt) => "<",

            BinOp::Value(op) => value_bin_op!(op, ""),

            BinOp::Assign(Some(op)) => value_bin_op!(op, "="),
            BinOp::Assign(None) => "=",
        }
    }
}

#[derive(Debug)]
pub enum Type {
    Path(Node<Qualified>),
    FunctionOld(FnSig),
    Function(FnSig),
    Infer,
}

impl Type {
    pub fn simple_path(name: &str) -> Node<Type> {
        Node::new(Type::Path(Qualified::one(name, vec![])))
    }
}

#[derive(Debug)]
pub enum Expr {
    Unary(UnOp, Node<Expr>),
    Binary(BinOp, Node<Expr>, Node<Expr>),
    Conditional(Node<Expr>, Node<Expr>, Node<Expr>),
    Is(Node<Expr>, Node<Type>),
    IsNot(Node<Expr>, Node<Type>),
    As(Node<Expr>, Node<Type>),
    Suffix(Node<Expr>, Suffix),
    Identifier(Symbol),
    Closure(FnSig, FnBody),
    New {
        const_: bool,
        path: Node<Qualified>,
        args: Args,
    },
    List {
        const_: bool,
        element_ty: Option<Node<Type>>,
        elements: Vec<Node<Expr>>,
    },
    Map {
        const_: bool,
        kv_ty: Option<(Node<Type>, Node<Type>)>,
        kv: Vec<(Node<Expr>, Node<Expr>)>,
    },
    Number(Symbol),
    String(Vec<StringLiteral>),
    Symbol(SymbolLiteral),
    Paren(Node<Expr>),
    Throw(Node<Expr>),
    Cascade(Node<Expr>, Cascade),
}

#[derive(Debug)]
pub enum SymbolLiteral {
    Op(OverloadedOp),
    Path(Vec<Symbol>),
}

#[derive(Debug)]
pub struct StringLiteral {
    pub raw: bool,
    pub triple: bool,
    pub quote: char,
    pub prefix: Span,
    pub interpolated: Vec<(Node<Expr>, Span)>,
}

impl StringLiteral {
    pub fn get_simple_string(&self) -> String {
        assert!(self.interpolated.is_empty());
        ::codemap().span_to_snippet(self.prefix).unwrap()
    }
}

#[derive(Debug)]
pub enum Suffix {
    Index(Node<Expr>),
    Field(Symbol),
    FieldIfNotNull(Symbol),
    Call(Vec<Node<Type>>, Args),
}

#[derive(Debug)]
pub struct Cascade {
    pub suffixes: Vec<Suffix>,
    pub assign: Option<(Option<ValueBinOp>, Node<Expr>)>,
}

#[derive(Debug)]
pub struct NamedArg {
    pub name: Symbol,
    pub expr: Node<Expr>,
}

#[derive(Debug)]
pub struct Args {
    pub unnamed: Vec<Node<Expr>>,
    pub named: Vec<NamedArg>,
}

#[derive(Debug)]
pub struct MetadataItem {
    pub qualified: Node<Qualified>,
    pub arguments: Option<Args>,
}

impl MetadataItem {
    pub fn simple(name: &str) -> MetadataItem {
        MetadataItem {
            qualified: Qualified::one(name, vec![]),
            arguments: None,
        }
    }
}

pub type Metadata = Vec<MetadataItem>;

#[derive(Debug)]
pub struct FnSig {
    pub return_type: Node<Type>,
    pub required: Vec<ArgDef>,
    pub optional: Vec<ArgDef>,
    pub optional_kind: OptionalArgKind,
    pub async: bool,
    pub generator: bool,
}

impl Default for FnSig {
    fn default() -> Self {
        FnSig {
            return_type: Node::new(Type::Infer),
            required: vec![],
            optional: vec![],
            optional_kind: OptionalArgKind::default(),
            async: false,
            generator: false,
        }
    }
}

#[derive(Debug)]
pub enum FnBody {
    Arrow(Node<Expr>),
    Block(Node<Statement>),
    Native(Option<StringLiteral>),
}

#[derive(PartialEq, Eq, Debug)]
pub enum OptionalArgKind {
    Positional,
    Named,
}

impl Default for OptionalArgKind {
    fn default() -> Self {
        OptionalArgKind::Positional
    }
}

#[derive(Debug)]
pub struct ArgDef {
    pub metadata: Metadata,
    pub covariant: bool,
    pub ty: VarType,
    pub field: bool,
    pub var: Node<VarDef>,
}

impl ArgDef {
    pub fn simple(ty: Node<Type>, name: &str) -> ArgDef {
        ArgDef {
            metadata: vec![],
            covariant: false,
            ty: VarType {
                fcv: FinalConstVar::Var,
                ty,
            },
            field: false,
            var: Node::new(VarDef {
                name: Symbol::intern(name),
                init: None,
            }),
        }
    }
}

#[derive(Debug)]
pub struct VarType {
    pub fcv: FinalConstVar,
    pub ty: Node<Type>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum FinalConstVar {
    Final,
    Const,
    Var,
}

#[derive(Debug)]
pub struct VarDef {
    pub name: Symbol,
    pub init: Option<Node<Expr>>,
}

#[derive(Debug)]
pub enum ForLoop {
    CLike(Node<Statement>, Option<Node<Expr>>, Vec<Node<Expr>>),
    In(Symbol, Node<Expr>),
    InVar(VarType, Node<VarDef>, Node<Expr>),
}

#[derive(Debug)]
pub struct SwitchCase {
    pub labels: Vec<Symbol>,
    pub value: Option<Node<Expr>>,
    pub statements: Vec<Node<Statement>>,
}

#[derive(Debug)]
pub struct CatchPart {
    pub exception: Symbol,
    pub trace: Option<Symbol>,
}

#[derive(Debug)]
pub struct TryPart {
    pub on: Option<Node<Type>>,
    pub catch: Option<CatchPart>,
    pub block: Node<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Block(Vec<Node<Statement>>),
    Vars(VarType, Vec<Node<VarDef>>),
    Function(Node<Function>),
    For(bool, ForLoop, Node<Statement>),
    While(Node<Expr>, Node<Statement>),
    DoWhile(Node<Statement>, Node<Expr>),
    Switch(Node<Expr>, Vec<SwitchCase>),
    If(Node<Expr>, Node<Statement>, Option<Node<Statement>>),
    Rethrow,
    Try(Node<Statement>, Vec<TryPart>),
    Break(Option<Symbol>),
    Continue(Option<Symbol>),
    Return(Option<Node<Expr>>),
    Yield(Node<Expr>),
    YieldEach(Node<Expr>),
    Expression(Option<Node<Expr>>),
    Assert(Args),
    Labelled(Symbol, Node<Statement>),
}
