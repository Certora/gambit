use serde::Deserialize;
use serde_json::Value;

use crate::MutationType;

pub struct TypeDescriptions {
    pub(crate) element: Option<Value>,
}

impl TypeDescriptions {
    pub fn new(v: Value) -> Self {
        if v.is_null() {
            Self { element: None }
        } else {
            Self { element: Some(v) }
        }
    }

    pub fn type_string(&self) -> Option<String> {
        self.element.as_ref().map(|e| e["typeString"].to_string())
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub struct SolAST {
    pub(crate) element: Option<Value>,
}

impl SolAST {
    pub fn new(v: Value) -> Self {
        if v.is_null() {
            Self { element: None }
        } else {
            Self { element: Some(v) }
        }
    }

    pub fn get_object(&self) -> Option<Value> {
        self.element.clone()
    }

    pub fn get_node(&self, fnm: &str) -> SolAST {
        let node: SolAST = match &self.get_object() {
            Some(v) => SolAST {
                element: Some(v[fnm].clone()),
            },
            None => SolAST { element: None },
        };
        node
    }

    pub fn get_string(&self, fnm: &str) -> Option<String> {
        self.get_object().map(|o| o[fnm].to_string())
    }

    pub fn src(&self) -> Option<String> {
        self.get_string("src")
    }

    pub fn name(&self) -> Option<String> {
        self.get_string("name")
    }

    pub fn node_type(&self) -> Option<String> {
        self.get_string("nodeType")
    }

    pub fn expression(&self) -> SolAST {
        self.get_node("expression")
    }

    pub fn left_expression(&self) -> SolAST {
        self.get_node("leftExpression")
    }

    pub fn right_expression(&self) -> SolAST {
        self.get_node("rightExpression")
    }

    pub fn left_hand_side(&self) -> SolAST {
        self.get_node("leftHandSide")
    }

    pub fn right_hand_side(&self) -> SolAST {
        self.get_node("rightHandSide")
    }

    pub fn arguments(&self) -> Vec<SolAST> {
        let o = self.get_object();
        match o {
            None => vec![],
            Some(v) => {
                let arg = &v["arguments"].as_array();
                match arg {
                    Some(lst) => lst.iter().map(|e| Self::new(e.clone())).collect(),
                    None => vec![],
                }
            }
        }
    }

    pub fn statements(&self) -> Vec<SolAST> {
        let o = self.get_object();
        match o {
            None => vec![],
            Some(v) => {
                let arg = &v["statements"].as_array();
                match arg {
                    Some(lst) => lst.iter().map(|e| Self::new(e.clone())).collect(),
                    None => vec![],
                }
            }
        }
    }

    pub fn condition(&self) -> SolAST {
        self.get_node("condition")
    }
    pub fn true_body(&self) -> SolAST {
        self.get_node("trueBody")
    }

    pub fn false_body(&self) -> SolAST {
        self.get_node("falseBody")
    }

    pub fn get_type_descs(&self) -> Option<TypeDescriptions> {
        self.get_object().map(TypeDescriptions::new)
    }

    pub fn traverse<T, F>(
        self,
        mut visitor: F,
        mut skip: impl Fn(&SolAST) -> bool,
        mut accept: impl Fn(&SolAST) -> bool,
    ) -> Vec<T> 
    where F: FnMut(&SolAST) -> Option<T>,
    {
        let mut result: Vec<T> = vec![];
        self.traverse_internal(&mut visitor, &mut skip, &mut accept, false, &mut result);
        result
    }

    fn traverse_internal<T>(
        self,
        visitor: &mut impl FnMut(&SolAST) -> Option<T>,
        skip: &mut impl FnMut(&SolAST) -> bool,
        accept: &mut impl FnMut(&SolAST) -> bool,
        accepted: bool,
        acc: &mut Vec<T>,
    )
    {
        let mut new_accepted = accepted;
        if accept(&self) {
            new_accepted = true
        }
        if skip(&self) {
            return;
        }
        if new_accepted {
            let res = visitor(&self);
            if let Some(r) = res {
                acc.push(r)
            }
        }
        if self.element.is_some() {
            if self.element.as_ref().unwrap().is_object() {
                self.element.as_ref().into_iter().for_each(|v| {
                    let child: SolAST = SolAST::new(v.clone());
                    child.traverse_internal(visitor, skip, accept, accepted, acc);
                });
            }
            if self.element.as_ref().unwrap().is_array() {
                self.element.as_ref().into_iter().for_each(|v| {
                    let child: SolAST = SolAST::new(v.clone());
                    child.traverse_internal(visitor, skip, accept, accepted, acc);
                });
            }
        }
    }

    pub fn get_bounds(&self) -> (usize, usize) {
        let src = self.src().expect("Source information missing.");
        let parts: Vec<&str> = src.split(':').collect();
        let start = parts[0].parse::<usize>().unwrap();
        (start, start + parts[1].parse::<usize>().unwrap())
    }

    pub fn get_text(&self, source: &[u8]) -> String {
        let (start, end) = self.get_bounds();
        let byte_vec = source[start..end].to_vec();
        String::from_utf8(byte_vec).expect("Slice is not u8.")
    }

    pub fn replace_in_source(&self, source: &[u8], new: String) -> String {
        let (start, end) = self.get_bounds();
        self.replace_part(source, new, start, end)
    }

    pub fn replace_part(&self, source: &[u8], new: String, start: usize, end: usize) -> String {
        let before = &source[0..start];
        let changed = new.as_bytes();
        let after = &source[end..source.len()];
        let res = [before, changed, after].concat();
        String::from_utf8(res).expect("Slice is not u8.")
    }
}
