use std::rc::Rc;

#[derive(Clone, Debug)]
pub enum Expr {
    Var(usize),
    App(Box<Expr>, Box<Expr>),
    Lambda(Box<Expr>),
}

// a stack frame for a new function
struct Closure {
    // Pointer to Combinator
    comb: CodePointer,
    // last one is continuation
    env: Vec<Atom>,
}

// When calling a function this will be allocated
struct Env {
    env: Vec<Atom>,
}

struct Atom;

struct CodePointer(usize);

enum Instr {
    Call { closure: Closure },
}
