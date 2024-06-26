use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

/// This is a thin wrapper around the json AST
/// generated by the solidity compiler.

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

/// Solidity AST representation.
///
/// There are two fields, `element` which is the underlying json object
/// representing an AST node and `contract` which indicates the name of the
/// contract that this node belongs to.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub struct SolAST {
    pub(crate) element: Option<Value>,
}

impl SolAST {
    /// Create a new AST node.
    pub fn new(v: Value) -> Self {
        if v.is_null() {
            Self { element: None }
        } else {
            Self { element: Some(v) }
        }
    }

    /// Return the `element` field of a `SolAST` struct.
    pub fn get_object(&self) -> Option<Value> {
        self.element.clone()
    }

    /// Return some node of this AST that has the field name `fnm` in the json
    /// representation.
    pub fn get_node(&self, fnm: &str) -> SolAST {
        let node: SolAST = self.get_object().map_or_else(
            || SolAST { element: None },
            |v| SolAST {
                element: Some(v[fnm].clone()),
            },
        );
        node
    }

    pub fn is_literal(&self) -> bool {
        self.node_type() == Some("Literal".into())
    }

    /// Check if this node has kind `"number"` or if it is a unary operator `"-"`
    /// on a kind "number".
    ///
    /// This function is necessary because solc doesn't parse negative integer
    /// literals as literals, but rather as a negative unary operator applied to
    /// a literal.  For instance, solc parses `-1` as
    ///
    /// `(unop '-' (number 1))`
    pub fn is_literal_number(&self) -> bool {
        let k = self.node_kind();
        if Some("number".into()) == k {
            true
        } else if self.node_type() == Some("UnaryOperator".into())
            && self.operator() == Some("-".into())
        {
            let operand = self.get_node("subExpression");
            operand.node_kind() == Some("number".into())
        } else {
            false
        }
    }

    /// A helper that is used in various places to get the value of some
    /// field name (`fnm`) in the AST's `element`.
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

    /// Returns the `src` field.
    pub fn src(&self) -> Option<String> {
        self.get_string("src")
    }

    /// Returns the `name` field.
    pub fn name(&self) -> Option<String> {
        self.get_string("name")
    }

    /// Returns the `node_type` field.
    pub fn node_type(&self) -> Option<String> {
        self.get_string("nodeType")
    }

    pub fn node_kind(&self) -> Option<String> {
        self.get_string("kind")
    }

    /// Returns the `expression` field.
    pub fn expression(&self) -> SolAST {
        self.get_node("expression")
    }

    /// Returns the `operator` field.
    pub fn operator(&self) -> Option<String> {
        self.get_string("operator")
    }

    /// Returns the `leftExpression` field.
    pub fn left_expression(&self) -> SolAST {
        self.get_node("leftExpression")
    }

    /// Returns the `rightExpression` field.
    pub fn right_expression(&self) -> SolAST {
        self.get_node("rightExpression")
    }

    /// Returns the `leftHandSide` field.
    pub fn left_hand_side(&self) -> SolAST {
        self.get_node("leftHandSide")
    }

    /// Returns the `rightHandSide` field.
    pub fn right_hand_side(&self) -> SolAST {
        self.get_node("rightHandSide")
    }

    /// Returns the `arguments` representing argument nodes to some function.
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

    /// Returns `statements` in some block.
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

    /// Returns the `condition` field.
    pub fn condition(&self) -> SolAST {
        self.get_node("condition")
    }

    /// Returns the `trueBody` field.
    pub fn true_body(&self) -> SolAST {
        self.get_node("trueBody")
    }

    /// Returns the `falseBody` field.
    pub fn false_body(&self) -> SolAST {
        self.get_node("falseBody")
    }

    /// Returns the `typeDescriptions` field.
    pub fn get_type_descs(&self) -> Option<TypeDescriptions> {
        self.get_object()
            .map(|obj| TypeDescriptions::new(obj["typeDescriptions"].clone()))
    }

    /// Recursively traverses the AST.
    ///
    /// This is how Gambit determines what nodes can be mutated using which
    /// types of mutations and the exact location in the source where the
    /// mutation must be done.
    ///
    /// # Arguments
    ///
    /// * `visitor` - see [`run::RunMutations::mk_closures()`]
    /// * `skip` - see [`run::RunMutations::mk_closures()`]
    /// * `accept` - see [`run::RunMutations::mk_closures()`]
    pub fn traverse<A, T>(self, visitor: &dyn SolASTVisitor<A, T>, arg: A) -> Vec<T> {
        let mut result: Vec<T> = vec![];
        self.traverse_internal(visitor, &arg, &mut result);
        result
    }

