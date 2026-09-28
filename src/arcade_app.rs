use itertools::Itertools;
use std::cmp::PartialEq;
use ratatui::{DefaultTerminal, Frame, Terminal};
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Margin};
use ratatui::prelude::Direction;
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::symbols::merge::MergeStrategy::Exact;
use ratatui::widgets::{Block, Borders, Paragraph, Table, Widget};
use crate::arcade_app::State::Started;

#[derive(PartialEq)]
pub enum State {
    Started,
    Playing,
    Won,
    Lost,
    Error
}

#[derive(Clone, PartialEq, Eq)]
struct Difficulty {
    pub tries : u8,
    pub range: u8,
    pub spots : u8
}

const EASY : Difficulty = Difficulty {tries : 12, range: 5, spots : 4 };
const MEDIUM : Difficulty = Difficulty {tries : 12, range: 7, spots : 4 };
const HARD : Difficulty = Difficulty {tries : 12, range: 7, spots : 5 };

enum LevelState {
    Guess,
    Notes,
    End
}

pub(crate) struct App {
    state : State,
    stopped : bool,
    level: Level,
}

struct Level {
    difficulty : Difficulty,
    level_state : LevelState,
    guess : Vec<u8>,
    solution : Vec<u8>,
    hint : Vec<Hint>,
}

struct Hint {
    old_guess : Vec<u8>,
    correctness : Vec<char>
}

impl Hint {
    fn new(old_guess : Vec<u8>, correctness : Vec<char>) -> Hint {
        Hint {old_guess, correctness}
    }
}

impl App {
    //hier könnte man statt default auch eine App spezifischer machen
    pub(crate) fn default() -> App {
        Self::new()
    }

    fn new() -> App {
        Self {
            state: State::Started,
            stopped: false,
            level: Level {
                difficulty : EASY,
                level_state : LevelState::Guess,
                guess : Vec::new(),
                solution : Vec::new(),
                hint : Vec::new(),
            }
        }
    }

