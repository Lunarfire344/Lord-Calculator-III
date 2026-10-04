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
    // todo!("Test that is_branch_equal works on nodes with different child reference indices (i.e. they are in different vectors), but the same tokens");
    //It probably doesn't
    let mut tokens: Vec<Token> = Vec::with_capacity(10);

    test_validation();
    // let mut user_input: String = read!("{}\n");
    let mut user_input:String = String::from("1 + 2^4");
    user_input.retain(|c| !c.is_whitespace());
    let mut input = user_input.as_str();
    if !validate_expression(&input){
        panic!("Invalid expression");
    }

    {
        let mut current_token: Token;
        while let Some(category) = get_token_category(input) {
            println!("Remaining untokenized data: {input}");
            (current_token, input) = build_token_of_type(category.clone(), &input);

            tokens.push(current_token);
        }
    }

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

    let mut operand_stack: VecDeque<usize> = VecDeque::with_capacity(tokens.len());
    let mut operator_stack: VecDeque<Token> = VecDeque::with_capacity(5);
    let mut tree: Vec<Node> = Vec::with_capacity(tokens.len());

    {
        let mut token_index: usize = 0;
        let mut loop_index:usize = 0;
        let mut vec_shifted: bool = false;

        while token_index < tokens.len() {
            println!();
            println!("{}", loop_index);
            match tokens[token_index].token_type {
                Constant | Variable => {
                    tree.push(Terminal(tokens.remove(token_index), 0, 0));
                    operand_stack.push_back(tree.len() - 1);
                    vec_shifted = true;
                }
                Operator => {
                    let incoming_operator = tokens.remove(token_index);
                    (operator_stack, tree) = process_operator(operator_stack, incoming_operator, &mut operand_stack, tree);
                    vec_shifted = true;
                }
                Parenthesis(true) => {
                    operator_stack.push_back(tokens.remove(token_index));
                    vec_shifted = true;
                }
                Parenthesis(false) => {
                    while let Some(top_token) = operator_stack.pop_back() {
                        if top_token.token_type == Parenthesis(true) {
                            break;
                        }
                        let child1: usize = operand_stack.pop_back().unwrap();
                        let child2: usize = operand_stack.pop_back().unwrap();
                        tree = push_new_node_to_stack(&mut operand_stack, top_token, child1, child2, 0,tree);
                    }
                }
            }
            token_index +=  if vec_shifted {0} else {1};
            vec_shifted = false;
            loop_index += 1;
            // for i in &operand_stack {
            //     print_tree(i, &mut String::from(""), true);
            // }
        }
    }

    while operator_stack.len() > 0{
        let child1: usize = operand_stack.pop_back().unwrap();
        let child2: usize =  operand_stack.pop_back().unwrap();
        tree = push_new_node_to_stack(&mut operand_stack, operator_stack.pop_back().unwrap(), child1, child2, 0 ,tree)
    }

    //Rebuild the tree to index it. This feels incredibly inefficient.
    let root_index:usize = tree.len()- 1;
    let mut new_vec: Vec<Node> = Vec::with_capacity(tree.len());
    new_vec.resize(tree.len(), tree[0].clone());
    println!("Tree before pruning:");
    print_tree(&tree, root_index, &mut String::from(""), true);
    tree = index_tree(&tree, new_vec, root_index, 1);

    let mut unique_variables: Vec<VariableTerm> = Vec::new();
    {
        let iterator = construct_tree_iterator(&tree, root_index);
        for node_index in iterator {
            let term =  convert_node_to_variable_term(&tree, node_index);
            if !term.is_some() || unique_variables.contains(&term.unwrap()) {continue}

            unique_variables.push(term.unwrap());
        }
    }

    if unique_variables.len() == 0{
        //No variables, evaluate as normal.
        println!("{}", evaluate_tree(&tree, root_index));
        return;
    }

    root_node = combine_all_variables(root_node, &unique_variables);
    println!("Tree after variable commutation:");
    print_tree(&root_node, &mut String::from(""), true);

    let simplifiable_nodes = Rc::new(RefCell::new(Vec::<usize>::new()));
    get_simplifiable_nodes(&root_node, Rc::clone(&simplifiable_nodes));

    // todo!("This code currently assumes ALL operators are commutative.");
    // for index in simplifiable_nodes.borrow().iter(){
    //     let index = *index;
    //     let node_value = evaluate_tree(get_node_by_index(&root_node, index));
    //     root_node = replace_node_at_index(root_node, Terminal(Token { token_type: Constant, token: node_value.to_string() }, index), index, get_position_of_first_one(index));
    //     print_tree(&root_node, &mut String::from(""), true);
    // }

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

