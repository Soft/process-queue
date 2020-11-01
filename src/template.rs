use std::str::FromStr;
use std::string::ToString;

use anyhow::{anyhow, Error, Result};
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

const ARG_STR: &str = "{}";
const VAR_ARG_STR: &str = "{...}";

#[derive(Debug, Eq, PartialEq)]
enum Piece {
    Static(String),
    Arg,
    VarArg,
}

#[derive(Debug, Eq, PartialEq)]
enum SizeInfo {
    Exactly {
        arg_count: usize,
    },
    Minimum {
        before_var_arg_count: usize,
        after_var_arg_count: usize,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct Template {
    pieces: Vec<Piece>,
    size_info: SizeInfo,
}

impl Template {
    pub fn instantiate<S, I>(&self, args: S) -> Result<Vec<String>>
    where
        S: AsRef<[I]>,
        I: AsRef<str>,
    {
        let args_len = args.as_ref().len();
        let mut args_iter = args.as_ref().iter();
        match self.size_info {
            SizeInfo::Exactly { arg_count } => {
                if args_len != arg_count {
                    return Err(anyhow!(
                        "wrong number of arguments ({} arguments required)",
                        arg_count
                    ));
                }
                Ok(self
                    .pieces
                    .iter()
                    .map(|piece| match piece {
                        Piece::Static(s) => s.into(),
                        Piece::Arg => args_iter.next().unwrap().as_ref().into(),
                        _ => unreachable!(),
                    })
                    .collect())
            }
            SizeInfo::Minimum {
                before_var_arg_count,
                after_var_arg_count,
            } => {
                let min_arg_count = before_var_arg_count + after_var_arg_count;
                if args_len < min_arg_count {
                    return Err(anyhow!(
                        "wrong number of arguments (at least {} arguments required)",
                        min_arg_count
                    ));
                }
                let var_arg_len = args_len - before_var_arg_count - after_var_arg_count;
                let mut result = vec![];
                for piece in self.pieces.iter() {
                    match piece {
                        Piece::Static(s) => result.push(s.into()),
                        Piece::Arg => result.push(args_iter.next().unwrap().as_ref().into()),
                        Piece::VarArg => {
                            result.extend(
                                (0..var_arg_len).map(|_| args_iter.next().unwrap().as_ref().into()),
                            );
                        }
                    }
                }
                Ok(result)
            }
        }
    }
}

impl FromStr for Template {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut has_var_arg = false;
        let err = || {
            Err(anyhow!(
                "template cannot contain multiple variable argument expansions"
            ))
        };
        let mut before_var_arg_count = 0;
        let mut after_var_arg_count = 0;
        shlex::split(s)
            .ok_or_else(|| anyhow!("failed to split template into arguments"))?
            .into_iter()
            .map(|piece| match piece.as_ref() {
                ARG_STR => {
                    if has_var_arg {
                        after_var_arg_count += 1;
                    } else {
                        before_var_arg_count += 1;
                    }
                    Ok(Piece::Arg)
                }
                VAR_ARG_STR if !has_var_arg => {
                    has_var_arg = true;
                    Ok(Piece::VarArg)
                }
                VAR_ARG_STR if has_var_arg => err(),
                _ => Ok(Piece::Static(piece)),
            })
            .collect::<Result<Vec<_>>>()
            .map(|pieces| Template {
                pieces,
                size_info: if has_var_arg {
                    SizeInfo::Minimum {
                        before_var_arg_count,
                        after_var_arg_count,
                    }
                } else {
                    SizeInfo::Exactly {
                        arg_count: before_var_arg_count,
                    }
                },
            })
    }
}

impl<'de> Deserialize<'de> for Template {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl ToString for Template {
    fn to_string(&self) -> String {
        self.pieces
            .iter()
            .map(|piece| match piece {
                Piece::Arg => ARG_STR.into(),
                Piece::VarArg => VAR_ARG_STR.into(),
                Piece::Static(s) => shlex::quote(s),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Serialize for Template {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[test]
fn test_instantiate() {
    let t: Template = "a b c".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&[]).unwrap(),
        vec!["a", "b", "c"]
    );
    assert!(t.instantiate::<&[&str], &str>(&["a"]).is_err());

    let t: Template = "a {} c".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["b"]).unwrap(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["hello world"]).unwrap(),
        vec!["a", "hello world", "c"]
    );
    assert!(t.instantiate::<&[&str], &str>(&[]).is_err());

    let t: Template = "{} {} {}".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["a", "b", "c"]).unwrap(),
        vec!["a", "b", "c"]
    );
    assert!(t.instantiate::<&[&str], &str>(&[]).is_err());

    let t: Template = "{...}".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&[]).unwrap(),
        vec![] as Vec<String>
    );
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["a", "b", "c"]).unwrap(),
        vec!["a", "b", "c"]
    );

    let t: Template = "a {...}".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["b", "c"]).unwrap(),
        vec!["a", "b", "c"]
    );
    assert_eq!(t.instantiate::<&[&str], &str>(&[]).unwrap(), vec!["a"]);

    let t: Template = "{} - {...}".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["a", "b", "c"]).unwrap(),
        vec!["a", "-", "b", "c"]
    );
    assert!(t.instantiate::<&[&str], &str>(&[]).is_err());

    let t: Template = "{} - {...} - {} {}".parse().unwrap();
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["a", "b", "c"]).unwrap(),
        vec!["a", "-", "-", "b", "c"]
    );
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["a", "b", "c", "d"])
            .unwrap(),
        vec!["a", "-", "b", "-", "c", "d"]
    );
    assert_eq!(
        t.instantiate::<&[&str], &str>(&["a", "b", "c", "d", "e"])
            .unwrap(),
        vec!["a", "-", "b", "c", "-", "d", "e"]
    );
}
