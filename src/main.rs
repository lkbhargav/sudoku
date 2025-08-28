use sudoko::game::game::Game;

// Website to validate for problem uniqueness: https://www.thonky.com/sudoku/solution-count
fn main() {
    let mut g = Game::new();
    g.start_game();
}
