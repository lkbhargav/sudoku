use sudoko::{game::game::Game, sudoku::Sudoku};

// Website to validate for problem uniqueness: https://www.thonky.com/sudoku/solution-count
fn main() {
    let mut g = Game::new();
    g.start_game();

    // let board = Sudoku::generate_random_board(40);

    // println!("{board}");

    // println!("{:?}", board.fetch_next_empty_cell());
}

fn test() {
    // tough
    // let str_val = ",,,1,,2,,,,,6,,,,,,7,,,,8,,,,9,,,4,,,,,,,,3,,5,,,,7,,,,2,,,,8,,,,1,,,9,,,,8,,5,,7,,,,,,6,,,,,3,,4,,,";

    // easy
    // let str_val = "4,,9,,7,2,,1,3,7,,2,8,3,,6,,,,1,6,,4,9,8,7,,2,,,1,,,,6,,5,4,7,,,,2,,,6,9,,,,4,,3,5,8,,3,4,,,,,6,,,,,,3,1,,,,6,,9,,,,4,";

    // easy 2
    // let str_val = ",8,7,,5,,,,,4,9,,,3,6,1,,,5,1,,9,8,2,,,4,,,,,,5,4,,6,7,,,,6,9,,1,,1,,,,4,,7,5,,2,,,8,1,3,6,,9,9,4,,,,7,,3,,,,,,,4,8,,7";

    // let str_val =
    //     "8,2,,,,,,,,,,4,,,,,,,,,1,,,,,,,,,,,,,9,,,,,,,,,,,,6,,,,,,,,,3,,,,,,7,,,,,,,,,,,,,,,,5,,,,";

    // let mut sdk = Sudoku::from_str(str_val).expect("expected a valid sudoku puzzle");

    // println!("{sdk}");

    // println!("{}", sdk.to_str());

    // sdk.solve(None);

    // println!("{sdk}");

    // println!("Is puzzle valid: {}", sdk.is_puzzle_valid());
}
