use std::io;
use std::mem;
use std::cmp;
use std::collections::VecDeque;
use regex::Regex;
use crate::Node::{Branching, Terminal};
use crate::TokenCategory::{Constant, Operator, Parenthesis, Variable};

const OPERATOR_PRECEDENCE_KEYS: [char; 5] = ['^', '*', '/', '+', '-'];
const OPERATOR_PRECEDENCE_VALUES: [i32; 5] = [2,1,1,0,0];
fn main() {
    let mut tokens: Vec<Token> = Vec::with_capacity(10);

    //TODO: Calculator does not support negative numbers
    test_validation();
    let mut input = "-3 * -1";
    {
        let mut current_token: Token;
        //Should output 7773.15625
        while let Some(category) = get_token_category(input) {
            println!("Remaining untokenized data: {input}");
            (current_token, input) = build_token_of_type(category, &input);

            tokens.push(current_token);
        }
    }


    //Handle implicit operations
    {
        let mut i = 0;
        while i <tokens.len()
        {
            if i > 0 && tokens[i].token_type == Variable && tokens[i - 1].token_type == Variable {
                tokens.insert(i, Token{token_type: Operator, token:'*'.to_string()});
                //Skip forward a token, as the next token in the list is now the one that we just checked.
                i += 1;
            }
            if i < (tokens.len() - 1) && (tokens[i].token_type == Variable || tokens[i].token_type == Constant) && tokens[i+1].token_type == Parenthesis(true){
                tokens.insert(i+1, Token{token_type: Operator, token:'*'.to_string()})
            }
            if i == 0 && tokens[i].token == "-"{
                tokens.insert(0, Token{token_type:Constant, token:"0".to_string()})
            }
            if i > 0 && tokens[i].token == "-" && (tokens[i-1].token_type == Operator || tokens[i-1].token_type == Parenthesis(true)){
                tokens.insert(i, Token{token_type:Constant, token:"0".to_string()});
                tokens.insert(i, Token{token_type:Parenthesis(true), token:"(".to_string()});
                tokens.insert(i+4, Token{token_type: Parenthesis(false), token:")".to_string()});
                i+= 1;
            }
            i += 1;
        }
    }

    println!("Tokens:");
    for token in &tokens{
        let temp = &token.token;
        println!("{}", *temp);
    }

    // let mut experiment = VecDeque::from(['a']);
    // experiment.push_back('b');
    // experiment.push_front('a');
    // for x in 0..experiment.len(){
    //     print!("{}", experiment[x])
    // }

    // Assume that the token vector will NOT be modified after this point.
    let tokens = tokens;
    let mut operand_stack: VecDeque<Node> = VecDeque::with_capacity(tokens.len());
    let mut operator_stack: VecDeque<&Token> = VecDeque::with_capacity(5);
    for i in 0..tokens.len() {
        match tokens[i].token_type{
            Constant | Variable => {
                operand_stack.push_back(Terminal(&tokens[i]));
            }
            Operator => {
                let incoming_operator = &tokens[i];
                operator_stack = process_operator(operator_stack, incoming_operator, &mut operand_stack);
            }
            Parenthesis(true) =>{
                operator_stack.push_back(&tokens[i])
            }
            Parenthesis(false) => {
                while let Some(top_token) = operator_stack.pop_back() {
                    if top_token.token_type == Parenthesis(true) {
                        break;
                    }
                    let child1:Node = operand_stack.pop_back().unwrap();
                    let child2:Node =  operand_stack.pop_back().unwrap();
                    operand_stack.push_back(Branching(top_token, Box::from(BranchingNode { children: (child2, child1)})));
                }
            }
        }
    }

    while operator_stack.len() > 0{
        let child1:Node = operand_stack.pop_back().unwrap();
        let child2:Node =  operand_stack.pop_back().unwrap();
        operand_stack.push_back(Branching(operator_stack.pop_back().unwrap(), Box::from(BranchingNode { children: (child2, child1)})));
    }

    //Tree successfully constructed
    print_tree(&operand_stack.front().unwrap(), &mut String::from(""), true);
    println!("{}", evaluate_tree(&operand_stack.front().unwrap()))
}

