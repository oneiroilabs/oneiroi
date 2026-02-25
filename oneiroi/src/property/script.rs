use serde::{Deserialize, Serialize};
mod ast;

use crate::{
    asset::{NodeIndex, editable::EditableAsset},
    type_system::{
        OwnedDataType,
        data_types::{DataType, DataTypeKind},
    },
};

use chumsky::prelude::*;

#[derive(Debug)]
enum Expr<'a> {
    Num(f64),
    Var(&'a str),

    Neg(Box<Expr<'a>>),
    Add(Box<Expr<'a>>, Box<Expr<'a>>),
    Sub(Box<Expr<'a>>, Box<Expr<'a>>),
    Mul(Box<Expr<'a>>, Box<Expr<'a>>),
    Div(Box<Expr<'a>>, Box<Expr<'a>>),

    Call(&'a str, Vec<Expr<'a>>),
    Let {
        name: &'a str,
        rhs: Box<Expr<'a>>,
        then: Box<Expr<'a>>,
    },
    Fn {
        name: &'a str,
        args: Vec<&'a str>,
        body: Box<Expr<'a>>,
        then: Box<Expr<'a>>,
    },
}

fn parser<'a>() -> impl Parser<'a, &'a str, Expr<'a>> {
    let ident = text::ascii::ident().padded();

    let expr = recursive(|expr| {
        let int = text::int(10).map(|s: &str| Expr::Num(s.parse().unwrap()));

        let call = ident
            .then(
                expr.clone()
                    .separated_by(just(','))
                    .allow_trailing() // Foo is Rust-like, so allow trailing commas to appear in arg lists
                    .collect::<Vec<_>>()
                    .delimited_by(just('('), just(')')),
            )
            .map(|(f, args)| Expr::Call(f, args));

        let atom = int
            .or(expr.delimited_by(just('('), just(')')))
            .or(call)
            .or(ident.map(Expr::Var))
            .padded();
        let op = |c| just(c).padded();

        let unary = op('-')
            .repeated()
            .foldr(atom, |_op, rhs| Expr::Neg(Box::new(rhs)));

        let product = unary.clone().foldl(
            choice((
                op('*').to(Expr::Mul as fn(_, _) -> _),
                op('/').to(Expr::Div as fn(_, _) -> _),
            ))
            .then(unary)
            .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        );

        let sum = product.clone().foldl(
            choice((
                op('+').to(Expr::Add as fn(_, _) -> _),
                op('-').to(Expr::Sub as fn(_, _) -> _),
            ))
            .then(product)
            .repeated(),
            |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
        );

        sum
    });

    let decl = recursive(|decl| {
        let r#let = text::ascii::keyword("let")
            .ignore_then(ident)
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl.clone())
            .map(|((name, rhs), then)| Expr::Let {
                name,
                rhs: Box::new(rhs),
                then: Box::new(then),
            });

        let r#fn = text::ascii::keyword("fn")
            .ignore_then(ident)
            .then(ident.repeated().collect::<Vec<_>>())
            .then_ignore(just('='))
            .then(expr.clone())
            .then_ignore(just(';'))
            .then(decl)
            .map(|(((name, args), body), then)| Expr::Fn {
                name,
                args,
                body: Box::new(body),
                then: Box::new(then),
            });

        r#let.or(r#fn).or(expr).padded()
    });

    decl
}

