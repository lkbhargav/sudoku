use sudoko::game::game::Game;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// Website to validate for problem uniqueness: https://www.thonky.com/sudoku/solution-count
fn main() {
    let mut g = Game::new();
    g.start_game();
}
