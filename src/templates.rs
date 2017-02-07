use std::borrow::ToOwned;

#[derive(Debug, PartialEq, Eq)]
enum Piece {
    Arg(String),
    Placeholder
}

#[derive(Debug)]
pub struct Template(Vec<Piece>);

pub const TEMPLATE_PLACEHOLDER: &'static str = "{}";

impl<'a, S> From<&'a [S]> for Template where S: AsRef<str> {
    fn from(v: &[S]) -> Self {
        Template(v.iter().map(|s| {
            let s = s.as_ref();
            if s == TEMPLATE_PLACEHOLDER {
                Piece::Placeholder
            } else {
                Piece::Arg(s.to_owned())
            }}).collect())
    }
}

#[derive(Debug)]
pub enum TemplateError {
    ArgumentCountMismatch
}

impl Template {
    pub fn fill<S>(&self, v: &[S]) -> Result<Vec<String>, TemplateError>
        where S: AsRef<str> {
        if self.placeholders() == v.len() {
            let mut values = v.iter();
            let res = self.0.iter().map(|p| match p {
                &Piece::Placeholder => values.next().unwrap().as_ref().to_owned(),
                &Piece::Arg(ref s) => s.to_owned()
            }).collect();
            Ok(res)
        } else {
            Err(TemplateError::ArgumentCountMismatch)
        }
    }

    pub fn placeholders(&self) -> usize {
        self.0.iter().filter(|p| **p == Piece::Placeholder).count()
    }
}

#[test]
fn test_templates() {
    let slice: &[&str] = &["a", "{}", "c", "{}"];
    let template: Template = slice.into();
    let filled = template.fill(&["b", "d"]);
    assert!(filled.is_ok());
    assert_eq!(filled.unwrap(), ["a", "b", "c", "d"]);
    assert!(template.fill(&["b"]).is_err());
}
