//! Recursive-descent XPath parser for the grammar subset in scope (design
//! §5). `Result` at every production; unsupported-but-valid axes/functions
//! (`following-sibling::`, `contains()`, ...) always produce
//! [`QueryError::UnsupportedFeature`], never a silently-wrong parse.

use crate::error::QueryError;
use crate::limits::MAX_NESTING_DEPTH;
use crate::xpath::ast::{Axis, NodeTest, Predicate, Step, XPathExpr};
use crate::xpath::tokenizer::{tokenize, PositionedToken, Token};

const KNOWN_UNSUPPORTED_AXES: &[&str] = &[
    "following-sibling",
    "preceding-sibling",
    "following",
    "preceding",
    "ancestor",
    "ancestor-or-self",
    "descendant-or-self",
    "namespace",
];

const KNOWN_UNSUPPORTED_NODE_TEST_FUNCTIONS: &[&str] = &["processing-instruction"];

struct Parser {
    tokens: Vec<PositionedToken>,
    cursor: usize,
}

pub(crate) fn parse(source: &str) -> Result<XPathExpr, QueryError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser { tokens, cursor: 0 };
    let expr = parser.parse_expr()?;
    if !matches!(parser.peek(), Token::Eof) {
        return Err(QueryError::InvalidXPath {
            message: "unexpected trailing content after xpath expression".to_string(),
            position: parser.position(),
        });
    }
    Ok(expr)
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor].token
    }

    fn position(&self) -> usize {
        self.tokens[self.cursor].position
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.cursor].token.clone();
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
        token
    }

    fn parse_expr(&mut self) -> Result<XPathExpr, QueryError> {
        let mut absolute = false;
        let mut leading_descendant = false;
        match self.peek() {
            Token::SlashSlash => {
                self.advance();
                absolute = true;
                leading_descendant = true;
            }
            Token::Slash => {
                self.advance();
                absolute = true;
            }
            _ => {}
        }

        let mut steps = Vec::new();
        if absolute && matches!(self.peek(), Token::Eof) {
            // Bare "/" selects the document root itself: represented as a
            // path with zero steps starting from the root context.
            return Ok(XPathExpr { absolute, steps });
        }

        loop {
            let step = self.parse_step(leading_descendant)?;
            steps.push(step);
            if steps.len() > MAX_NESTING_DEPTH {
                return Err(QueryError::TooComplex {
                    limit: crate::error::LimitKind::SelectorNesting,
                });
            }
            leading_descendant = false;
            match self.peek() {
                Token::SlashSlash => {
                    self.advance();
                    leading_descendant = true;
                }
                Token::Slash => {
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(XPathExpr { absolute, steps })
    }

    fn parse_step(&mut self, leading_descendant: bool) -> Result<Step, QueryError> {
        let step_position = self.position();
        if matches!(self.peek(), Token::Dot) {
            self.advance();
            if leading_descendant {
                return Err(QueryError::UnsupportedFeature {
                    feature: "'//' combined with an explicit axis specifier ('.')".to_string(),
                    position: step_position,
                });
            }
            let predicates = self.parse_predicates()?;
            return Ok(Step {
                axis: Axis::SelfAxis,
                test: NodeTest::Node,
                predicates,
            });
        }
        if matches!(self.peek(), Token::DotDot) {
            self.advance();
            if leading_descendant {
                return Err(QueryError::UnsupportedFeature {
                    feature: "'//' combined with an explicit axis specifier ('..')".to_string(),
                    position: step_position,
                });
            }
            let predicates = self.parse_predicates()?;
            return Ok(Step {
                axis: Axis::Parent,
                test: NodeTest::Node,
                predicates,
            });
        }

        let mut explicit_axis = false;
        let axis = if matches!(self.peek(), Token::At) {
            self.advance();
            explicit_axis = true;
            Axis::Attribute
        } else if let Token::Ident(name) = self.peek().clone() {
            if matches!(
                self.tokens.get(self.cursor + 1).map(|t| &t.token),
                Some(Token::ColonColon)
            ) {
                explicit_axis = true;
                self.advance(); // ident
                self.advance(); // '::'
                match name.as_str() {
                    "child" => Axis::Child,
                    "descendant" => Axis::Descendant,
                    "attribute" => Axis::Attribute,
                    "self" => Axis::SelfAxis,
                    "parent" => Axis::Parent,
                    other if KNOWN_UNSUPPORTED_AXES.contains(&other) => {
                        return Err(QueryError::UnsupportedFeature {
                            feature: format!("{other}:: axis"),
                            position: step_position,
                        });
                    }
                    other => {
                        return Err(QueryError::InvalidXPath {
                            message: format!("unknown axis '{other}::'"),
                            position: step_position,
                        });
                    }
                }
            } else {
                Axis::Child
            }
        } else {
            Axis::Child
        };

        if leading_descendant && explicit_axis {
            return Err(QueryError::UnsupportedFeature {
                feature: "'//' combined with an explicit axis specifier".to_string(),
                position: step_position,
            });
        }
        let axis = if leading_descendant {
            Axis::Descendant
        } else {
            axis
        };

        let test = self.parse_node_test()?;
        if matches!(axis, Axis::Attribute) && matches!(test, NodeTest::Any) {
            return Err(QueryError::UnsupportedFeature {
                feature: "@* (wildcard attribute node test)".to_string(),
                position: step_position,
            });
        }
        let predicates = self.parse_predicates()?;
        Ok(Step {
            axis,
            test,
            predicates,
        })
    }

    fn parse_node_test(&mut self) -> Result<NodeTest, QueryError> {
        match self.peek().clone() {
            Token::Star => {
                self.advance();
                Ok(NodeTest::Any)
            }
            Token::Ident(name) => {
                self.advance();
                if matches!(self.peek(), Token::LParen) {
                    self.advance();
                    if !matches!(self.peek(), Token::RParen) {
                        return Err(QueryError::UnsupportedFeature {
                            feature: format!("{name}(...) with arguments"),
                            position: self.position(),
                        });
                    }
                    self.advance();
                    match name.as_str() {
                        "text" => Ok(NodeTest::Text),
                        "node" => Ok(NodeTest::Node),
                        "comment" => Ok(NodeTest::Comment),
                        other if KNOWN_UNSUPPORTED_NODE_TEST_FUNCTIONS.contains(&other) => {
                            Err(QueryError::UnsupportedFeature {
                                feature: format!("{other}()"),
                                position: self.position(),
                            })
                        }
                        other => Err(QueryError::InvalidXPath {
                            message: format!("unknown node-test function '{other}()'"),
                            position: self.position(),
                        }),
                    }
                } else {
                    Ok(NodeTest::Name(name))
                }
            }
            _ => Err(QueryError::InvalidXPath {
                message: "expected a node test (element name, '*', text(), node(), comment())"
                    .to_string(),
                position: self.position(),
            }),
        }
    }

    fn parse_predicates(&mut self) -> Result<Vec<Predicate>, QueryError> {
        let mut predicates = Vec::new();
        while matches!(self.peek(), Token::LBracket) {
            self.advance();
            let predicate = self.parse_predicate_expr()?;
            match self.peek() {
                Token::RBracket => {
                    self.advance();
                }
                _ => {
                    return Err(QueryError::InvalidXPath {
                        message: "expected ']' to close predicate".to_string(),
                        position: self.position(),
                    });
                }
            }
            predicates.push(predicate);
            if predicates.len() > MAX_NESTING_DEPTH {
                return Err(QueryError::TooComplex {
                    limit: crate::error::LimitKind::SelectorNesting,
                });
            }
        }
        Ok(predicates)
    }

    fn parse_predicate_expr(&mut self) -> Result<Predicate, QueryError> {
        let mut left = self.parse_predicate_operand()?;
        while let Token::Ident(word) = self.peek().clone() {
            if word == "and" {
                self.advance();
                let right = self.parse_predicate_operand()?;
                left = Predicate::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_predicate_operand(&mut self) -> Result<Predicate, QueryError> {
        match self.peek().clone() {
            Token::Number(digits) => {
                self.advance();
                let index: usize = digits.parse().map_err(|_| QueryError::InvalidXPath {
                    message: format!("invalid positional predicate '{digits}'"),
                    position: self.position(),
                })?;
                Ok(Predicate::Index(index))
            }
            Token::Ident(name) if name == "last" => {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                Ok(Predicate::Last)
            }
            Token::Ident(name) => Err(QueryError::UnsupportedFeature {
                feature: format!("{name}(...) predicate function"),
                position: self.position(),
            }),
            Token::At => {
                self.advance();
                let name = match self.peek().clone() {
                    Token::Ident(name) => {
                        self.advance();
                        name
                    }
                    _ => {
                        return Err(QueryError::InvalidXPath {
                            message: "expected attribute name after '@'".to_string(),
                            position: self.position(),
                        });
                    }
                };
                if matches!(self.peek(), Token::Eq) {
                    self.advance();
                    let value = match self.peek().clone() {
                        Token::Str(value) => {
                            self.advance();
                            value
                        }
                        _ => {
                            return Err(QueryError::InvalidXPath {
                                message: "expected a quoted string after '='".to_string(),
                                position: self.position(),
                            });
                        }
                    };
                    Ok(Predicate::AttributeEquals(name, value))
                } else {
                    Ok(Predicate::AttributeExists(name))
                }
            }
            _ => Err(QueryError::InvalidXPath {
                message: "expected a predicate expression (position, last(), or @attr)".to_string(),
                position: self.position(),
            }),
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), QueryError> {
        if *self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(QueryError::InvalidXPath {
                message: format!("expected {expected:?}"),
                position: self.position(),
            })
        }
    }
}