    pub(crate) fn run(&mut self, terminal : &mut DefaultTerminal) {
        terminal.draw(|f| { self.render_level_selection(f) });
        while !self.stopped {
            if let Ok(event) = event::read() {
                match event {
                    Event::Resize(_width , _height) => {
                        match self.state {
                            State::Started => {
                                terminal.draw(|f| { self.render_level_selection(f) });
                            }
                            State::Playing => {
                                terminal.draw(|f| { self.render_game(f) });
                            }
                            _ => {}
                        }
                    }
                    Event::Key(key) => {
                        match (key.code, &self.state, &self.level.level_state) {
                            (KeyCode::Char('q'), _, _) => {
                                return;
                            },
                            (kc, State::Started, _) if kc == KeyCode::Left || kc == KeyCode::Right => {
                                match (self.level.difficulty.clone(), kc) {
                                    (EASY, KeyCode::Right) => {
                                        self.level.difficulty = MEDIUM;
                                    }
                                    (MEDIUM, KeyCode::Right) => {
                                        self.level.difficulty = HARD;
                                    }
                                    (MEDIUM, KeyCode::Left) => {
                                        self.level.difficulty = EASY;
                                    }
                                    (HARD, KeyCode::Left) => {
                                        self.level.difficulty = MEDIUM;
                                    }
                                    _ => {}
                                }
                                terminal.draw(|f| { self.render_level_selection(f) });
                            }
                            (KeyCode::Enter, State::Started, _) => {
                                terminal.draw(|f| { self.render_game(f) });
                                self.state = State::Playing;
                                self.initialize_game();
                            },
                            (KeyCode::Backspace, State::Playing, _) => {
                                self.level.guess.pop();
                                terminal.draw(|f| { self.render_game(f) });
                            }
                            (KeyCode::Char(c) , State::Playing, LevelState::Guess) => {
                                if self.level.guess.len() < self.level.difficulty.spots as usize && (c.is_digit(10) && c.to_digit(10).unwrap() <= self.level.difficulty.range as u32) {
                                    self.level.guess.push(c.to_digit(10).unwrap() as u8);
                                    terminal.draw(|f| { self.render_game(f) });
                                }
                            },
                            (KeyCode::Enter, State::Playing, LevelState::Guess) => {
                                if self.level.guess.len() == self.level.difficulty.spots as usize {
                                    self.check_if_found();
                                    self.level.guess.clear();
                                    terminal.draw(|f| { self.render_game(f) });
                                }
                            },
                            _ => {

                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn render_level_selection(&self, frame: &mut Frame) {
        let base_layout = Layout::default().direction(Direction::Vertical).constraints(vec![Constraint::Fill(1), Constraint::Percentage(10)]).split(frame.area());
        let level_selection_layout = Layout::default()
            .direction(Direction::Horizontal).constraints(vec![Constraint::Percentage(35), Constraint::Percentage(10), Constraint::Percentage(10), Constraint::Percentage(10), Constraint::Percentage(35)])
            .split(base_layout[1]);

        frame.render_widget(Block::bordered().title(" Level auswählen "), base_layout[0]);

        let mut easy_widget = Paragraph::new("Einfach")
            .block(Block::bordered())
            .alignment(Alignment::Center);
        let mut medium_widget = Paragraph::new("Schwer")
            .block(Block::bordered())
            .alignment(Alignment::Center);
        let mut hard_widget = Paragraph::new("Experte")
            .block(Block::bordered())
            .alignment(Alignment::Center);

        match self.level.difficulty {
            MEDIUM => {
                medium_widget = medium_widget.bg(Color::White).fg(Color::Black);
            },
            HARD => {
                hard_widget = hard_widget.bg(Color::White).fg(Color::Black);
            },
            _ => {
                easy_widget = easy_widget.bg(Color::White).fg(Color::Black);
            },
        }

        frame.render_widget(easy_widget, level_selection_layout[1]);
        frame.render_widget(medium_widget, level_selection_layout[2]);
        frame.render_widget(hard_widget, level_selection_layout[3]);
    }

    pub fn render_game(&self, frame: &mut Frame) {
        let base_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(70), Constraint::Fill(1)])
        .split(frame.area());

        let input_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Min(0)])
        .split(base_layout[0]);

        let input_guess_layout = Layout::default().margin(10).constraints(vec![Constraint::Fill(1)]).split(input_layout[0]);
        let guess_hint_layout = Layout::default().margin(10).constraints(vec![Constraint::Min(0)]).split(base_layout[1]);

        let guess = self.level.guess.clone();
        let text = guess.iter().join(" ");

        frame.render_widget(Block::bordered().merge_borders(Exact).title(" Deine Versuche "), base_layout[1]);
        frame.render_widget(Block::bordered().merge_borders(Exact).title(" Neuen Versuch eingeben "), input_layout[0]);
        frame.render_widget(Block::bordered().merge_borders(Exact).title(" Notizen "), input_layout[1]);
        frame.render_widget(Paragraph::new(text), input_guess_layout[0]);

        let mut results = String::new();
        for hint in self.level.hint.iter().clone() {
            results = results + &*hint.old_guess.iter().join(" ");
            results += "\n";
            results = results + &*hint.correctness.iter().join(" ");
            results += "\n";
        }

        frame.render_widget(Paragraph::new(results), guess_hint_layout[0]);
    }

    fn initialize_game(&mut self) {
        self.generate_level();
    }

    fn generate_level(&mut self) {
        let range = self.level.difficulty.range;
        let spots = self.level.difficulty.spots;

        let mut combination:Vec<u8> = Vec::new();
        for _i in 0..spots {
            let number = rand::random_range(0..(range - 1));
            combination.push(number);
        }
        print!("{:?}", combination);
        self.level.solution = combination;
    }

    fn check_if_found(&mut self) -> bool {

        let mut hint: Vec<char> = Vec::new();
        let mut guess_helper = self.level.guess.clone();
        let mut guess = self.level.guess.clone();
        let mut solution = self.level.solution.clone();
        let mut found = true;

        for i in (0..guess_helper.len()).rev() {
            if guess_helper.get(i).unwrap() == solution.get(i).unwrap() {
                hint.push('\u{2713}'); //checkmark
                guess.remove(i);
                solution.remove(i);
            }
        }

        guess_helper = guess.clone();
        for i in (0..guess_helper.len()).rev() {
            if solution.contains(guess_helper.get(i).unwrap()) {
                hint.push('o');
                guess.remove(i);
                for c in 0..solution.len() {
                    if solution.get(c).unwrap() == guess_helper.get(i).unwrap() {
                        solution.remove(c);
                        break;
                    }
                }
                found = false;
            }
        }

        guess_helper = guess.clone();
        for _i in (0..guess_helper.len()).rev() {
            hint.push('x');
            found = false;
        }

        self.level.hint.push(Hint::new(self.level.guess.clone(), hint));

        found
    }
}

