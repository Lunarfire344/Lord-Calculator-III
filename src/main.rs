use std::cell::{RefCell};
use text_io::read;
use std::mem;
use std::cmp;
use std::collections::VecDeque;
use std::rc::Rc;
use regex::Regex;
use crate::Node::{Branching, Terminal};
use crate::TokenCategory::{Constant, Operator, Parenthesis, Variable};

const OPERATOR_PRECEDENCE_KEYS: [char; 6] = ['^', '*', '/', '+', '-', '='];
const OPERATOR_PRECEDENCE_VALUES: [i32; 6] = [2,1,1,0,0,-1];
fn main() {
    let mut tokens: Vec<Token> = Vec::with_capacity(10);

    test_validation();
    let mut user_input: String = read!("{}\n");
    user_input.retain(|c| !c.is_whitespace());
    let mut input = user_input.as_str();
    let mut unique_variables: Vec<String> = Vec::new();
    if !validate_expression(&input){
        panic!("Invalid expression");
    }

    {
        let mut current_token: Token;
        while let Some(category) = get_token_category(input) {
            println!("Remaining untokenized data: {input}");
            (current_token, input) = build_token_of_type(category.clone(), &input);

            if category == Variable && !unique_variables.contains(&current_token.token) {
                unique_variables.push(current_token.token.clone());
            }
            tokens.push(current_token);

        }
    }
    let unique_variables = unique_variables;

    //Handle implicit operations
    {
        let mut i = 0;
        while i <tokens.len()
        {
            if i > 0 && tokens[i].token_type == Variable && (tokens[i - 1].token_type == Variable || tokens[i-1].token_type == Constant) {
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
        print!("{},", *temp);
    }
    println!();

    let mut operand_stack: VecDeque<Node> = VecDeque::with_capacity(tokens.len());
    let mut operator_stack: VecDeque<Token> = VecDeque::with_capacity(5);

    {
        let mut loop_index: usize = 0;
        let mut debug_index:usize = 0;
        let mut vec_shifted: bool = false;

        while loop_index < tokens.len() {
            println!();
            println!("{}", debug_index);
            debug_index += 1;
            match tokens[loop_index].token_type {
                Constant | Variable => {
                    operand_stack.push_back(Terminal(tokens.remove(loop_index), 0));
                    vec_shifted = true;
                }
                Operator => {
                    let incoming_operator = tokens.remove(loop_index);
                    operator_stack = process_operator(operator_stack, incoming_operator, &mut operand_stack);
                    vec_shifted = true;
                }
                Parenthesis(true) => {
                    operator_stack.push_back(tokens.remove(loop_index));
                    vec_shifted = true;
                }
                Parenthesis(false) => {
                    while let Some(top_token) = operator_stack.pop_back() {
                        if top_token.token_type == Parenthesis(true) {
                            break;
                        }
                        let child1: Node = operand_stack.pop_back().unwrap();
                        let child2: Node = operand_stack.pop_back().unwrap();
                        push_new_node_to_stack(&mut operand_stack, top_token, child1, child2,0)
                    }
                }
            }
            loop_index +=  if vec_shifted {0} else {1};
            vec_shifted = false;
            for i in &operand_stack {
                print_tree(i, &mut String::from(""), true);
            }
        }
    }

    while operator_stack.len() > 0{
        let child1:Node = operand_stack.pop_back().unwrap();
        let child2:Node =  operand_stack.pop_back().unwrap();
        push_new_node_to_stack(&mut operand_stack, operator_stack.pop_back().unwrap(), child1, child2,0)
    }

    //Rebuild the tree to index it. This feels incredibly inefficient.
    let mut root_node = index_tree(operand_stack.pop_front().unwrap(), 0, false, true);

    //Tree successfully constructed
    print_tree(&root_node, &mut String::from(""), true);

    if unique_variables.len() == 0{
        //No variables, evaluate as normal.
        println!("{}", evaluate_tree(&root_node));
        return;
    }

    root_node = combine_all_variables(root_node, &unique_variables);
    print_tree(&root_node, &mut String::from(""), true);
    todo!("This code currently assumes ALL operators are commutative.");
    let simplifiable_nodes = Rc::new(RefCell::new(Vec::<usize>::new()));
    get_simplifiable_nodes(&root_node, Rc::clone(&simplifiable_nodes));

    for index in simplifiable_nodes.borrow().iter(){
        let index = *index;
        let node_value = evaluate_tree(get_node_by_index(&root_node, index));
        root_node = replace_node_at_index(root_node, Terminal(Token { token_type: Constant, token: node_value.to_string() }, index), index, get_position_of_first_one(index));
        print_tree(&root_node, &mut String::from(""), true);
    }

    //STEPS TO SOLVE FOR VARIABLES
    //1: Simplify any constants and combine variables
    //1a If a branching node has one constant child and one branching child which contains the same operator as itself, and that branching child has one constant and one variable as operands, the first branching node can be replaced
    //with a new branching node with a constant child containing the result of the original node's operator applied to the two constants, and the other child containing the variable.
    //1b To simplify constant operations, identify the highest point on the tree with no variable descendants, and then replace that node with a constant containing the return value of evaluate_tree() when called on that node.
    //1.5 Account for the commutative property
    //2: Apply the distributive property
    //3: Isolate (somehow)

    //To implement the commutative property, use the same pass when identifying simplifiable nodes to flag all un-simplifiable nodes. All un-simplifiable nodes can either be simplified to coefficient-variable(C-V) form, or will require the commutative property.
    //NOTE: Multiplication doesn't disqualify a branch for this, so long as exactly one of the operands is a variable.
}

fn get_position_of_first_one(number:usize) -> usize{
    for i in (0..usize::BITS).rev(){
        if number & (1 << i) > 0 {
            return usize::try_from(i).unwrap()
        }
    }
    panic!("Function should not be called on the number zero")
}

fn validate_expression(input: &str) -> bool{
    let rules : [Regex; 7] = [
        Regex::new("\\( *\\)").unwrap(),
        Regex::new("[^a-z A-Z0-9.+\\-*/^()=<>]").unwrap(),
        Regex::new("[+\\-*/^]{2}").unwrap(),
        Regex::new("\\([+\\-*/^]").unwrap(),
        Regex::new("[a-z A-Z0-9.] +[a-z A-Z0-9.]").unwrap(),
        Regex::new("[a-z A-Z0-9.+\\-*/^()]+[=><]$").unwrap(),
        Regex::new("^[=><][a-zA-Z0-9.+\\-*/^()]+").unwrap()
    ] ;
    for rule in rules{
        if rule.is_match(input){
            return false
        }
    }

    validate_parenthesis(input)
}

///is the node in the form n * variable
fn is_node_variable_coefficient(node:&Node) -> bool{
    if let Branching(token, child_1, child_2, ..) = node{
        if token.token != "*" {return false}

        if check_this_and_that(|node| -> bool {node.get_token().token_type == Constant},|node| -> bool {node.get_token().token_type == Variable}, child_1, child_2){
            return true;
        }

        if check_this_and_that(|node| -> bool {node.get_token().token_type == Constant},|node| -> bool {is_node_variable_exponent(&node)}, child_1, child_2){
            return true;
        }
    };
        false
}

fn is_node_variable_exponent(node:&Node) -> bool{
    if let Branching(token, child_1, child_2, _) = node{
        if token.token != "^" {return false;}
        return child_1.get_token().token_type == Variable;
    }
    return false;
}

///is the node equivalent to 2 * variable
fn is_node_variable_sum(node:&Node) -> bool{
    if let Branching(token, child_1, child_2, ..) = node{
        if token.token != "+" && token.token != "-" {return false};

        if child_1.get_token().token_type != Variable || child_2.get_token().token_type != Variable {return false};

        return child_1.get_token().token == child_2.get_token().token
    }

    false
}

fn combine_nodes(mut root_node: Node, index1:usize, index2: usize) -> Node
{
    let token1: Token;
    let token2: Token;
    let variable_number_1: Option<f32>;
    let variable_number_2: Option<f32>;
    {
        let node1 = get_node_by_index(&root_node, index1);
        let node2 = get_node_by_index(&root_node, index2);
        token1 = node1.get_token().clone();
        token2 = node2.get_token().clone();

        variable_number_1 = node1.get_variable_coefficient();
        variable_number_2 = node2.get_variable_coefficient();
    }

    if token1.token_type == Constant && token2.token_type == Constant{
        let number= token1.token.parse::<f32>().unwrap() + token2.token.parse::<f32>().unwrap();
        root_node = replace_node_at_index(root_node, construct_constant(number, index1), index1, get_position_of_first_one(index1));
    }

    let variable = if token1.token_type == Variable {token1} else {token2};

    root_node = replace_node_at_index(root_node, construct_variable_coefficient_node(variable, variable_number_1.unwrap() + variable_number_2.unwrap(), index1), index1, get_position_of_first_one(index1));
    let new_node = get_identity_node(get_parent_of_node(&root_node, index2), index2);

    return replace_node_at_index(root_node, new_node, index2, get_position_of_first_one(index2))
}

fn combine_all_variables(mut root_node:Node, variables:&Vec<String>) -> Node{
    let tree_iterator = TreeIterator{root:&root_node, current:Some(&root_node), unexplored:VecDeque::new()};
    let mut result: Vec<Vec<usize>> = Vec::with_capacity(variables.len());
    for _ in 0..variables.len(){
        result.push(Vec::new());
    }

    for node in tree_iterator{
        let variable_index = variables.iter().position(|thing| *thing == node.get_token().token);
        if let Some(index) = variable_index {
            let node_index = node.get_index();
            result[index].push(
                if is_node_variable_coefficient(get_parent_of_node(&root_node, node_index)) {
                    node_index / 2} else {node_index }
            );
        }
    };

    for mut result_set in result{
        while result_set.len() > 1{
            root_node = combine_nodes(root_node, result_set.swap_remove(0), result_set.swap_remove(0))
        }
    }
    root_node
}

fn construct_variable_coefficient_node(variable:Token, coefficient:f32, index:usize) -> Node{
    let constant_child = Box::from(construct_constant(coefficient, index * 2));
    let variable_child = Box::from(Terminal(variable, index * 2 + 1));
    let new_token = Token{token_type:Operator, token: String::from("*")};
    Branching(new_token, constant_child,variable_child, index)
}

fn get_parent_of_node(root_node: &Node, index:usize) -> &Node {
    get_node_by_index(root_node, index/ 2 )
}

///Return the node that will make the parent node an identity
fn get_identity_node(parent_node: &Node, index:usize) -> Node{
    if let Branching(token, ..) = parent_node{
        let required_number = match token.token.as_str(){
            "+" | "-" => 0f32,
            "*" | "/" | "^" => 1f32,
            &_ => unreachable!("Function called on non-operator node"),
        };
        return construct_constant(required_number, index)
    }
    unreachable!("Never call this function on a terminal node")
}

fn construct_constant(value:f32, index:usize) -> Node{
    Terminal(Token{token_type:Constant, token:value.to_string()}, index)
}

///This function assumes that its input will contain only matching variables
fn get_variable_combination_result(first_node:&Node, second_node:&Node) -> Node {
    let final_coefficient: i32 =
        match first_node {
            Terminal(..) => { 1 },
            Branching(..) => {
                if is_node_variable_sum(first_node) { 2 } else {
                    first_node.get_child_of_type(Constant).unwrap().get_token().token.parse().unwrap()
                }
            }
        } +
            match second_node {
                Terminal(..) => { 1 },
                Branching(..) => {
                    if is_node_variable_sum(second_node) { 2 } else {
                        second_node.get_child_of_type(Constant).unwrap().get_token().token.parse().unwrap()
                    }
                }
            };

    let constant_child = Terminal(Token{token_type:Constant, token:final_coefficient.to_string()}, first_node.get_index() * 2);
    let variable_child = Terminal(first_node.get_token().clone(), first_node.get_index() * 2 + 1);
    let new_token = Token{token_type:Operator, token: String::from("*")};
    Branching(new_token, Box::from(constant_child),Box::from(variable_child), first_node.get_index())
}

type NodeCheck = fn(&Node) -> bool;
///Check if condition1 applies to child1 and condition2 applies to child2 or vice versa
fn check_this_and_that(condition1:NodeCheck, condition2:NodeCheck, node_1:&Node, node_2:&Node) -> bool{
    (condition1(node_1) && condition2(node_2)) || (condition2(node_1) && condition1(node_2))
}

fn push_new_node_to_stack(stack: &mut VecDeque<Node>, node_token: Token, child1:Node, child2:Node, node_index:usize){
    stack.push_back(Branching(node_token, Box::from(child2), Box::from(child1), node_index));
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

fn index_tree<'a>(original_tree:Node, parent_index:usize, is_left:bool, is_root: bool)-> Node{
    let self_index: usize = if is_root {1} else {parent_index * 2 + if is_left {0} else {1}};
    match original_tree{
        Branching(token, child_1, child_2, ..) =>{
            Branching(token,
                      Box::from(index_tree(*child_1, self_index, true, false)), Box::from(index_tree(*child_2, self_index, false, false)),
                      self_index)
        }
        Terminal(token, ..) => {
            Terminal(token, self_index)
        }
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

fn get_simplifiable_nodes(parent_node: &Node, found_nodes: Rc<RefCell<Vec<usize>>>) -> bool {
    //Check children
    //If one is a variable, return false
    //If both are constants, return true
    if let Branching(_, child_1, child_2, _) = parent_node{
        let child0valid: bool =
            match &**child_1{
                Terminal(token, _) => {token.token_type == Constant}
                Branching(..) => {
                    get_simplifiable_nodes(&child_1, Rc::clone(&found_nodes))
                }
            };

        let child1valid: bool =
            match &**child_2{
                Terminal(token, _) => {token.token_type == Constant}
                Branching(..) => {
                    get_simplifiable_nodes(&child_2, Rc::clone(&found_nodes))
                }
            };

        if child0valid && !child1valid {
            found_nodes.borrow_mut().push(child_1.get_index());
        }

        if !child0valid && child1valid{
            found_nodes.borrow_mut().push(child_2.get_index());
        }

        return child0valid && child1valid;
    }

    unreachable!("This function should never be called on a non-branching node");
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
    println!("{}", node.get_token().token.clone());

    if let Branching(_, child_1, child_2, _) = node{
        print_tree(&child_1, &local_indent, false);
        print_tree(&child_2, &local_indent, true);
    }
}

fn get_token_category_of_character (c: &char) -> Option<TokenCategory> {
    match c{
        '0'..='9' | '.' => Some(Constant),
        'a'..='z' | 'A'..='Z' => Some(Variable),
        ' ' => None,
        '+' | '-' | '*' | '/' | '^' | '=' => Some(Operator),
        '(' | ')' => Some(Parenthesis(*c == '(')),
        _ => {
            println!("Unexpected character: {}", *c as u32);
            None
        }
    }
}

fn get_node_by_index(root_node: &Node, index:usize) -> &Node {
    let thing= (0..usize::BITS).rev().map(|n| (index >> n) & 1);
    let mut first_one_found: bool = false;
    let mut current_node: &Node = root_node;
    for bit in thing{
        if !first_one_found{
            first_one_found = bit == 1;
            continue;
        };

        if let Branching(_, child_1,child_2, ..) = current_node {
            current_node = if bit == 1 {&child_2} else {&child_1}
        }
        else{
            panic!("This code should never run")
        }
    };
    current_node
}

fn evaluate_tree (node: &Node) -> f32{
    match node{
        Branching(token, child_1, child_2, ..) =>{
            match token.token.as_str() {
                "+" =>{
                    evaluate_tree(&child_1) + evaluate_tree(&child_2)
                }
                "-" =>{
                    evaluate_tree(&child_1) - evaluate_tree(&child_2)
                }
                "*" =>{
                    evaluate_tree(&child_1) * evaluate_tree(&child_2)
                }
                "/" =>{
                    evaluate_tree(&child_1) / evaluate_tree(&child_2)
                }
                "^" =>{
                    evaluate_tree(&child_1).powf(evaluate_tree(&child_2))
                }
                _ =>{
                    panic!("Branching nodes should only contain operators")
                }
            }
        }
        Terminal(token, ..) => {
            extract_variable_or_constant(token)
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

fn process_operator<'a>(mut operator_stack: VecDeque<Token>, incoming: Token, operands: &mut VecDeque<Node>) -> VecDeque<Token>{
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
                push_new_node_to_stack(operands, operator_stack.pop_back().unwrap(), child1, child2, 0);
                operator_stack = process_operator(operator_stack, incoming, operands);
            }
            cmp::Ordering::Equal =>{
                println!("Operator of equal precedence");
                let child1:Node = operands.pop_back().unwrap();
                let child2:Node =  operands.pop_back().unwrap();
                push_new_node_to_stack(operands, operator_stack.pop_back().unwrap(), child1, child2, 0);
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

fn replace_node_at_index<'a>(root_node:Node, new_node:Node, index:usize, path_position:usize) -> Node {
    if path_position == 0{
        return new_node
    }
    let path_position = path_position - 1;
    if let Branching(token, child_1, child_2, current_index) = root_node {
        let bit =index & (1 << path_position) > 0;
        let (mut keep_child, mut change_child) = (*child_1, *child_2);

        let new_children = if bit == false{
            (keep_child, change_child) = (change_child, keep_child);
            (replace_node_at_index(change_child, new_node, index, path_position), keep_child)
        }
        else{
            (keep_child, replace_node_at_index(change_child, new_node, index, path_position))
        };

        Branching(token,
                  Box::from(new_children.0), Box::from(new_children.1),
                  current_index)
    } else {
        unreachable!("Reached end of tree before end of path")
    }
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
    Parenthesis(bool),
}

#[derive(Clone)]
struct Token<>{
    token_type: TokenCategory,
    token: String
}

#[derive(Clone)]
enum Node {
    Branching(Token, Box<Node>, Box<Node>, usize),
    Terminal(Token, usize)
}

impl Node{
    fn get_token(&self) -> &Token{
        match self{
            Branching(token, ..) => token,
            Terminal(token, _) => token
        }
    }
    fn get_index(&self) -> usize{
        match self{
            Branching(_,_,_,index) => *index,
            Terminal(_, index) => *index
        }
    }
    fn is_terminal(&self) -> bool{
        match self{
            Terminal(..) => true,
            Branching(..) => false
        }
    }
    fn get_child_of_type(&self, category:TokenCategory) -> Option<&Node>{
        match self {
            Terminal(..) => None,
            Branching(_, child_1, child_2,_) =>{
                if child_1.get_token().token_type == category {Some(child_1)}
                else if child_2.get_token().token_type == category {Some(child_2)}
                else {None}
            }
        }
    }
    fn get_variable_coefficient(&self) -> Option<f32>{
        match self{
            Terminal(token, ..) => {if token.token_type == Variable {Some(1f32)} else {None}},
            Branching(token, ..) =>{
                if token.token != "*" {None} else{ Some(self.get_child_of_type(Constant).unwrap().get_token().token.parse().unwrap())}
            }
        }
    }
}

struct TreeIterator<'a>{
    root: &'a Node,
    current: Option<&'a Node>,
    unexplored: VecDeque<&'a Node>
}

impl<'a> Iterator for TreeIterator<'a>{
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item>{
        let current = self.current;
        if let Some(current_node) = current {
            match current_node {
                Branching(_, child_1, child_2, _) => {
                    self.current = Some(&child_1);
                    self.unexplored.push_back(&child_2);
                }
                Terminal(..) => {
                    self.current = self.unexplored.pop_back();
                }
            }
        }

        current
    }
}

#[derive(Clone)]
struct BranchingNode{
    children: (Node, Node)}

struct TreeBranch{
    start: usize,
    ends:Vec<usize>
}

