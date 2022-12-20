use itertools::Itertools;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Default, Clone)]
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

struct Replacement {
    start: usize,
    end: usize,
    new: String,
}

/// AST representation.
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
        let obj = self.get_object();
        match obj {
            Some(o) => {
                let v = o[fnm].as_str();
                v.map(|s| s.into())
            }
            None => None,
        }
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

    pub fn operator(&self) -> Option<String> {
        self.get_string("operator")
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

    pub fn get_type_descs(&self) -> TypeDescriptions {
        let obj = self
            .get_object()
            .unwrap_or_else(|| panic!("There is no type description."));
        TypeDescriptions::new(obj["typeDescriptions"].clone())
    }

    pub fn traverse<T, F>(
        self,
        mut visitor: F,
        mut skip: impl Fn(&SolAST) -> bool,
        mut accept: impl Fn(&SolAST) -> bool,
    ) -> Vec<T>
    where
        F: FnMut(&SolAST) -> Option<T>,
    {
        let mut result: Vec<T> = vec![];
        log::info!("entering traverse_internal");
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
    ) {
        let mut new_accepted = accepted;
        // log::info!("accepted = {}", new_accepted);
        if accept(&self) {
            new_accepted = true;
        }
        if skip(&self) {
            return;
        }
        if new_accepted {
            // log::info!("about to visit {:?}", &self);
            let res = visitor(&self);
            if let Some(r) = res {
                log::info!("adding results to list of accepted nodes");
                acc.push(r)
            } else {
                // log::info!("no mutation points found");
            }
        }
        if self.element.is_some() {
            let e = self.element.unwrap();
            if e.is_object() {
                let e_obj = e.as_object().unwrap();
                for v in e_obj.values() {
                    let child: SolAST = SolAST::new(v.clone());
                    // log::info!("object child: {:?}", child);
                    child.traverse_internal(visitor, skip, accept, new_accepted, acc);
                }
            } else if e.is_array() {
                let e_arr = e.as_array().unwrap();
                for a in e_arr {
                    let child: SolAST = SolAST::new(a.clone());
                    // log::info!("array child: {:?}", child.name());
                    child.traverse_internal(visitor, skip, accept, new_accepted, acc);
                }
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

    pub fn replace_multiple(&self, source: &[u8], reps: Vec<(SolAST, String)>) -> String {
        let sorted = reps
            .iter()
            .map(|(node, n)| {
                let (s, e) = node.get_bounds();
                Replacement {
                    start: s,
                    end: e,
                    new: n.into(),
                }
            })
            .sorted_by_key(|x| x.start);
        let mut new_src = source.to_vec();
        let mut curr_offset = 0;
        for r in sorted {
            let actual_start = r.start + curr_offset;
            let actual_end = r.end + curr_offset;
            let replace_bytes = r.new.as_bytes();
            let new_start = &new_src[0..actual_start];
            let new_end = &new_src[actual_end..new_src.len()];
            new_src = [new_start, replace_bytes, new_end].concat();
            let new_offset = replace_bytes.len() - (r.end - r.start);
            curr_offset += new_offset;
        }
        String::from_utf8(new_src.to_vec()).expect("Slice new_src is not u8.")
    }

    pub fn comment_out(&self, source: &[u8]) -> String {
        let (start, mut end) = self.get_bounds();
        let rest_of_str = String::from_utf8(source[end..source.len()].to_vec())
            .unwrap_or_else(|_| panic!("cannot convert bytes to string."));
        let mtch = Regex::new(r"^\*").unwrap().find(rest_of_str.as_str());
        if let Some(m) = mtch {
            end +=
                rest_of_str[0..m.range().last().unwrap_or_else(|| {
                    panic!("There was a match but last() still returned None.")
                }) + 1]
                    .as_bytes()
                    .len();
        }
        self.replace_part(
            source,
            "/*".to_string() + &String::from_utf8(source[start..end].to_vec()).unwrap() + "*/",
            start,
            end,
        )
    }
}
