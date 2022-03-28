use crate::models::{Range, State, Token, WCode, WFuncVariant, WTokens};
use crate::utils::{first_special_instance, last_function, special_pairs, WFunc};
use itertools::Itertools;
use rayon::prelude::*;

pub trait WEval {
    fn wsection_eval(&mut self, data: Vec<WCode>) -> Vec<WTokens>;
    fn eval(&self, data: WTokens) -> WTokens;
}

impl WEval for State {
    fn wsection_eval(&mut self, data: Vec<WCode>) -> Vec<WTokens> {
        let mut result: Vec<WTokens> = Vec::new();

        for section in data {
            match section.container {
                Some(container) => {
                    let mut cases = vec![];

                    if let Some(container_cases) = section.cases {
                        cases.append(&mut container_cases.clone())
                    }

                    cases.push((vec![Token::Value(1.0)], section.default_case));

                    self.insert(container, cases);
                }
                None => result.push(self.eval(section.default_case)),
            }
        }

        result
    }

    fn eval(&self, data: WTokens) -> WTokens {
        let mut new_code = data.clone();

        while let Some((first_func_pos, func)) = last_function(&new_code) {
            let first = first_special_instance("(".to_string(), &new_code);
            let second = match first {
                Some(x) => special_pairs(("(".to_string(), ")".to_string()), &new_code, &x),
                None => None,
            };

            if let (Some(x), Some(y)) = (first, second) {
                new_code.splice(x..=y, self.eval(new_code[x+1..y].to_vec()));
            } else {
                let code_to_evaluate: WTokens = new_code[..first_func_pos].to_vec();

                match func {
                    WFuncVariant::Function(func) => {
                        let result = func(code_to_evaluate);
                        new_code.splice(..first_func_pos + 1, result);
                    }
                    WFuncVariant::Container(x) => {
                        let mut case: WTokens = vec![];
                        let mut container_acc: WTokens = vec![];

                        for container_case in self.get(&x).unwrap() {
                            let mut joined = container_case.0.clone();
                            joined.append(&mut container_case.1.clone());
                            container_acc.append(&mut joined);

                            let case_prefix =
                                self.eval(self.apply(&container_case.0, &code_to_evaluate));

                            if case_prefix[0] != Token::Value(0.0) {
                                case = container_case.1.clone();
                                break;
                            }
                        }

                        let expanded_range = container_acc
                            .par_iter()
                            .filter(|x| matches!(x, Token::Parameter(_)))
                            .map(|range| match range {
                                Token::Parameter(Range::Full(full)) => {
                                    full.clone().collect::<Vec<_>>()
                                }
                                Token::Parameter(Range::From(from)) => {
                                    (0..=from.end).collect::<Vec<_>>()
                                }
                                Token::Parameter(Range::To(to)) => {
                                    (to.start..=code_to_evaluate.len() - 1).collect::<Vec<_>>()
                                }
                                _ => panic!(),
                            })
                            .flatten()
                            .collect::<Vec<_>>()
                            .iter()
                            .unique()
                            .map(|wlang_index| code_to_evaluate.len() - (wlang_index + 1))
                            .sorted()
                            .rev()
                            .collect::<Vec<usize>>();

                        let result = self.apply(&case, &code_to_evaluate);
                        new_code.splice(first_func_pos..=first_func_pos, result);
                        for n in expanded_range {
                            new_code.remove(n);
                        }
                    }
                }
            }
        }

        new_code
    }
}