fn eval<'a>(
    expr: &'a Expr<'a>,
    vars: &mut Vec<(&'a str, f64)>,
    funcs: &mut Vec<(&'a str, &'a [&'a str], &'a Expr<'a>)>,
) -> Result<f64, String> {
    match expr {
        Expr::Num(x) => Ok(*x),
        Expr::Neg(a) => Ok(-eval(a, vars, funcs)?),
        Expr::Add(a, b) => Ok(eval(a, vars, funcs)? + eval(b, vars, funcs)?),
        Expr::Sub(a, b) => Ok(eval(a, vars, funcs)? - eval(b, vars, funcs)?),
        Expr::Mul(a, b) => Ok(eval(a, vars, funcs)? * eval(b, vars, funcs)?),
        Expr::Div(a, b) => Ok(eval(a, vars, funcs)? / eval(b, vars, funcs)?),
        Expr::Var(name) => {
            if let Some((_, val)) = vars.iter().rev().find(|(var, _)| var == name) {
                Ok(*val)
            } else {
                Err(format!("Cannot find variable `{}` in scope", name))
            }
        }
        Expr::Let { name, rhs, then } => {
            let rhs = eval(rhs, vars, funcs)?;
            vars.push((*name, rhs));
            let output = eval(then, vars, funcs);
            vars.pop();
            output
        }
        Expr::Call(name, args) => {
            if let Some((_, arg_names, body)) =
                funcs.iter().rev().find(|(var, _, _)| var == name).copied()
            {
                if arg_names.len() == args.len() {
                    let mut args = args
                        .iter()
                        .map(|arg| eval(arg, vars, funcs))
                        .zip(arg_names.iter())
                        .map(|(val, name)| Ok((*name, val?)))
                        .collect::<Result<_, String>>()?;
                    let old_vars = vars.len();
                    vars.append(&mut args);
                    let output = eval(body, vars, funcs);
                    vars.truncate(old_vars);
                    output
                } else {
                    Err(format!(
                        "Wrong number of arguments for function `{}`: expected {}, found {}",
                        name,
                        arg_names.len(),
                        args.len(),
                    ))
                }
            } else {
                Err(format!("Cannot find function `{}` in scope", name))
            }
        }
        Expr::Fn {
            name,
            args,
            body,
            then,
        } => {
            funcs.push((name, args, body));
            let output = eval(then, vars, funcs);
            funcs.pop();
            output
        }
    }
}

//TODO remove default
/* #[derive(Debug, Default, Clone)]
struct ParserCache<T: DataType> {
    origin: NodeIndex,
    target: NodeIndex,
    //info: Option<PropertyInfo>,
    value: T,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UninitializedOneiroiScript<T: DataType> {
    literal: String,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OneiroiScript<T: DataType> {
    literal: String,
    #[serde(skip)]
    parsed_tree: ParserCache<T>,
}
impl<T: DataType> OneiroiScript<T> {
    /* pub fn get_default_value(&self) -> DataTypeValue {
        //TODO actually get the thing
        /* match T::DATA_TYPE_TYPE {
            DataTypeType::Omni => todo!(),
            DataTypeType::Mesh => todo!(),
            DataTypeType::Collection => todo!(),
            DataTypeType::Instance => todo!(),
            DataTypeType::Curve => todo!(),
            DataTypeType::Collider => todo!(),
            DataTypeType::Vec3 => DataTypeValue::Vec3(Vec3::new(1.0, 1.0, 1.0)),
            DataTypeType::Int => todo!(),
            DataTypeType::Float => DataTypeValue::Float(2.0),
            DataTypeType::Bool => todo!(),
            DataTypeType::Selection => todo!(),
            DataTypeType::Material => todo!(),
            DataTypeType::CubicBezier => todo!(),
            DataTypeType::Transform => todo!(),
        } */
        panic!()
    } */

    pub fn set_new_value(&mut self, value: T) {
        self.parsed_tree.value = value
    }

    pub fn get_value(&self) -> &T {
        &self.parsed_tree.value
    }

    pub fn get_string(&self) -> String {
        self.literal.clone()
    }
} */

#[derive(Debug)]
pub struct OneiroiScriptParserError {}

//The script type gets created from a string and always gets evaluated
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Script {
    resolver_type: DataTypeKind,
    content: String,
    //ast: Ast,
    // evaluated_value: DataTypeValue,
}

impl Script {
    pub fn check(
        node_index: NodeIndex, //TODO maybe remove
        property: u8,          //TODO maybe remove
        resolver_type: DataTypeKind,
        script: &str,
        context: &mut EditableAsset,
    ) -> Result<Self, OneiroiScriptParserError> {
        Ok(Script {
            resolver_type,
            content: String::new(),
            //ast: Ast::TODO,
            //evaluated_value:
        })
    }

    // this can maybe even be immutable when we guarantee
    fn evaluate(&mut self, context: &EditableAsset) -> OwnedDataType {
        OwnedDataType::Float(5.0)
    }

    pub(crate) fn get_content(&self) -> &str {
        &self.content
    }
}
