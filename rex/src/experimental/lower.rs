use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Expr {
    Var(usize),
    App(Box<Expr>, Box<Expr>),
    Lambda(Box<Expr>),
}

pub struct SuperCombinator {
    pub args: usize,
    pub cont: (),
    pub body: CoreExpr,
}

pub enum CoreExpr {
    Var(usize),
    App(Box<CoreExpr>, Vec<CoreExpr>),
    Lambda(usize, Box<CoreExpr>),
}

pub fn lift_expr(expr: Expr) -> CoreExpr {
    match expr {
        Expr::Var(var) => CoreExpr::Var(var),
        Expr::App(left, right) => {
            let mut lambda = *left;

            let right_lifted = lift_expr(*right);
            let mut args = vec![right_lifted];

            while let Expr::App(left, right) = lambda {
                lambda = *left;
                let right_lifted = lift_expr(*right);
                args.push(right_lifted);
            }

            let core_lambda = lift_expr(lambda);

            CoreExpr::App(Box::new(core_lambda), args)
        }
        Expr::Lambda(inner) => {
            let mut num_args = 1;
            let mut body = *inner;

            while let Expr::Lambda(inner) = body {
                num_args += 1;
                body = *inner;
            }

            let core_body = lift_expr(body);

            CoreExpr::Lambda(num_args, Box::new(core_body))
        } // recursive case,
    }
}

// pub fn cps_transform(expr: CoreExpr, cont: CoreExpr) -> CoreExpr {
//     match expr {
//         CoreExpr::Var(i) => CoreExpr::App(Box::new(cont), vec![CoreExpr::Var(i)]),
//         CoreExpr::App(f, args) => {}
//     }
// }

// fn compile(expr: &Expr, code: &mut Vec<Instruction>) {
//     match expr {
//         Expr::Var(i) => code.push(Instruction::LoadVar(*i)),
//         Expr::App(f, x) => {
//             compile(x, code);
//             compile(f, code);
//             code.push(Instruction::Call)
//         }
//         Expr::Lambda(body) => {
//             let start = code.len();
//             compile(body, code);
//             code.push(Instruction::Return);
//         }
//     }
// }

// a stack frame for a new function

#[derive(Debug, Clone)]
pub enum Instruction {
    LocalGet(usize),
    I32Const(i32),
    I32Add,
    Call(usize),
    MakeClosure(usize),
    Print,
    Return,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub locals: Vec<Atom>,      // local environment
    pub code: Vec<Instruction>, // instructions to execute
    pub pc: usize,              // program counter
}

#[derive(Debug, Clone)]
pub struct Closure {
    code: Vec<Instruction>,
    env: Vec<Atom>,
}

#[derive(Debug, Clone)]
pub enum Atom {
    Int(i32),
    Closure(Box<Closure>),
}

#[derive(Debug, Clone)]
pub struct VM {
    // This is the stack that instructions use directly for example "add" pops two arguments and
    // pushes on the result
    pub operand_stack: Vec<Atom>,

    // This is the normal call stack
    pub call_stack: Vec<Frame>,
}

impl VM {
    pub fn new() -> Self {
        Self {
            operand_stack: vec![],
            call_stack: vec![],
        }
    }

    pub fn run(&mut self, mut instrs: Vec<Instruction>) {
        let mut pc = 0;
        while pc < instrs.len() {
            match &instrs[pc] {
                // TODO: understand how this works
                Instruction::LocalGet(i) => {
                    let val = self.call_stack.last().unwrap().locals[*i].clone();
                    self.operand_stack.push(val);
                    pc += 1;
                }
                // This is trivial
                Instruction::I32Const(n) => {
                    self.operand_stack.push(Atom::Int(*n));
                    pc += 1;
                }
                Instruction::Call(n) => {
                    if let Atom::Closure(closure) = self.operand_stack.pop().unwrap() {
                        let args = self.operand_stack.split_off(self.operand_stack.len() - n);
                        let mut new_locals = closure.env.clone();
                        new_locals.extend(args);
                        self.call_stack.push(Frame {
                            locals: new_locals,
                            code: closure.code.clone(),
                            pc: 0,
                        });
                        // switch execution to new frame
                        // TODO: understand
                        let frame = self.call_stack.last_mut().unwrap();
                        instrs = frame.code.clone();
                        pc = frame.pc;
                    } else {
                        panic!("Call on non-closure");
                    }
                }
                // TODO: understand
                Instruction::MakeClosure(n) => {
                    if let Atom::Closure(mut closure) = self.operand_stack.pop().unwrap() {
                        let args = self.operand_stack.split_off(self.operand_stack.len() - n);
                        closure.env.extend(args);
                        self.operand_stack.push(Atom::Closure(closure));
                        pc += 1;
                    } else {
                        panic!("MakeClosure on non-closure");
                    }
                }

                // TODO: do we need this at all?
                Instruction::Return => {
                    if let Some(frame) = self.call_stack.pop() {
                        instrs = frame.code;
                        pc = frame.pc;
                    } else {
                        break; // end of program
                    }
                }
                // trivial
                Instruction::I32Add => {
                    let b = if let Atom::Int(i) = self.operand_stack.pop().unwrap() {
                        i
                    } else {
                        panic!("Expected int")
                    };
                    let a = if let Atom::Int(i) = self.operand_stack.pop().unwrap() {
                        i
                    } else {
                        panic!("Expected int")
                    };

                    self.operand_stack.push(Atom::Int(a + b))
                }

                // trivial
                Instruction::Print => {
                    println!("{:?}", self.operand_stack.pop().unwrap());
                }
            }
        }
    }
}
