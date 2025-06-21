use std::fmt;
use chumsky::prelude::*;

pub type Span = SimpleSpan<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Delim { Paren, Brack, Brace }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token<'a> {
    Int(i64), Float(f64), Str(&'a str),
    Sym(&'a str),
    Open(Delim), Close(Delim),
    Nil, Quote,
}

impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use { Token::*, Delim::* };
        match self {
            Int(n)     => write!(f, "{}", n),
            Float(x)   => write!(f, "{}", x),
            Str(s)     => write!(f, "\"{}\"", s),
            Sym(s)     => write!(f, "{}", s),
            Open(d)    => write!(f, "{}",
                match d { Paren => "(", Brack => "[", Brace => "{" }
            ),
            Close(d)   => write!(f, "{}",
                match d { Paren => ")", Brack => "]", Brace => "}" }
            ),
            Quote      => write!(f, "'"),
            Nil        => write!(f, "nil"),
        }
    }
}

const ALLOWED_SYMS: &str = "~!@#$%^&*-_=+\\|.<>?/";

#[inline(always)]
fn allowed_sym(c: char) -> bool {
    c.is_ascii_alphabetic() || ALLOWED_SYMS.contains(c)
}

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<(Token<'src>, Span)>, extra::Err<Rich<'src, char, Span>>> {
    let num = text::int(10)
        .from_str()
        .unwrapped()
        .then(just('.')
            .ignore_then(text::int(10).from_str::<u64>().unwrapped())
            .or_not()
        )
        .map(|(i, f)| match f {
            Some(f) => Token::Float(format!("{}.{}", i, f).parse().unwrap()),
            None    => Token::Int(i),
        });

    let str_ = just('"')
        .ignore_then(none_of('"').repeated().to_slice())
        .then_ignore(just('"'))
        .map(Token::Str);

    let sym = any()
        .try_map(|c: char, span| {
            if c.is_ascii_alphabetic() || allowed_sym(c) {
                Ok(c)
            } else {
                Err(Rich::custom(span, format!("Invalid symbol character: '{}'", c)))
            }
        })
        .then(select! {
            c if (c as char).is_ascii_alphanumeric()
            || allowed_sym(c) => () }.repeated()
        )
        .to_slice()
        .map(|s: &str| match s {
            "nil" => Token::Nil,
            s     => Token::Sym(s),
        });

    let punct = choice((
        just('(').to(Token::Open(Delim::Paren)),
        just('[').to(Token::Open(Delim::Brack)),
        just('{').to(Token::Open(Delim::Brace)),
        just(')').to(Token::Close(Delim::Paren)),
        just(']').to(Token::Close(Delim::Brack)),
        just('}').to(Token::Close(Delim::Brace)),
        just('\'').to(Token::Quote),
    ));

    let token = num.or(str_).or(sym).or(punct);

    let comment = just(";")
        .then(any().and_is(just('\n').not()).repeated())
        .padded();

    token
        .map_with(|t, e| (t, e.span()))
        .padded_by(comment.repeated())
        .padded()
        .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
}

pub fn lex(src: &str) -> (Option<Vec<(Token, Span)>>, Vec<Rich<char, Span>>) {
    lexer().parse(src).into_output_errors()
}


#[derive(Clone, Debug)]
pub enum List<'a> {
    Error,

    Nil, Int(i64), Float(f64), Str(&'a str),
    Sym(&'a str),
    Cons(Vec<Spanned<Self>>),
    Vec(Vec<Spanned<Self>>),
    Quote(Box<Spanned<Self>>),
}

impl fmt::Display for List<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use List::*;
        match self {
            Error    => write!(f, "<error>"),
            Nil      => write!(f, "nil"),
            Int(n)   => write!(f, "{}", n),
            Float(x) => write!(f, "{}", x),
            Str(s)   => write!(f, "{}", s),
            Sym(s)   => write!(f, "{}", s),
            Cons(es) => {
                write!(f, "(")?;
                for (i, (e, _)) in es.iter().enumerate() {
                    write!(f, "{}", e)?;
                    if i + 1 < es.len() {
                        write!(f, " ")?;
                    }
                }
                write!(f, ")")
            },
            Vec(es) => {
                write!(f, "[")?;
                for (i, (e, _)) in es.iter().enumerate() {
                    write!(f, "{}", e)?;
                    if i + 1 < es.len() {
                        write!(f, " ")?;
                    }
                }
                write!(f, "]")
            },
            Quote(e)      => write!(f, "'{}", e.0),
        }
    }
}

pub fn parser<'tks, 'src: 'tks, I>() -> impl Parser<
    'tks,
    I,
    Vec<Spanned<List<'src>>>,
    extra::Err<Rich<'tks, Token<'src>, Span>>,
> + Clone
where
    I: chumsky::input::ValueInput<'tks, Token = Token<'src>, Span = Span>
{
    recursive(|expr| {
        let atom = select! {
            Token::Nil      => List::Nil,
            Token::Int(n)   => List::Int(n),
            Token::Float(x) => List::Float(x),
            Token::Str(s)   => List::Str(s),
            Token::Sym(s)   => List::Sym(s),
        }.map_with(|e, s| (e, s.span()))
        .labelled("atom");

        macro_rules! quoted {
            ($tok:expr, $to:ident) => {
                just($tok)
                    .ignore_then(expr.clone())
                    .map(|e| List::$to(Box::new(e)))
                    .map_with(|e, s| (e, s.span()))
            };
        }

        let quotes = choice((
            quoted!(Token::Quote, Quote),
        )).labelled("quote");

        macro_rules! list {
            ($delim:expr, $to:expr) => {
                expr.clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(
                        just(Token::Open($delim)),
                        just(Token::Close($delim))
                    )
                    .map($to)
                    .map_with(|e, s| (e, s.span()))
                    .recover_with(via_parser(nested_delimiters(
                        Token::Open($delim),
                        Token::Close($delim),
                        [
                            (Token::Open(Delim::Paren), Token::Close(Delim::Paren)),
                            (Token::Open(Delim::Brack), Token::Close(Delim::Brack)),
                            (Token::Open(Delim::Brace), Token::Close(Delim::Brace)),
                        ],
                        |span| (List::Error, span),
                    )))
            };
        }

        let list = choice((
            list!(Delim::Paren, List::Cons),
            list!(Delim::Brack, List::Vec),
        )).labelled("list");

        list
            .or(atom)
            .or(quotes)
    })
        .boxed()
        .repeated()
        .collect::<Vec<_>>()
}

pub fn parse<'src>(tks: &'src Vec<(Token<'src>, SimpleSpan)>, eoi: Span)
    -> (Option<Vec<(List<'src>, SimpleSpan)>>, Vec<Rich<'src, Token<'src>>>) {
    parser()
        .parse(tks.as_slice()
            .map(eoi, |(t, s)| (t, s)),
        )
        .into_output_errors()
}