#[derive(Debug, PartialEq)]
pub struct Function<'input> {
    pub name: &'input str,
    pub args: Vec<&'input str>,
    pub block: Block<'input>,
}

pub type Program<'input> = Vec<Function<'input>>;

#[derive(Debug, PartialEq)]
pub enum BlockElement<'input> {
    LetStatement {
        name: &'input str,
        expression: Expression<'input>,
    },
    AssignmentStatement {
        name: &'input str,
        expression: Expression<'input>,
    },
    ReturnStatement(Expression<'input>),
    NestedBlock(Block<'input>),
}

pub type Block<'input> = Vec<BlockElement<'input>>;

#[derive(Debug, PartialEq)]
pub struct FunctionCall<'input> {
    pub name: &'input str,
    pub args: Vec<Expression<'input>>,
}

#[derive(Debug, PartialEq)]
pub enum Expression<'input> {
    Identifier(&'input str),
    Number(i64),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Sub(Box<Self>, Box<Self>),
    Mul(Box<Self>, Box<Self>),
    Div(Box<Self>, Box<Self>),
    Pow(Box<Self>, Box<Self>),
    Rem(Box<Self>, Box<Self>),
    FunctionCall(FunctionCall<'input>),
}