// fn vec_contains_identical_node(comparison_vec:&Vec<Node>, node_vec:&Vec<Node>, index:usize) -> bool{
//     for i in 0..comparison_vec.len() {
//         if is_branch_equal_to(comparison_vec, Some(node_vec), i, index) {return true}
//     }
//     false
// }

fn convert_node_to_variable_term(node_vec:&Vec<Node>, index:usize) -> Option<VariableTerm> {
    match &node_vec[index] {
        Terminal(token, _, parent) => {
            if token.token_type == Variable && node_vec[*parent].get_token().token != "^" {
                Some(VariableTerm { variable: token.token.parse().unwrap(), exponent: 1f32 })
            } else { None }
        }
        Branching(token, _, _, _, _) => {
            let variable = get_immediate_child_of_type(node_vec, index, Variable);
            let constant = get_immediate_child_of_type(node_vec, index, Constant);
            if token.token == "^" && let Some(variable) = variable && let Some(constant) = constant {
                Some(VariableTerm { variable: variable.get_token().token.parse().unwrap(), exponent: constant.get_token().token.parse().unwrap() })
            } else {
                None
            }
        }
    }
}

// fn copy_node_to_vec(origin:&Vec<Node>, mut destination:Vec<Node>, index:usize) -> Vec<Node>{
//     match origin[index] {
//         Terminal(..) =>{
//             destination.push(origin[index].clone());
//             destination
//         }
//         Branching(_, child_1,child_2, ..){
//             destination = copy_node_to_vec(origin, destination, child_1);
//             let index_1 = destination.len() - 1;
//             destination = copy_node_to_vec(origin, destination, child_2);
//             let index_2 = destination.len() - 1;
//         }
//     }
// }

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
// fn is_node_variable_coefficient(node:&Node) -> bool{
//     if let Branching(token, child_1, child_2, ..) = node{
//         if token.token != "*" {return false}
//
//         if check_this_and_that(|node| -> bool {node.get_token().token_type == Constant},|node| -> bool {node.get_token().token_type == Variable}, child_1, child_2){
//             return true;
//         }
//
//         if check_this_and_that(|node| -> bool {node.get_token().token_type == Constant},|node| -> bool {is_node_variable_exponent(&node)}, child_1, child_2){
//             return true;
//         }
//     };
//         false
// }
//
// fn is_node_variable_exponent(node:&Node) -> bool{
//     if let Branching(token, child_1, _, _) = node{
//         if token.token != "^" {return false;}
//         //Only check the left child because the exponent base is always the left node
//         return child_1.get_token().token_type == Variable;
//     }
//     false
// }
//
fn combine_nodes(mut node_vec: Vec<Node>, index1:usize, index2: usize) -> Node
{
    let token1: Option<Token>;
    let token2: Option<Token>;
    let variable_number_1: Option<f32>;
    let variable_number_2: Option<f32>;
    let variable:Option<Token>;
    let is_constant:bool;
    {
        let node1 = node_vec[index1];
        let node2 = node_vec[index2];
        let temp_token_1 = node1.get_token();
        let temp_token_2 = node2.get_token();

        variable = if temp_token_1.token_type == Variable { Some(temp_token_1.clone()) }
            else if temp_token_2.token_type == Variable{ Some(temp_token_2.clone()) }
            else if let Some(node) = get_any_child_of_type(&node_vec, index1, Variable) { Some(node.get_token().clone()) }
            else if let Some(other_node) = get_any_child_of_type(&node_vec, index2, Variable) { Some(other_node.get_token().clone()) }
            else {None};
        // todo!("Detect exponents and pass them");
        //It would likely be cleanest to give all variables exponents of one. However, this makes the tree bigger.
        //It might also be wise to pass this function the exponents and variables (via the new struct), rather than calculating them again.
        is_constant = !variable.is_some();

        (token1, token2) = if is_constant {(Some(temp_token_1.clone()), Some(temp_token_2.clone()))} else {(None, None)};

        variable_number_1 = get_variable_coefficient(&node_vec, index1);
        variable_number_2 = get_variable_coefficient(&node_vec, index2);
    }

    if is_constant{
        let number= token1.unwrap().token.parse::<f32>().unwrap() + token2.unwrap().token.parse::<f32>().unwrap();
        root_node = replace_node_at_index(root_node, construct_constant(number, index1), index1, get_position_of_first_one(index1));
    }
    else {
        root_node = replace_node_at_index(root_node, construct_variable_coefficient_node(variable.unwrap(), variable_number_1.unwrap() + variable_number_2.unwrap(), 0f32, index1), index1, get_position_of_first_one(index1));
    }

    let new_node = get_identity_node(get_parent_of_node(&root_node, index2), index2);

    replace_node_at_index(root_node, new_node, index2, get_position_of_first_one(index2))
}

