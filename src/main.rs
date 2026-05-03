use std::io;
use std::mem;
use crate::TokenCategory::{Constant, Operator, Parenthesis, Variable};

fn main() {
    let mut input = " z*(x+y)";
    let mut tokens: Vec<Token> = Vec::with_capacity(10);
    let mut current_token: Token;

    while let Some(category) = get_token_category(input){
        println!("Remaining untokenized data: {input}");
        (current_token , input) = build_token_of_type(category, &input);

        let token: &String = &current_token.token;
        println!("{}", *token);

        tokens.push(current_token);

    }
}

fn get_token_category_of_character (c: &char) -> Option<TokenCategory> {
    match c{
        '0'..='9' | '.' => Some(Constant),
        'a'..='z' => Some(Variable),
        ' ' => None,
        '+' | '-' | '*' | '/' => Some(Operator),
        '(' | ')' => Some(Parenthesis),
        _ => {
            println!("Unexpected character: {c}");
            None
        }
    }

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

enum TokenCategory {
    Constant,
    Variable,
    Operator,
    Parenthesis
}

struct Token{
    token_type: TokenCategory,
    token: String
}
