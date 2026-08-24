use nom::{IResult, Parser};
use nom::branch::{alt};
use nom::character::complete::{char, one_of, alpha1, alphanumeric1, digit1, multispace1};
use nom::combinator::{recognize, map, cut, all_consuming, opt};
use nom::multi::{many0_count, many0, many1};
use nom::sequence::{delimited, preceded, terminated};
use nom::bytes::{take_till};

use crate::val::{Val};
use crate::create;

fn comment(inp: &str) -> IResult<&str, &str> {
    recognize((char('#'), space, inst)).parse(inp)
}

fn space(inp: &str) -> IResult<&str, usize> {
    many0_count(alt((comment, multispace1))).parse(inp)
}

fn sym_char(inp: &str) -> IResult<&str, &str> {
    recognize(one_of("=<>_+-/!")).parse(inp)
}

fn psym(inp: &str) -> IResult<&str, &str> {
    recognize((alt((alpha1, sym_char)), many0_count(alt((alphanumeric1, sym_char))))).parse(inp)
}

fn pnum(inp: &str) -> IResult<&str, &str> {
    recognize((opt(char('-')), digit1)).parse(inp)
}

fn pnumlit(inp: &str) -> IResult<&str, Val> {
    map(pnum, |v: &str|
        Val::Int(i64::from_str_radix(v, 10).unwrap())
    ).parse(inp)
}

fn pstr(inp: &str) -> IResult<&str, &str> {
    delimited(char('"'),  take_till(|c| c == '"'), cut(char('"'))).parse(inp)
}

fn pstrlit(inp: &str) -> IResult<&str, Val> {
    map(pstr, |v: &str|
        Val::Str(v.replace("\\n", "\n"))
    ).parse(inp)
}

fn plit(inp: &str) -> IResult<&str, Val> {
    map(alt((pnumlit, pstrlit)), create::lit).parse(inp)
}

fn pderef(inp: &str) -> IResult<&str, Val> {
    map(psym, create::deref).parse(inp)
}

fn pdot(f: &Val) -> impl Fn(&str) -> IResult<&str, Val> {
    |inp: &str| {
        map(many1(preceded((char('.'), space), cut(pinst_))),
            |mut args| {
                let mut args2: Vec<Val> = vec![f.clone()];
                args2.append(&mut args);
                create::inst("dot", args2)
            }).parse(inp)
    }
}

fn pbind(name: &Val) -> impl Fn (&str) -> IResult<&str, Val> {
    |inp: &str | {
        map(((char(':'), space), cut(inst)),
            |(_, body)| create::inst("bind", vec![name.clone(), body])).parse(inp)
    }
}

fn pbraceinst(inp: &str) -> IResult<&str, Val> {
    map(delimited(char('{'), (psym, insts), cut(char('}'))),
        |(op, args): (&str, Vec<Val>)| create::inst(op, args)
    ).parse(inp)
}

fn plistinst(inp: &str) -> IResult<&str, Val> {
    map(delimited(char('['), insts, char(']')),
        |entries: Vec<Val>| create::inst("list", entries)
    ).parse(inp)
}

fn pcallinst(inp: &str) -> IResult<&str, Val> {
    map(delimited(char('('), (inst, insts), char(')')),
        |(f, args): (Val, Vec<Val>)| create::inst("call", vec![f, create::inst("list", args)])
    ).parse(inp)
}

fn pinst_(inp: &str) -> IResult<&str, Val> {
    alt((plit, pcallinst, plistinst, pbraceinst, pderef)).parse(inp)
}

pub fn inst(inp: &str) -> IResult<&str, Val> {
    // TODO check that bind and dot work correctly together
    // TODO make this less ugly
    let (rest, inst) = pinst_.parse(inp)?;
    alt((pbind(&inst), pdot(&inst))).parse(rest).or(Ok((rest, inst.clone())))
}

fn insts(inp: &str) -> IResult<&str, Vec<Val>> {
    terminated(many0(preceded(space, inst)), space).parse(inp)
}

pub fn module(inp: &str) -> IResult<&str, Val> {
    map(all_consuming(insts),
        |body| create::inst("module", body)
    ).parse(inp)
}