fn combine_all_variables(tree:Vec<Node>, root_index:usize, variables:Vec<VariableTerm>) -> Node{
    let mut tree_iterator = construct_tree_iterator(&tree, root_index);
    let mut result: Vec<Vec<usize>> = Vec::with_capacity(variables.len());
    for _ in 0..variables.len(){
        result.push(Vec::new());
    }

    while let Some(node) = tree_iterator.next(){
        let node = tree[node];
        println!("{}", node.index());
        let term = convert_node_to_variable_term(&tree, root_index);

        if !term.is_some(){continue;}
        let term = term.unwrap();

        let variable_index:usize = variables.iter().position(|thing| *thing == term).unwrap();
        let node_index:usize = node.index();

        result[variable_index].push(node_index);
        tree_iterator.skip_next_branch = true;
    };

    for mut result_set in result{
        while result_set.len() > 1{
            root_node = combine_nodes(root_node, result_set[0], result_set.swap_remove(1))
        }
    }
    root_node
}

// fn construct_variable_coefficient_node(variable:Token, coefficient:f32, exponent:f32, index:usize) -> Node{
//     let constant_child = Box::from(construct_constant(coefficient, index * 2));
//     let new_token = Token{token_type:Operator, token: String::from("*")};
//
//     if exponent != 1f32{
//         let exponent_token = Token{token_type:Operator, token:String::from("^")};
//         let exponent_power = Box::from(construct_constant(exponent, (index * 2 + 1) * 2 + 1));
//         let variable_child = Box::from(Terminal(variable, (index * 2 + 1) *2));
//         let exponent_child = Box::from(Branching(exponent_token, variable_child, exponent_power, index * 2 + 1));
//         return Branching(new_token, constant_child,exponent_child, index);
//     }
//
//     let variable_child = Box::from(Terminal(variable, index * 2 + 1));
//     Branching(new_token, constant_child,variable_child, index)
// }
//
// fn get_parent_of_node(root_node: &Node, child_index:usize) -> &Node {
//     get_node_by_index(root_node, child_index/ 2 )
// }
//
// ///Return the node that will make the parent node an identity
// fn get_identity_node(parent_node: &Node, index:usize) -> Node{
//     if let Branching(token, ..) = parent_node{
//         let required_number = match token.token.as_str(){
//             "+" | "-" => 0f32,
//             "*" | "/" | "^" => 1f32,
//             &_ => unreachable!("Function called on non-operator node"),
//         };
//         return construct_constant(required_number, index)
//     }
//     unreachable!("Never call this function on a terminal node")
// }
//
// fn construct_constant(value:f32, index:usize) -> Node{
//     Terminal(Token{token_type:Constant, token:value.to_string()}, index)
// }
//
type NodeCheck = fn(&Node) -> bool;
///Check if condition1 applies to child1 and condition2 applies to child2 or vice versa
fn check_this_and_that(condition1:NodeCheck, condition2:NodeCheck, node_1:&Node, node_2:&Node) -> bool{
    (condition1(node_1) && condition2(node_2)) || (condition2(node_1) && condition1(node_2))
}

