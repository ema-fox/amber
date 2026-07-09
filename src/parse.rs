use nom::{IResult, Parser};
use nom::branch::{alt};
use nom::character::complete::{char, one_of, alpha1, alphanumeric1, digit1, multispace0};
use nom::combinator::{recognize, map};
use nom::multi::{many0_count, many0};
use nom::sequence::{delimited, preceded};
use nom::bytes::{take_till};

use crate::val::{Val};

fn sym_char(inp: &str) -> IResult<&str, &str> {
    recognize(one_of("<+-")).parse(inp)
}

fn psym(inp: &str) -> IResult<&str, &str> {
    recognize((alt((alpha1, sym_char)), many0_count(alt((alphanumeric1, sym_char))))).parse(inp)
}

fn pnum(inp: &str) -> IResult<&str, &str> {
    digit1.parse(inp)
}

fn pnumlit(inp: &str) -> IResult<&str, Val> {
    map(pnum, |v: &str|
        Val::Int(i64::from_str_radix(v, 10).unwrap())
    ).parse(inp)
}

fn pstr(inp: &str) -> IResult<&str, &str> {
    delimited(char('"'),  take_till(|c| c == '"'), char('"')).parse(inp)
}

fn pstrlit(inp: &str) -> IResult<&str, Val> {
    map(pstr, |v: &str|
        Val::Str(v.to_string())
    ).parse(inp)
}

fn plit(inp: &str) -> IResult<&str, Val> {
    map(alt((pnumlit, pstrlit)), |v: Val|
        Val::Dict(im::HashMap::from(vec![
            ("op".into(), "lit".into()),
            ("val".into(), v)
        ]))
    ).parse(inp)
}

fn pderef(inp: &str) -> IResult<&str, Val> {
    map(psym, |v: &str|
        Val::Dict(im::HashMap::from(vec![
            ("op".into(), "deref".into()),
            ("name".into(), v.into())
        ]))
    ).parse(inp)
}

fn pbind(inp: &str) -> IResult<&str, Val> {
    map((pinst_, (char(':'), multispace0), inst),
        |(name, _, body)| create_inst("bind", vec![name, body])).parse(inp)
}


fn create_inst(op: &str, args: Vec<Val>) -> Val {
    Val::Dict(im::HashMap::from(vec![
        ("op".into(), op.into()),
        ("args".into(), args.into())
    ]))
}

fn pbraceinst(inp: &str) -> IResult<&str, Val> {
    map(delimited(char('{'), (psym, insts), char('}')),
        |(op, args): (&str, Vec<Val>)| create_inst(op, args)
    ).parse(inp)
}

fn plistinst(inp: &str) -> IResult<&str, Val> {
    map(delimited(char('['), insts, char(']')),
        |entries: Vec<Val>| create_inst("list", entries)
    ).parse(inp)
}

fn pcallinst(inp: &str) -> IResult<&str, Val> {
    map(delimited(char('('), (inst, insts), char(')')),
        |(f, args): (Val, Vec<Val>)| create_inst("call", vec![f, create_inst("list", args)])
    ).parse(inp)
}

fn pinst_(inp: &str) -> IResult<&str, Val> {
    alt((plit, pcallinst, plistinst, pbraceinst, pderef)).parse(inp)
}

pub fn inst(inp: &str) -> IResult<&str, Val> {
    alt((pbind, pinst_)).parse(inp)
}

pub fn insts(inp: &str) -> IResult<&str, Vec<Val>> {
    many0(preceded(multispace0, inst)).parse(inp)
}
