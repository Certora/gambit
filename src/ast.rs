use serde::Deserialize;
use serde_json::{Value, json};


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

    pub fn type_string(self) -> Option<String> {
        return self.element.map(|e| e["typeString"].to_string())
    }
}

#[derive(Debug, Deserialize, Default)]
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

    pub fn get_object(self) -> Option<Value> {
        return  self.element
    }

    pub fn get_node(self, fnm: &str) -> SolAST {
        let node: SolAST = match &self.get_object() {
            Some(v) => SolAST{ element: Some(v[fnm].clone()) },
            None => SolAST { element: None },
        };
        return node
    }

    pub fn get_string(self, fnm: &str) -> Option<String> {
        return self.get_object().map(|o| o[fnm].to_string())
    }

    pub fn src(self) -> Option<String> {
        return self.get_string("src")
    }

    pub fn name(self) -> Option<String> {
        return self.get_string("name");
    }

    pub fn node_type(self) -> Option<String> {
        return self.get_string("nodeType");
    }

    pub fn expression(self) -> SolAST {
        self.get_node("expression")
    }

    pub fn left_expression(self) -> SolAST {
        self.get_node("leftExpression")
    }

    pub fn right_expression(self) -> SolAST {
        self.get_node("rightExpression")
    }

    pub fn left_hand_side(self) -> SolAST {
        self.get_node("leftHandSide")
    }

    pub fn right_hand_side(self) -> SolAST {
        self.get_node("rightHandSide")
    }

    pub fn arguments(self) -> Vec<SolAST> {
        let o = self.get_object();
        match o {
            None => return vec![],
            Some(v) => {
                let arg = &v["arguments"].as_array();
                match arg {
                    Some(lst) =>  {
                        lst.into_iter().map(|e| Self::new(e.clone())).collect()
                    },
                    None => return vec![],
                }

            }
        }
    }

    pub fn statements(self) -> Vec<SolAST> {
        let o = self.get_object();
        match o {
            None => return vec![],
            Some(v) => {
                let arg = &v["statements"].as_array();
                match arg {
                    Some(lst) =>  {
                        lst.into_iter().map(|e| Self::new(e.clone())).collect()
                    },
                    None => return vec![],
                }

            }
        }
    }

    pub fn condition(self) -> SolAST {
        return self.get_node("condition")
    }
    pub fn true_body(self) -> SolAST {
        return self.get_node("trueBody")
    }

    pub fn false_body(self) -> SolAST {
        return self.get_node("falseBody")
    }

    pub fn get_type_descs(self) -> Option<TypeDescriptions> {
        return self.get_object().map(|o| TypeDescriptions::new(o))
    }

    pub fn traverse<T>(self, visitor: fn(&SolAST) -> Option<T>, skip: fn(&SolAST) -> bool, accept: fn(&SolAST) -> bool ) -> Vec<T> {
        let mut result: Vec<T> = vec![];
        self.traverse_internal(visitor, skip, accept, false, &mut result);
        return result
    }

    fn traverse_internal<T>(self, visitor: fn(&SolAST) -> Option<T>, skip: fn(&SolAST) -> bool, accept: fn(&SolAST) -> bool, accepted: bool, acc: &mut Vec<T>) {
        let mut new_accepted = accepted;
        if accept(&self) {
            new_accepted = true
        }
        if skip(&self) {
            return;
        }
        if new_accepted {
            let res = visitor(&self);
            if res.is_some() {
                acc.push(res.unwrap())
            }
        }
        if self.element.is_some() {
            if self.element.as_ref().unwrap().is_object() {
                self.element.as_ref().into_iter().for_each(|v| {
                    let c = SolAST::new(v.clone());
                    c.traverse_internal(visitor, skip, accept, accepted, acc);
            });
            }
            if self.element.as_ref().unwrap().is_array() {
                self.element.as_ref().into_iter().for_each(|v| {
                    let c = SolAST::new(v.clone());
                    c.traverse_internal(visitor, skip, accept, accepted, acc);
            });
            }
        }
    }


}