fn validate_expression(input: &str) -> bool{
    let rules : [Regex; 5] = [
        Regex::new("( *)").unwrap(),
        Regex::new("[^a-z A-Z0-9.+\\-*/^()]").unwrap(),
        Regex::new("[+\\-*/^]{2}").unwrap(),
        Regex::new("\\([+\\-*/^]").unwrap(),
        Regex::new("[a-z A-Z0-9.] +[a-z A-Z0-9.]").unwrap()
    ] ;
    for rule in rules{
        if rule.is_match(input){
            return false
        }
    }

    validate_parenthesis(input)
}
fn test_validation(){
    let equations: [&str; 6] = [
        "5 + (  )",
        "3 * 4 ^@",
        "6 - 8 * 3 ++ 2",
        "7 * (+3)",
        "124 x",
        "3 + (4 * "
    ];

    for equation in equations{
        println!("{}", validate_expression(equation))
    }
}

fn validate_parenthesis(input: &str) -> bool{
    let mut number: i32 = 0;
    for x in input.chars(){
        match x {
            '(' => { number += 1;}
            ')' => {number -= 1}
            _ => {}
        }

        if number < 0{
            return  false
        }
    }

    number == 0
}

fn print_tree(node: &Node, indent: &String, last: bool){
    print!("{}", indent);
    let mut local_indent: String = indent.into();
    if last{
        print!("\\->");
        local_indent.push_str("   ");
    }
    else{
        print!("|->");
        local_indent.push_str("|  ");
    }
    println!("{}", node.get_token().token);

    if let Branching(_, children) = node{
        print_tree(&children.children.0, &local_indent, false);
        print_tree(&children.children.1, &local_indent, true);
    }
}

fn get_token_category_of_character (c: &char) -> Option<TokenCategory> {
    match c{
        '0'..='9' | '.' => Some(Constant),
        'a'..='z' | 'A'..='Z' => Some(Variable),
        ' ' => None,
        '+' | '-' | '*' | '/' | '^' => Some(Operator),
        '(' | ')' => Some(Parenthesis(*c == '(')),
        _ => {
            println!("Unexpected character: {c}");
            None
        }
    }
}

fn evaluate_tree (node: &Node) -> f32{
    match node{
        Branching(token, next_nodes) =>{
            match token.token.as_str() {
                "+" =>{
                    evaluate_tree(&next_nodes.children.0) + evaluate_tree(&next_nodes.children.1)
                }
                "-" =>{
                    evaluate_tree(&next_nodes.children.0) - evaluate_tree(&next_nodes.children.1)
                }
                "*" =>{
                    evaluate_tree(&next_nodes.children.0) * evaluate_tree(&next_nodes.children.1)
                }
                "/" =>{
                    evaluate_tree(&next_nodes.children.0) / evaluate_tree(&next_nodes.children.1)
                }
                "^" =>{
                    evaluate_tree(&next_nodes.children.0).powf(evaluate_tree(&next_nodes.children.1))
                }
                _ =>{
                    panic!("Characters should only be operator")
                }
            }
        },
        Terminal(token) => {
            extract_variable_or_constant(*token)
        }
    }
}

fn extract_variable_or_constant(token: &Token) -> f32{
    match token.token_type{
        Variable => /* idk */ 1f32,
        Constant => token.token.parse().unwrap(),
        _ => panic!("Input was neither a constant nor a variable")
    }
}