fn push_new_node_to_stack(stack: &mut VecDeque<usize>, node_token: Token, child1:usize, child2:usize, index:usize, mut nodes:Vec<Node>) -> Vec<Node>{
    nodes.push(Branching(node_token, child2, child1, index, 0));
    stack.push_back(nodes.len() - 1);
    nodes
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

fn index_tree<'a>(original_vec:&Vec<Node>, mut tree_vec:Vec<Node>, reference_index:usize, tree_index:usize) -> Vec<Node>{
    let parent_index = reference_index / 2;
    match &original_vec[reference_index]{
        Branching(token, child_1, child_2, _, _) =>{
            tree_vec[reference_index] = Branching(token.clone(), *child_1, *child_2, tree_index, parent_index);
            tree_vec = index_tree(original_vec, tree_vec, *child_1, tree_index * 2);
            tree_vec = index_tree(original_vec, tree_vec, *child_2, tree_index * 2 + 1);
            tree_vec
        }
        Terminal(token, _, _) => {
            tree_vec[reference_index] = Terminal(token.clone(), tree_index, parent_index);
            tree_vec
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

// fn get_simplifiable_nodes(parent_node: &Node, found_nodes: Rc<RefCell<Vec<usize>>>) -> bool {
//     //Check children
//     //If one is a variable, return false
//     //If both are constants, return true
//     if let Branching(_, child_1, child_2, _) = parent_node{
//         let child0valid: bool =
//             match &**child_1{
//                 Terminal(token, _) => {token.token_type == Constant}
//                 Branching(..) => {
//                     get_simplifiable_nodes(&child_1, Rc::clone(&found_nodes))
//                 }
//             };
//
//         let child1valid: bool =
//             match &**child_2{
//                 Terminal(token, _) => {token.token_type == Constant}
//                 Branching(..) => {
//                     get_simplifiable_nodes(&child_2, Rc::clone(&found_nodes))
//                 }
//             };
//
//         if child0valid && !child1valid {
//             found_nodes.borrow_mut().push(child_1.index());
//         }
//
//         if !child0valid && child1valid{
//             found_nodes.borrow_mut().push(child_2.index());
//         }
//
//         return child0valid && child1valid;
//     }
//
//     unreachable!("This function should never be called on a non-branching node");
// }

fn print_tree(node_vec:&Vec<Node>, reference_index:usize, indent: &String, last: bool){
    print!("{}", indent);
    let node = &node_vec[reference_index];
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

    if let Branching(_, child_1, child_2, _, _) = node{
        print_tree(node_vec, *child_1, &local_indent, false);
        print_tree(node_vec, *child_2, &local_indent, true);
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

// fn get_node_by_index(root_node: &Node, index:usize) -> &Node {
//     let thing= (0..usize::BITS).rev().map(|n| (index >> n) & 1);
//     let mut first_one_found: bool = false;
//     let mut current_node: &Node = root_node;
//     for bit in thing{
//         if !first_one_found{
//             first_one_found = bit == 1;
//             continue;
//         };
//
//         if let Branching(_, child_1,child_2, ..) = current_node {
//             current_node = if bit == 1 {&child_2} else {&child_1}
//         }
//         else{
//             panic!("This code should never run")
//         }
//     };
//     current_node
// }

fn evaluate_tree (node_vec:&Vec<Node>, index:usize) -> f32{
    match &node_vec[index]{
        Branching(token, child_1, child_2, ..) =>{
            match token.token.as_str() {
                "+" =>{
                    evaluate_tree(node_vec, *child_1) + evaluate_tree(node_vec, *child_2)
                }
                "-" =>{
                    evaluate_tree(node_vec, *child_1) - evaluate_tree(node_vec, *child_2)
                }
                "*" =>{
                    evaluate_tree(node_vec, *child_1) * evaluate_tree(node_vec, *child_2)
                }
                "/" =>{
                    evaluate_tree(node_vec, *child_1) / evaluate_tree(node_vec, *child_2)
                }
                "^" =>{
                    evaluate_tree(node_vec,*child_1).powf(evaluate_tree(node_vec, *child_2))
                }
                _ =>{
                    panic!("Branching nodes should only contain operators")
                }
            }
        }
        Terminal(token, ..) => {
            extract_variable_or_constant(&token)
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

fn process_operator<'a>(mut operator_stack: VecDeque<Token>, incoming: Token, operands: &mut VecDeque<usize>, mut nodes:Vec<Node>) -> (VecDeque<Token>, Vec<Node>){
    if let Some(last_operator_option) = operator_stack.back(){
        // println!("{x}");
        //Treat a left parenthesis as empty stack
        if get_first_char(&last_operator_option.token).unwrap() == '('{
            operator_stack.push_back(incoming);
            return (operator_stack, nodes)
        }

        match compare_operator_precedence(&get_first_char(&last_operator_option.token).unwrap(), &get_first_char(&incoming.token).unwrap())
        {
            cmp::Ordering::Less =>{
                println!("Operator of lesser precedence found on the stack");
                operator_stack.push_back(incoming);
            }
            cmp::Ordering::Greater =>{
                println!("Operator of greater precedence found on the stack");
                let child1: usize = operands.pop_back().unwrap();
                let child2: usize =  operands.pop_back().unwrap();
                nodes = push_new_node_to_stack(operands, operator_stack.pop_back().unwrap(), child1, child2, 0, nodes);
                (operator_stack, nodes) = process_operator(operator_stack, incoming, operands, nodes);
            }
            cmp::Ordering::Equal =>{
                println!("Operator of equal precedence");
                let child1: usize = operands.pop_back().unwrap();
                let child2: usize =  operands.pop_back().unwrap();
                nodes = push_new_node_to_stack(operands, operator_stack.pop_back().unwrap(), child1, child2, 0, nodes);
                operator_stack.push_back(incoming);
            }
        }
        (operator_stack, nodes)
    }
    else{
        operator_stack.push_back(incoming);
        (operator_stack, nodes)
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

// fn replace_node_at_index<'a>(root_node:Node, new_node:Node, index:usize, path_position:usize) -> Node {
//     if path_position == 0{
//         return new_node
//     }
//     let path_position = path_position - 1;
//     if let Branching(token, child_1, child_2, current_index) = root_node {
//         let bit =index & (1 << path_position) > 0;
//         let (mut keep_child, mut change_child) = (*child_1, *child_2);
//
//         let new_children = if bit == false{
//             (keep_child, change_child) = (change_child, keep_child);
//             (replace_node_at_index(change_child, new_node, index, path_position), keep_child)
//         }
//         else{
//             (keep_child, replace_node_at_index(change_child, new_node, index, path_position))
//         };
//
//         Branching(token,
//                   Box::from(new_children.0), Box::from(new_children.1),
//                   current_index)
//     } else {
//         unreachable!("Reached end of tree before end of path")
//     }
// }

fn construct_tree_iterator(vec:&Vec<Node>, start:usize) -> TreeIterator<'_> {
    TreeIterator{current:None, vector:vec, root_index:start, unexplored:VecDeque::new(), skip_next_branch:false}
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

#[derive(Clone, PartialEq)]
struct Token<>{
    token_type: TokenCategory,
    token: String
}

#[derive(Clone)]
enum Node<> {
    //Self token, child 1, child 2, self index, parent index
    Branching(Token, usize, usize, usize, usize),
    Terminal(Token, usize, usize)
}

impl Node{
    fn get_token(&self) -> &Token{
        match self{
            Branching(token, ..) => token,
            Terminal(token, ..) => token
        }
    }
    fn index(&self) -> usize{
        match self{
            Branching(_,_,_,index, _) => *index,
            Terminal(_, index, _) => *index
        }
    }
    fn is_terminal(&self) -> bool{
        match self{
            Terminal(..) => true,
            Branching(..) => false
        }
    }




}

fn get_variable_coefficient(node_vec:&Vec<Node>, index:usize) -> Option<f32>{
    match &node_vec[index]{
        Terminal(token, ..) => {if token.token_type == Variable {Some(1f32)} else {None}},
        Branching(token, ..) =>{
            //If this is a multiplication node, assume that it is a variable-coefficient node.
            println!("I cannot imagine that this detects exponents correctly");
            if token.token == "*" {return Some(get_immediate_child_of_type(node_vec, index, Constant).unwrap().get_token().token.parse().unwrap())}

            None
        }
    }
}

//Assumes that the two nodes are in the same vector
fn is_branch_equal_to(vec_1:&Vec<Node>, vec_2:Option<&Vec<Node>>, index_1:usize, index_2:usize) -> bool {
    let local_vec_2 = if let Some(thing) = vec_2 {thing} else { vec_1 };
    let other_token = local_vec_2[index_2].get_token();
    if vec_1[index_1].get_token().token_type != other_token.token_type {return false};

    match &vec_1[index_1] {
        Terminal(token, ..) => {
            token.token == other_token.token
        },
        Branching(_, child_1, child_2, ..) => {
            let (other_child_1, other_child_2) = if let Branching(_, one, two, ..) = local_vec_2[index_2]
            { (one, two) } else { unreachable!() };

            let condition_1 = is_branch_equal_to(vec_1, vec_2, *child_1, other_child_1) && is_branch_equal_to(vec_1, vec_2, *child_2, other_child_2);
            let condition_2 = is_branch_equal_to(vec_1, vec_2, *child_1, other_child_2) && is_branch_equal_to(vec_1, vec_2, *child_2, other_child_1);

            condition_1 || condition_2
        }
    }
}

fn get_any_child_of_type(node_vec:&Vec<Node>, index:usize, category:TokenCategory) -> Option<&Node>{
    let node = &node_vec[index];
    if node.get_token().token_type == category {return Some(node)}
    match node {
        Terminal(..) =>{None},
        Branching(_, child1, child2, _, _) => {
            if let Some(thing) = get_any_child_of_type(node_vec, *child1, category.clone()) {Some(thing)}
            else if let Some(thing) = get_any_child_of_type(node_vec, *child2, category) {Some(thing)}
            else {None}
        }
    }
}

fn get_immediate_child_of_type(tree_vector:&Vec<Node>, node_index:usize, category:TokenCategory) -> Option<&Node>{
    match tree_vector[node_index] {
        Terminal(..) => None,
        Branching(_, child_1, child_2,_, _) =>{
            if tree_vector[child_1].get_token().token_type == category {Some(&tree_vector[child_1])}
            else if tree_vector[child_2].get_token().token_type == category {Some(&tree_vector[child_2])}
            else {None}
        }
    }
}
#[derive(Clone)]
struct TreeIterator<'a>{
    current: Option<usize>,
    vector: &'a Vec<Node>,
    root_index: usize,
    unexplored: VecDeque<usize>,
    skip_next_branch:bool
}

impl<'b> Iterator for TreeIterator<'b>{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item>{
        if self.skip_next_branch
        {
            self.current = self.unexplored.pop_back();
            self.skip_next_branch = false;
            return self.current
        };

        if !self.current.is_some(){
            self.current = Some(self.root_index);
            return self.current
        }

        let current_node = &self.vector[self.current.unwrap()];
        self.current =
            match current_node {
                Branching(_, child_1, child_2, ..) => {
                    self.unexplored.push_back(*child_2);
                    Some(*child_1)
                }
                Terminal(..) => {
                    self.unexplored.pop_back()
                }
            };


        self.current
    }
}

struct TreeBranch{
    start: usize,
    ends:Vec<usize>
}

#[derive(PartialEq)]
struct VariableTerm{
    variable:char,
    exponent:f32
}