    /// Helper function to traverse AST
    ///
    /// # Arguments
    ///
    /// * `visitor` - see [`run::RunMutations::mk_closures()`]
    /// * `skip` - see [`run::RunMutations::mk_closures()`]
    /// * `accept` - see [`run::RunMutations::mk_closures()`]
    /// * `accepted` - is this node the descendent of an accepted node? This
    ///   value is monotonic as we descend an AST: it begins as false but once
    ///   set to true will be true for all recursive calls
    /// * `acc` - TODO: ?
    fn traverse_internal<A, T>(
        &self,
        visitor: &dyn SolASTVisitor<A, T>,
        arg: &A,
        acc: &mut Vec<T>,
    ) {
        log::debug!(
            "Traversing Node: kind: {:?}, type: {:?}",
            self.node_kind(),
            self.node_type(),
        );
        if visitor.skip_node(self, arg) {
            log::debug!("    Skipping");
            return;
        }

        if let Some(result) = visitor.visit_node(self, arg) {
            log::debug!("    Visit successful");
            acc.push(result);
        } else {
            log::debug!("    Visit failed")
        }

        if self.element.is_none() {
            return;
        }

        let e = self.element.as_ref().unwrap();
        if e.is_object() {
            let e_obj = e.as_object().unwrap();

            // TODO: We are _cloning_ entire ASTs! This is no bueno!
            log::debug!("    Recursively traversing children");
            for v in e_obj.values() {
                let child: SolAST = SolAST::new(v.clone());
                child.traverse_internal(visitor, arg, acc);
            }
        } else if e.is_array() {
            let e_arr = e.as_array().unwrap();
            for a in e_arr {
                let child: SolAST = SolAST::new(a.clone());
                child.traverse_internal(visitor, arg, acc);
            }
        }
    }

    /// Extracts the bounds from the AST that indicate where in the source
    /// a node's text starts and ends.
    /// This is represented by the `src` field in the AST about which more
    /// information can be found [here](https://docs.soliditylang.org/en/v0.8.17/using-the-compiler.html?highlight=--ast-compact--json#compiler-input-and-output-json-description).
    pub fn get_bounds(&self) -> (usize, usize) {
        let src = self.src().expect("Source information missing.");
        let parts: Vec<&str> = src.split(':').collect();
        let start = parts[0].parse::<usize>().unwrap();
        (start, start + parts[1].parse::<usize>().unwrap())
    }

    /// Returns the text corresponding to an AST node in the given `source`.
    pub fn get_text(&self, source: &[u8]) -> String {
        let (start, end) = self.get_bounds();
        let byte_vec = source[start..end].to_vec();
        String::from_utf8(byte_vec).expect("Slice is not u8.")
    }

    /// This method is used by a variety of mutations like `FunctionCallMutation`,
    /// `RequireMutation`, etc. (see more in `mutation.rs`) to directly
    /// mutate the source guided by information gathered from traversing the AST.
    pub fn replace_in_source(&self, source: &[u8], new: String) -> String {
        let (start, end) = self.get_bounds();
        self.replace_part(source, new, start, end)
    }

    /// This method is used to replace part of a statement.
    /// Example mutation types that use it are are `BinaryOperatorMutation`,
    /// `UnaryOperatorMutation`, and `ElimDelegateMutation`.
    pub fn replace_part(&self, source: &[u8], new: String, start: usize, end: usize) -> String {
        let before = &source[0..start];
        let changed = new.as_bytes();
        let after = &source[end..source.len()];
        let res = [before, changed, after].concat();
        String::from_utf8(res).expect("Slice is not u8.")
    }

    /// This method is used for mutations that comment out
    /// some piece of code using block comments.
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

/// Implement this to traverse an AST
pub trait SolASTVisitor<A, R> {
    /// Performs logic on a given node
    fn visit_node(&self, node: &SolAST, arg: &A) -> Option<R>;

    /// Determines if this node should not be recursively visited. If `true`,
    /// this will not be visited, nor will its children
    fn skip_node(&self, _node: &SolAST, _arg: &A) -> bool {
        false
    }
}