fn process_operator<'a>(mut operator_stack: VecDeque<&'a Token>, incoming: &'a Token, operands: &mut VecDeque<Node<'a>>) -> (VecDeque<&'a Token>){
    if let Some(last_operator_option) = operator_stack.back(){
        // println!("{x}");
        //Treat a left parenthesis as empty stack
        if get_first_char(&last_operator_option.token).unwrap() == '('{
            operator_stack.push_back(incoming);
            return operator_stack
        }

        match compare_operator_precedence(&get_first_char(&last_operator_option.token).unwrap(), &get_first_char(&incoming.token).unwrap())
        {
            cmp::Ordering::Less =>{
                println!("Operator of lesser precedence found on the stack");
                operator_stack.push_back(incoming);
            }
            cmp::Ordering::Greater =>{
                println!("Operator of greater precedence found on the stack");
                let child1:Node = operands.pop_back().unwrap();
                let child2:Node =  operands.pop_back().unwrap();
                operands.push_back(Branching(operator_stack.pop_back().unwrap(), Box::from(BranchingNode { children: (child2, child1)})));
                operator_stack = process_operator(operator_stack, incoming, operands);
            }
            cmp::Ordering::Equal =>{
                println!("Operator of equal precedence");
                let child1:Node = operands.pop_back().unwrap();
                let child2:Node =  operands.pop_back().unwrap();
                operands.push_back(Branching(operator_stack.pop_back().unwrap(), Box::from(BranchingNode { children: (child2, child1)})));
                operator_stack.push_back(incoming);
            }
        }
        operator_stack
    }
    else{
        operator_stack.push_back(incoming);
        operator_stack
    }
}

fn get_first_char (input: &String) -> Option<char>{
    for i in input.chars(){
        match i{
            '\r' => continue,
            _ => return Some(i)
        }
    }
    None
}

fn compare_operator_precedence(operator1: &char, operator2:&char) -> cmp::Ordering{
    let operator_precedence = OPERATOR_PRECEDENCE_VALUES[
        OPERATOR_PRECEDENCE_KEYS.iter().position(|&r| r.eq(operator1)).unwrap()
        ];
    let token_precedence = OPERATOR_PRECEDENCE_VALUES[
        OPERATOR_PRECEDENCE_KEYS.iter().position(|&r| r.eq(operator2)).unwrap()
        ];

    operator_precedence.cmp(&token_precedence)
}

fn get_token_category(slice: &str) -> Option<TokenCategory>{
    for i in slice.chars(){
        let option = get_token_category_of_character(&i);

        if option.is_some(){
            return option;
        }
    }

    None
}

fn build_token_of_type(category: TokenCategory, data: &str) -> (Token, &str){
    let mut token_string = String::new();
    let mut end_of_current_token: usize = 0;

    for i in data.chars(){
        if let Some(x) = get_token_category_of_character(&i){
            if mem::discriminant(&x) != mem::discriminant(&category){ break;}

            token_string.push(i);
            //Variables and operators are only one character, so we don't need to keep searching
            if category == Variable || category == Operator
            {
                end_of_current_token += 1;
                break;
            }
        }
        end_of_current_token += 1;
    }

    (Token {token_type: category, token:token_string}, &data[end_of_current_token..])
}

// fn print_token_category(category: TokenCategory) {
//     let x =
//     match category{
//         Constant => "Constant",
//         Variable => "Variable",
//         Operator => "Operator",
//         Parenthesis => "Parenthesis"
//     }.to_string();
//      println!("{x}")
// }

#[derive(Eq, PartialEq, Clone)]
enum TokenCategory {
    Constant,
    Variable,
    Operator,
    Parenthesis(bool)
}
struct Token<>{
    token_type: TokenCategory,
    token: String
}

enum Node<'a> {
    Branching(&'a Token, Box<BranchingNode<'a>>),
    Terminal(&'a Token)
}

impl Node<'_>{
    fn get_token(&self) -> &Token{
        match self{
            Branching(token, _) => *token,
            Terminal(token) => *token
        }
    }
}

struct BranchingNode<'a>{
    children: (Node<'a>, Node<'a>),
}
