extern crate clipboard;
extern crate termion;

use termion::event::*;
use termion::input::{MouseTerminal, TermRead};
use termion::raw::IntoRawMode;
use termion::{color, style};

use rayon::prelude::*;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use clipboard::{ClipboardContext, ClipboardProvider};

use std::io::{stdin, stdout, Read, Write};

enum KeyCaptureState {
    Gameplay,
    EditBoard,
    ChooseColour,
    PromotePawn,
    ExitGame,
}

#[derive(Clone, PartialEq)]
enum Piece {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
    Empty,
}

#[derive(Clone, PartialEq, EnumIter)]
enum Move {
    U,
    D,
    R,
    L,
    UL,
    DL,
    UR,
    DR,
    RRU,
    RUU,
    RRD,
    RDD,
    LUU,
    LLU,
    LLD,
    LDD,
}

#[derive(Clone)]
struct Square {
    icon: char,
    color: usize,
    piece: Piece,
    moves: Vec<Move>,
    is_valid_move: bool,
}

struct Game<R, W> {
    board: Vec<Vec<Square>>,
    x: usize,
    y: usize,
    cursor_x: u16,
    cursor_y: u16,
    turn: usize,
    king_in_check: bool,
    selected_piece: [usize; 2],
    en_passant: Vec<[usize; 2]>,
    castling_rights: [[bool; 2]; 2],
    king_coords: [[usize; 2]; 2],
    moves: Vec<[usize; 2]>,
    show_fen: bool,
    halfmove_clock: usize,
    fullmoves: usize,
    stdout: W,
    stdin: R,
}

impl Square {
    pub fn new(piece: Piece, color: usize) -> Self {
        Self {
            icon: match piece {
                Piece::King => {
                    if color == 0 {
                        '♔'
                    } else {
                        '♚'
                    }
                }
                Piece::Queen => {
                    if color == 0 {
                        '♕'
                    } else {
                        '♛'
                    }
                }
                Piece::Rook => {
                    if color == 0 {
                        '♖'
                    } else {
                        '♜'
                    }
                }
                Piece::Bishop => {
                    if color == 0 {
                        '♗'
                    } else {
                        '♝'
                    }
                }
                Piece::Knight => {
                    if color == 0 {
                        '♘'
                    } else {
                        '♞'
                    }
                }
                Piece::Pawn => {
                    if color == 0 {
                        '\u{2659}'
                    } else {
                        '\u{265F}'
                    }
                }
                _ => ' ',
            },
            moves: match piece {
                Piece::King => vec![
                    Move::U,
                    Move::D,
                    Move::L,
                    Move::R,
                    Move::UR,
                    Move::UL,
                    Move::DR,
                    Move::DL,
                ],
                Piece::Queen => vec![
                    Move::U,
                    Move::D,
                    Move::L,
                    Move::R,
                    Move::UR,
                    Move::UL,
                    Move::DR,
                    Move::DL,
                ],
                Piece::Rook => vec![Move::U, Move::D, Move::L, Move::R],
                Piece::Bishop => vec![Move::UR, Move::UL, Move::DR, Move::DL],
                Piece::Knight => vec![
                    Move::RRU,
                    Move::RUU,
                    Move::RRD,
                    Move::RDD,
                    Move::LLU,
                    Move::LUU,
                    Move::LLD,
                    Move::LDD,
                ],
                Piece::Pawn => {
                    if color == 0 {
                        vec![Move::U, Move::UL, Move::UR]
                    } else {
                        vec![Move::D, Move::DL, Move::DR]
                    }
                }
                _ => vec![],
            },
            piece,
            color,
            is_valid_move: false,
        }
    }
}

fn init_game<R: Read, W: Write>(stdout: W, stdin: R) {
    let mut game = Game {
        board: Vec::new(),
        x: 0,
        y: 0,
        cursor_x: 2,
        cursor_y: 1,
        turn: 0,
        king_in_check: false,
        selected_piece: [0, 0],
        castling_rights: [[true, true], [true, true]],
        king_coords: [[4, 7], [4, 0]],
        en_passant: vec![],
        moves: Vec::new(),
        show_fen: false,
        halfmove_clock: 0,
        fullmoves: 1,
        stdout,
        stdin: stdin.events(),
    };

    game.start();
}

fn get_change_from_move(m: &Move) -> [isize; 2] {
    match m {
        Move::U => [0, -1],
        Move::D => [0, 1],
        Move::R => [1, 0],
        Move::L => [-1, 0],
        Move::UR => [1, -1],
        Move::UL => [-1, -1],
        Move::DR => [1, 1],
        Move::DL => [-1, 1],
        Move::RRU => [2, -1],
        Move::RUU => [1, -2],
        Move::RRD => [2, 1],
        Move::RDD => [1, 2],
        Move::LLU => [-2, -1],
        Move::LUU => [-1, -2],
        Move::LLD => [-2, 1],
        Move::LDD => [-1, 2],
    }
}

impl<R: Iterator<Item = Result<Event, std::io::Error>>, W: Write> Game<R, W> {
    fn init_board(&mut self) {
        // First row
        let mut row: Vec<Square> = vec![
            Square::new(Piece::Rook, 1),
            Square::new(Piece::Knight, 1),
            Square::new(Piece::Bishop, 1),
            Square::new(Piece::Queen, 1),
            Square::new(Piece::King, 1),
            Square::new(Piece::Bishop, 1),
            Square::new(Piece::Knight, 1),
            Square::new(Piece::Rook, 1),
        ];
        self.board.push(row);

        row = Vec::new();
        for _ in 0..8 {
            row.push(Square::new(Piece::Pawn, 1));
        }
        self.board.push(row);

        for _ in 0..4 {
            row = Vec::new();
            for _ in 0..8 {
                row.push(Square::new(Piece::Empty, 2));
            }
            self.board.push(row);
        }

        row = Vec::new();
        for _ in 0..8 {
            row.push(Square::new(Piece::Pawn, 0));
        }
        self.board.push(row);

        row = vec![
            Square::new(Piece::Rook, 0),
            Square::new(Piece::Knight, 0),
            Square::new(Piece::Bishop, 0),
            Square::new(Piece::Queen, 0),
            Square::new(Piece::King, 0),
            Square::new(Piece::Bishop, 0),
            Square::new(Piece::Knight, 0),
            Square::new(Piece::Rook, 0),
        ];
        self.board.push(row);
    }

    fn get_bg_color(&self, x: u16, y: u16) -> String {
        let white = color::Bg(color::Rgb(200, 200, 200)).to_string();
        let black = color::Bg(color::LightGreen).to_string();

        if x % 2 == 0 {
            if y % 2 == 0 {
                white
            } else {
                black
            }
        } else if y % 2 == 0 {
            black
        } else {
            white
        }
    }

    fn print_initial_board(&mut self) {
        write!(
            self.stdout,
            "{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();

        for y in 0..8 {
            write!(
                self.stdout,
                "{}{}{}",
                termion::cursor::Goto(1, y + 1),
                color::Bg(color::Blue),
                8 - y
            )
            .unwrap();

            for x in 0..8 {
                write!(
                    self.stdout,
                    "{}{}",
                    self.get_bg_color(x, y),
                    self.board[y as usize][x as usize].icon
                )
                .unwrap();
            }
        }

        write!(
            self.stdout,
            "{}{} ",
            color::Bg(color::Blue),
            termion::cursor::Goto(1, 9)
        )
        .unwrap();

        for i in 0..8 {
            write!(self.stdout, "{}", char::from_u32(i as u32 + 65).unwrap()).unwrap();
        }

        write!(self.stdout, "{}", style::Reset).unwrap();
        self.stdout.flush().unwrap();
    }

    //FEN helper functions
    fn create_fen_string(&mut self) -> String {
        let mut fen: String = "".to_string();
        let mut empty_count: usize = 0;
        let mut piece_char: char;
        for y in 0..8 {
            for x in 0..8 {
                match self.board[y][x].piece {
                    Piece::King => piece_char = 'k',
                    Piece::Queen => piece_char = 'q',
                    Piece::Rook => piece_char = 'r',
                    Piece::Bishop => piece_char = 'b',
                    Piece::Knight => piece_char = 'n',
                    Piece::Pawn => piece_char = 'p',
                    Piece::Empty => {
                        piece_char = ' ';
                        empty_count += 1;
                    }
                }

                if piece_char != ' ' {
                    if empty_count > 0 {
                        fen += &empty_count.to_string();
                        empty_count = 0;
                    }
                    if self.board[y][x].color == 0 {
                        piece_char = piece_char.to_uppercase().next().unwrap();
                    }
                    fen += &piece_char.to_string();
                }
            }
            if empty_count > 0 {
                fen += &empty_count.to_string();
                empty_count = 0;
            }

            if y < 7 {
                fen += "/";
            }
        }

        if self.turn == 0 {
            fen += " w ";
        } else {
            fen += " b ";
        }

        let mut castle_string = "".to_string();
        if self.castling_rights[0][0] {
            castle_string += "K";
        }

        if self.castling_rights[0][1] {
            castle_string += "Q";
        }

        if self.castling_rights[1][0] {
            castle_string += "k";
        }

        if self.castling_rights[1][1] {
            castle_string += "q";
        }

        if castle_string.is_empty() {
            castle_string += "-";
        }

        fen += &castle_string;

        if self.en_passant.is_empty() {
            fen += " -";
        } else {
            fen += &format!(
                " {}{}",
                &char::from_u32(self.en_passant[0][0] as u32 + 97)
                    .unwrap()
                    .to_string(),
                &self.en_passant[0][1].to_string(),
            );
        }

        fen += &format!(
            " {} {}",
            &self.halfmove_clock.to_string(),
            &self.fullmoves.to_string()
        );

        fen
    }

    fn display_fen_string(&mut self) {
        let fen = self.create_fen_string();
        write!(
            self.stdout,
            "{}{}{}{}{}",
            termion::cursor::Goto(1, 12),
            termion::clear::AfterCursor,
            color::Bg(color::Green),
            fen,
            style::Reset
        )
        .unwrap();
        self.reset_cursor();
    }

    fn copy_fen_to_clipboard(&mut self) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        ctx.set_contents(self.create_fen_string()).unwrap();
        write!(
            self.stdout,
            "{}Copied FEN string to clipboard!",
            termion::cursor::Goto(1, 13)
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }

    fn fill_board_from_fen_string(&mut self, fen: String) {
        let contents: Vec<&str> = fen.split_whitespace().collect();
        let pieces = contents[0];
        let color = contents[1];
        let castling_rights = contents[2];
        let en_passant = contents[3];
        let halfmove_clock = contents[4];
        let fullmoves = contents[5];

        let lines: Vec<&str> = pieces.split('/').collect();
        for (y, line) in lines.into_iter().enumerate() {
            let mut x = 0;
            for c in line.chars() {
                let piece: Piece = if c.is_ascii_digit() {
                    Piece::Empty
                } else {
                    {
                        match c.to_ascii_lowercase() {
                            'k' => Piece::King,
                            'q' => Piece::Queen,
                            'r' => Piece::Rook,
                            'b' => Piece::Bishop,
                            'n' => Piece::Knight,
                            'p' => Piece::Pawn,
                            _ => Piece::Empty,
                        }
                    }
                };

                if piece == Piece::Empty {
                    let i: usize = c.to_digit(10).unwrap() as usize;
                    for _ in 0..i {
                        self.place_piece(Piece::Empty, 2, x, y);
                        x += 1;
                    }
                } else {
                    let color: usize = if c.is_ascii_uppercase() { 0 } else { 1 };

                    if piece == Piece::King {
                        self.king_coords[color] = [x, y];
                    }
                    self.place_piece(piece, color, x, y);
                    x += 1;
                }
            }
        }

        if color.starts_with('w') {
            self.turn = 0;
        } else {
            self.turn = 1;
        }

        self.castling_rights = [[false, false], [false, false]];
        for c in castling_rights.chars() {
            match c {
                'K' => self.castling_rights[0][0] = true,
                'Q' => self.castling_rights[0][1] = true,
                'k' => self.castling_rights[1][0] = true,
                'q' => self.castling_rights[1][1] = true,
                '-' => break,
                _ => (),
            }
        }

        let en_p_chars: Vec<char> = en_passant.chars().collect();
        if en_p_chars[0] != '-' {
            let x = en_p_chars[0].to_ascii_uppercase() as usize - 65;
            let y = en_p_chars[1].to_digit(10).unwrap() as usize;
            self.en_passant.clear();
            self.en_passant.push([x, y]);
        }

        self.halfmove_clock = halfmove_clock.parse::<usize>().unwrap();
        self.fullmoves = fullmoves.parse::<usize>().unwrap();
    }

    // Valid move finder helper functions
    fn check_for_pin(&mut self) -> Option<[Move; 2]> {
        let x = self.selected_piece[0] as isize;
        let y = self.selected_piece[1] as isize;
        let king_x = self.king_coords[self.turn][0] as isize;
        let king_y = self.king_coords[self.turn][1] as isize;
        let moves;

        if x == king_x {
            if y > king_y {
                moves = [Move::U, Move::D];
            } else {
                moves = [Move::D, Move::U];
            }
        } else if y == king_y {
            if x > king_x {
                moves = [Move::L, Move::R];
            } else {
                moves = [Move::R, Move::L];
            }
        } else if (y - king_y).abs() == (x - king_x).abs() {
            if y > king_y && x > king_x {
                moves = [Move::UL, Move::DR];
            } else if y > king_y && x < king_x {
                moves = [Move::UR, Move::DL];
            } else if y < king_y && x > king_x {
                moves = [Move::DL, Move::UR];
            } else {
                moves = [Move::DR, Move::UL];
            }
        } else {
            return None;
        }

        let mut change = get_change_from_move(&moves[0]);
        let mut tmp_x: isize = x;
        let mut tmp_y: isize = y;
        loop {
            tmp_x += change[0];
            tmp_y += change[1];

            if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                return None;
            }

            match self.board[tmp_y as usize][tmp_x as usize].piece {
                Piece::King => {
                    if self.board[tmp_y as usize][tmp_x as usize].color != self.turn {
                        return None;
                    } else {
                        break;
                    }
                }
                Piece::Empty => continue,
                _ => return None,
            }
        }

        change = get_change_from_move(&moves[1]);
        tmp_x = x;
        tmp_y = y;
        loop {
            tmp_x += change[0];
            tmp_y += change[1];

            if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                return None;
            }
            if self.board[tmp_y as usize][tmp_x as usize].color == self.turn {
                return None;
            }

            match self.board[tmp_y as usize][tmp_x as usize].piece {
                Piece::Queen | Piece::Bishop | Piece::Rook => {
                    if self.board[tmp_y as usize][tmp_x as usize]
                        .moves
                        .contains(&moves[0])
                    {
                        return Some(moves);
                    } else {
                        return None;
                    }
                }
                _ => (),
            }
        }
    }

    fn is_attacked(&mut self, x: isize, y: isize) -> bool {
        let dirs: Vec<Move> = Move::iter().collect();
        let attacked: Vec<bool> = dirs
            .into_par_iter()
            .filter_map(|dir| {
                let change = get_change_from_move(&dir);
                let mut tmp_x: isize = x;
                let mut tmp_y: isize = y;

                for i in 0..7 {
                    tmp_x += change[0];
                    tmp_y += change[1];
                    if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                        return None;
                    }

                    if self.board[tmp_y as usize][tmp_x as usize].color == self.turn {
                        return None;
                    }

                    match self.board[tmp_y as usize][tmp_x as usize].piece {
                        Piece::Empty => (),
                        Piece::King | Piece::Knight => {
                            if i > 0 {
                                return None;
                            }

                            if self.board[tmp_y as usize][tmp_x as usize]
                                .moves
                                .contains(&dir)
                            {
                                return Some(true);
                            } else {
                                return None;
                            }
                        }
                        Piece::Pawn => {
                            if i > 0 {
                                return None;
                            }

                            if self.turn == 0 {
                                if dir == Move::UL || dir == Move::UR {
                                    return Some(true);
                                }
                                return None;
                            } else {
                                if dir == Move::DL || dir == Move::DR {
                                    return Some(true);
                                }
                                return None;
                            }
                        }
                        _ => {
                            if self.board[tmp_y as usize][tmp_x as usize]
                                .moves
                                .contains(&dir)
                            {
                                return Some(true);
                            } else {
                                return None;
                            }
                        }
                    }
                }

                None
            })
            .collect();

        !attacked.is_empty()
    }

    fn filter_legal_moves(&mut self) {
        let current_square = self.board[self.y][self.x].clone();
        self.empty_square(self.x, self.y);
        self.moves = self
            .moves
            .clone()
            .into_iter()
            .filter(|coords| {
                if current_square.piece == Piece::King {
                    self.king_coords[self.turn] = *coords;
                }
                let replaced_piece = self.board[coords[1]][coords[0]].clone();
                self.board[coords[1]][coords[0]] = current_square.clone();
                let check = self.is_attacked(
                    self.king_coords[self.turn][0] as isize,
                    self.king_coords[self.turn][1] as isize,
                );

                self.board[coords[1]][coords[0]] = replaced_piece;
                if current_square.piece == Piece::King {
                    self.king_coords[self.turn] = [self.x, self.y];
                }

                !check
            })
            .collect();
        self.board[self.y][self.x] = current_square;
    }

    fn find_moves(&mut self) {
        self.moves.clear();
        match self.board[self.y][self.x].piece {
            Piece::Pawn => {
                let valid_moves: Vec<Move> = match self.check_for_pin() {
                    Some(pin_moves) => pin_moves
                        .into_iter()
                        .filter(|m| self.board[self.y][self.x].moves.contains(m))
                        .collect(),
                    None => self.board[self.y][self.x].moves.clone(),
                };

                self.moves = valid_moves
                    .into_par_iter()
                    .flat_map(|m| {
                        let change = get_change_from_move(&m);
                        let mut tmp_x: isize = self.x as isize;
                        let mut tmp_y: isize = self.y as isize;
                        let mut moves: Vec<[usize; 2]> = Vec::new();
                        tmp_x += change[0];
                        tmp_y += change[1];

                        if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                            return moves;
                        }

                        if self.board[tmp_y as usize][tmp_x as usize].color == self.turn {
                            return moves;
                        }

                        match m {
                            Move::U | Move::D => {
                                if self.board[tmp_y as usize][tmp_x as usize].piece == Piece::Empty
                                {
                                    moves.push([tmp_x as usize, tmp_y as usize]);
                                }

                                if (self.turn == 0 && self.y == 6)
                                    || (self.turn == 1 && self.y == 1)
                                {
                                    tmp_y += change[1];
                                } else {
                                    return moves;
                                }

                                if self.board[tmp_y as usize][tmp_x as usize].piece == Piece::Empty
                                {
                                    moves.push([tmp_x as usize, tmp_y as usize]);
                                }
                            }
                            _ => {
                                if self.board[tmp_y as usize][tmp_x as usize].piece != Piece::Empty
                                {
                                    moves.push([tmp_x as usize, tmp_y as usize]);
                                    return moves;
                                }

                                if !self.en_passant.is_empty()
                                    && self.en_passant[0][0] == tmp_x as usize
                                    && self.en_passant[0][1] == tmp_y as usize
                                {
                                    moves.push([tmp_x as usize, tmp_y as usize]);
                                }
                            }
                        }
                        moves
                    })
                    .collect();

                if self.king_in_check {
                    self.filter_legal_moves();
                }
            }
            Piece::King => {
                self.moves = self.board[self.y][self.x]
                    .moves
                    .par_iter()
                    .filter_map(|m| {
                        let change = get_change_from_move(m);
                        let mut tmp_x: isize = self.x as isize;
                        let mut tmp_y: isize = self.y as isize;
                        tmp_x += change[0];
                        tmp_y += change[1];

                        if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                            return None;
                        }

                        if self.board[tmp_y as usize][tmp_x as usize].color == self.turn {
                            return None;
                        }

                        Some([tmp_x as usize, tmp_y as usize])
                    })
                    .collect();
                self.filter_legal_moves();
                if self.castling_rights[self.turn][0] {
                    let mut castle = true;
                    let mut tmp_x = self.x;
                    for _ in 0..2 {
                        tmp_x += 1;
                        if self.board[self.y][tmp_x].piece != Piece::Empty {
                            castle = false;
                            break;
                        }

                        if self.is_attacked(tmp_x as isize, self.y as isize) {
                            castle = false;
                            break;
                        }
                    }

                    if castle {
                        self.moves.push([self.x + 2, self.y]);
                    }
                }

                if self.castling_rights[self.turn][1] {
                    let mut castle = true;
                    let mut tmp_x = self.x;

                    for _ in 0..3 {
                        tmp_x -= 1;
                        if self.board[self.y][tmp_x].piece != Piece::Empty {
                            castle = false;
                            break;
                        }

                        if self.is_attacked(tmp_x as isize, self.y as isize) {
                            castle = false;
                            break;
                        }
                    }

                    if castle {
                        self.moves.push([self.x - 2, self.y]);
                    }
                }
            }
            Piece::Knight => {
                if !matches!(self.check_for_pin(), None) {
                    return;
                }

                self.moves = self.board[self.y][self.x]
                    .moves
                    .par_iter()
                    .filter_map(|m| {
                        let change = get_change_from_move(m);
                        let mut tmp_x: isize = self.x as isize;
                        let mut tmp_y: isize = self.y as isize;
                        tmp_x += change[0];
                        tmp_y += change[1];

                        if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                            return None;
                        }

                        if self.board[tmp_y as usize][tmp_x as usize].color == self.turn {
                            return None;
                        }

                        Some([tmp_x as usize, tmp_y as usize])
                    })
                    .collect();
                if self.king_in_check {
                    self.filter_legal_moves();
                }
            }
            _ => {
                let valid_moves: Vec<Move> = match self.check_for_pin() {
                    Some(pin_moves) => pin_moves
                        .into_iter()
                        .filter(|m| self.board[self.y][self.x].moves.contains(m))
                        .collect(),
                    None => self.board[self.y][self.x].moves.clone(),
                };
                self.moves = valid_moves
                    .into_par_iter()
                    .flat_map(|m| {
                        let change = get_change_from_move(&m);
                        let mut tmp_x: isize = self.x as isize;
                        let mut tmp_y: isize = self.y as isize;
                        let mut moves: Vec<[usize; 2]> = Vec::new();
                        for _ in 0..7 {
                            tmp_x += change[0];
                            tmp_y += change[1];

                            if !(0..=7).contains(&tmp_x) || !(0..=7).contains(&tmp_y) {
                                break;
                            }

                            if self.board[tmp_y as usize][tmp_x as usize].color == self.turn {
                                break;
                            }

                            match self.board[tmp_y as usize][tmp_x as usize].piece {
                                Piece::Empty => moves.push([tmp_x as usize, tmp_y as usize]),
                                _ => {
                                    moves.push([tmp_x as usize, tmp_y as usize]);
                                    break;
                                }
                            }
                        }

                        moves
                    })
                    .collect();
                if self.king_in_check {
                    self.filter_legal_moves();
                }
            }
        }
    }

    // Gameplay helper functions
    fn update_en_passant_capture(&mut self) {
        if self.en_passant.is_empty() {
            return;
        }

        let sel_x = self.selected_piece[0];
        let sel_y = self.selected_piece[1];

        if self.board[sel_y][sel_x].piece != Piece::Pawn {
            return;
        }

        if self.x != self.en_passant[0][0] || self.y != self.en_passant[0][1] {
            return;
        }

        if self.turn == 0 {
            self.place_piece(Piece::Empty, 2, self.x, self.y + 1);
        } else {
            self.place_piece(Piece::Empty, 2, self.x, self.y - 1);
        }
    }

    fn castle_king(&mut self) {
        let sel_x = self.selected_piece[0];
        let sel_y = self.selected_piece[1];

        if self.board[sel_y][sel_x].piece != Piece::King {
            return;
        }

        if (sel_x as isize - self.x as isize).abs() != 2 {
            return;
        }

        if self.x > sel_x {
            self.place_piece(Piece::Rook, self.turn, self.x - 1, self.y);
            self.place_piece(Piece::Empty, 2, 7, self.y);
        } else {
            self.place_piece(Piece::Rook, self.turn, self.x + 1, self.y);
            self.place_piece(Piece::Empty, 2, 0, self.y);
        }
    }

    fn should_promote_pawn(&mut self) -> bool {
        if self.board[self.y][self.x].piece != Piece::Pawn {
            return false;
        }

        if self.turn == 0 {
            self.y == 0
        } else {
            self.y == 7
        }
    }

    fn move_selected_piece(&mut self) {
        let to_move_x = self.selected_piece[0];
        let to_move_y = self.selected_piece[1];
        self.board[self.y][self.x] = self.board[to_move_y][to_move_x].clone();
        self.empty_square(to_move_x, to_move_y);
        self.update_square(to_move_x, to_move_y);
        self.update_square(self.x, self.y);
    }

    //Terminal output helper functions
    fn handle_click_or_enter(&mut self, state: &mut KeyCaptureState) {
        if self.board[self.y][self.x].is_valid_move {
            self.update_halfmove_clock();
            self.update_en_passant_capture();
            self.update_en_passant_field();
            self.update_castling_rights();
            self.castle_king();
            self.update_king_coords();
            self.move_selected_piece();
            self.unhighlight_moves();
            self.reset_cursor();
            if self.should_promote_pawn() {
                *state = KeyCaptureState::PromotePawn;
                return;
            }
            self.update_turn();
            self.king_in_check = self.is_attacked(
                self.king_coords[self.turn][0] as isize,
                self.king_coords[self.turn][1] as isize,
            );
            self.check_for_mate();
            if self.show_fen {
                self.display_fen_string();
            }
        } else if matches!(self.board[self.y][self.x].piece, Piece::Empty)
            || self.board[self.y][self.x].color != self.turn
        {
            self.unhighlight_square(self.selected_piece[0], self.selected_piece[1]);
            self.unhighlight_moves();
            self.reset_cursor();
        } else {
            self.select_piece();
            self.unhighlight_moves();
            self.find_moves();
            self.highlight_moves();
            self.reset_cursor();
        }
    }
    fn update_square(&mut self, x: usize, y: usize) {
        write!(
            self.stdout,
            "{}{}{}{}",
            termion::cursor::Goto((x + 2) as u16, (y + 1) as u16),
            self.get_bg_color(x as u16, y as u16),
            self.board[y][x].icon,
            style::Reset
        )
        .unwrap();
    }

    fn highlight_square(&mut self, x: usize, y: usize) {
        write!(
            self.stdout,
            "{}{}{}{}",
            termion::cursor::Goto(x as u16 + 2, y as u16 + 1),
            color::Bg(color::Rgb(200, 100, 0)),
            self.board[y][x].icon,
            style::Reset,
        )
        .unwrap();
    }

    fn unhighlight_square(&mut self, x: usize, y: usize) {
        write!(
            self.stdout,
            "{}{}{}{}",
            termion::cursor::Goto(x as u16 + 2, y as u16 + 1),
            self.get_bg_color(x as u16, y as u16),
            self.board[y][x].icon,
            style::Reset,
        )
        .unwrap();
    }

    fn highlight_moves(&mut self) {
        for m in self.moves.clone() {
            self.highlight_square(m[0], m[1]);
            self.board[m[1]][m[0]].is_valid_move = true;
        }
    }

    fn unhighlight_moves(&mut self) {
        for m in self.moves.clone() {
            self.unhighlight_square(m[0], m[1]);
            self.board[m[1]][m[0]].is_valid_move = false;
        }
        self.moves.clear();
    }

    fn select_piece(&mut self) {
        if matches!(self.board[self.y][self.x].piece, Piece::Empty) {
            return;
        }

        // Deselect currently selected piece
        self.unhighlight_square(self.selected_piece[0], self.selected_piece[1]);
        self.selected_piece[0] = self.x;
        self.selected_piece[1] = self.y;

        // Select current piece
        self.highlight_square(self.x, self.y);
    }

    fn place_piece(&mut self, p: Piece, color: usize, x: usize, y: usize) {
        self.board[y][x] = Square::new(p, color);
        self.update_square(x, y);
    }

    fn empty_board(&mut self) {
        for y in 0..8 {
            for x in 0..8 {
                self.empty_square(x, y);
                self.update_square(x, y);
            }
        }
    }

    fn empty_square(&mut self, x: usize, y: usize) {
        if self.board[y][x].piece != Piece::Empty {
            self.board[y][x] = Square::new(Piece::Empty, 2);
        }
    }

    // Game data helper functions
    fn update_turn(&mut self) {
        if self.turn == 0 {
            self.turn = 1;
            self.fullmoves += 1;
        } else {
            self.turn = 0;
        }
    }

    fn update_en_passant_field(&mut self) {
        self.en_passant.clear();
        let sel_x = self.selected_piece[0];
        let sel_y = self.selected_piece[1];

        if self.board[sel_y][sel_x].piece != Piece::Pawn {
            return;
        }

        if (self.y as isize - sel_y as isize).abs() == 2 {
            if self.turn == 0 {
                self.en_passant.push([self.x, self.y + 1]);
            } else {
                self.en_passant.push([self.x, self.y - 1]);
            }
        }
    }

    fn update_castling_rights(&mut self) {
        let sel_x = self.selected_piece[0];
        let sel_y = self.selected_piece[1];
        let square = &self.board[sel_y][sel_x];

        if square.piece != Piece::King && square.piece != Piece::Rook {
            return;
        }

        if square.piece == Piece::King {
            self.castling_rights[self.turn] = [false, false];
            return;
        }

        if sel_x == 7 && self.castling_rights[self.turn][0] {
            self.castling_rights[self.turn][0] = false;
        } else if sel_x == 0 && self.castling_rights[self.turn][1] {
            self.castling_rights[self.turn][1] = false;
        } else {
            return;
        }
    }

    fn update_king_coords(&mut self) {
        let sel_x = self.selected_piece[0];
        let sel_y = self.selected_piece[1];

        if self.board[sel_y][sel_x].piece != Piece::King {
            return;
        }

        self.king_coords[self.turn] = [self.x, self.y];
    }

    fn update_halfmove_clock(&mut self) {
        let sel_x = self.selected_piece[0];
        let sel_y = self.selected_piece[1];
        if self.board[sel_y][sel_x].piece == Piece::Pawn
            || self.board[self.y][self.x].piece != Piece::Empty
        {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }
    }

    fn check_for_mate(&mut self) {
        let cur_x = self.x;
        let cur_y = self.y;
        for y in 0..8 {
            for x in 0..8 {
                if self.board[y][x].color == self.turn {
                    self.x = x;
                    self.y = y;
                    self.find_moves();
                    if !self.moves.is_empty() {
                        self.x = cur_x;
                        self.y = cur_y;
                        return;
                    }
                }
            }
        }

        if self.king_in_check {
            write!(
                self.stdout,
                "{}{}Checkmate!{}",
                termion::cursor::Goto(1, 11),
                color::Bg(color::Red),
                style::Reset
            )
            .unwrap();
        } else {
            write!(
                self.stdout,
                "{}{}Stalemate!{}",
                termion::cursor::Goto(1, 11),
                color::Bg(color::Red),
                style::Reset
            )
            .unwrap();
        }

        self.x = cur_x;
        self.y = cur_y;
        self.reset_cursor();
    }

    // Cursor Functions
    fn reset_cursor(&mut self) {
        write!(
            self.stdout,
            "{}",
            termion::cursor::Goto(self.cursor_x, self.cursor_y)
        )
        .unwrap();
        self.stdout.flush().unwrap();
    }

    fn mouse_move_cursor(&mut self, x: u16, y: u16) {
        if (2..=9).contains(&x) && (1..=8).contains(&y) {
            self.x = (x - 2) as usize;
            self.y = (y - 1) as usize;
            self.cursor_x = x;
            self.cursor_y = y;
            self.reset_cursor();
        }
    }

    fn left(&mut self) {
        if self.x > 0 {
            self.x -= 1;
            self.cursor_x -= 1;
            self.reset_cursor();
        }
    }

    fn right(&mut self) {
        if self.x < 7 {
            self.x += 1;
            self.cursor_x += 1;
            self.reset_cursor();
        }
    }

    fn down(&mut self) {
        if self.y < 7 {
            self.y += 1;
            self.cursor_y += 1;
            self.reset_cursor();
        }
    }

    fn up(&mut self) {
        if self.y > 0 {
            self.y -= 1;
            self.cursor_y -= 1;
            self.reset_cursor();
        }
    }

    //Keypress handlers
    fn handle_promote_pawn_event(&mut self, state: &mut KeyCaptureState) {
        write!(
            self.stdout,
            "{}{}{}q:Queen r:Rook n:Knight b:Bishop{}",
            termion::cursor::Goto(1, 10),
            termion::clear::AfterCursor,
            color::Bg(color::Red),
            style::Reset
        )
        .unwrap();
        self.reset_cursor();
        loop {
            let b = self.stdin.next().unwrap().unwrap();
            match b {
                Event::Key(Key::Char('q')) => {
                    self.place_piece(Piece::Queen, self.turn, self.x, self.y);
                    break;
                }
                Event::Key(Key::Char('r')) => {
                    self.place_piece(Piece::Rook, self.turn, self.x, self.y);
                    break;
                }
                Event::Key(Key::Char('n')) => {
                    self.place_piece(Piece::Knight, self.turn, self.x, self.y);
                    break;
                }
                Event::Key(Key::Char('b')) => {
                    self.place_piece(Piece::Bishop, self.turn, self.x, self.y);
                    break;
                }
                _ => (),
            }
        }
        *state = KeyCaptureState::Gameplay;
        self.update_turn();
    }

    fn handle_gameplay_event(&mut self, state: &mut KeyCaptureState) {
        write!(
            self.stdout,
            "{}{}{}q:Quit{}",
            termion::cursor::Goto(1, 10),
            termion::clear::AfterCursor,
            color::Bg(color::Red),
            style::Reset
        )
        .unwrap();
        self.reset_cursor();
        self.king_in_check = self.is_attacked(
            self.king_coords[self.turn][0] as isize,
            self.king_coords[self.turn][1] as isize,
        );

        if self.show_fen {
            self.display_fen_string();
        }

        loop {
            let b = self.stdin.next().unwrap().unwrap();
            match b {
                Event::Mouse(MouseEvent::Release(x, y)) => {
                    self.mouse_move_cursor(x, y);
                    self.handle_click_or_enter(state);
                }
                Event::Key(Key::Left) => self.left(),
                Event::Key(Key::Right) => self.right(),
                Event::Key(Key::Up) => self.up(),
                Event::Key(Key::Down) => self.down(),
                Event::Key(Key::Char('\n')) => {
                    self.handle_click_or_enter(state);
                }
                Event::Key(Key::Char('e')) => {
                    *state = KeyCaptureState::EditBoard;
                    return;
                }
                Event::Key(Key::Char('f')) => {
                    if self.show_fen {
                        self.show_fen = false;
                        write!(
                            self.stdout,
                            "{}{}{}",
                            termion::cursor::Goto(1, 12),
                            termion::clear::CurrentLine,
                            style::Reset
                        )
                        .unwrap();
                        self.reset_cursor();
                    } else {
                        self.show_fen = true;
                        self.display_fen_string()
                    }
                }
                Event::Key(Key::Char('c')) => {
                    if self.show_fen {
                        self.copy_fen_to_clipboard();
                    }
                }
                Event::Key(Key::Char('q')) => {
                    *state = KeyCaptureState::ExitGame;
                    return;
                }
                _ => (),
            }
        }
    }

    fn handle_edit_board_event(&mut self, state: &mut KeyCaptureState, piece_to_place: &mut Piece) {
        write!(
            self.stdout,
            "{}{}{}ESC:Exit c:Clear d:Delete{}k:King q:Queen r:Rook n:Knight b:Bishop p:Pawn{}",
            termion::cursor::Goto(1, 10),
            termion::clear::AfterCursor,
            color::Bg(color::Red),
            termion::cursor::Goto(1, 11),
            style::Reset,
        )
        .unwrap();
        self.unhighlight_square(self.selected_piece[0], self.selected_piece[1]);
        self.unhighlight_moves();
        self.reset_cursor();

        loop {
            let b = self.stdin.next().unwrap().unwrap();
            match b {
                Event::Mouse(MouseEvent::Release(x, y)) => {
                    self.mouse_move_cursor(x, y);
                }
                Event::Key(Key::Left) => self.left(),
                Event::Key(Key::Right) => self.right(),
                Event::Key(Key::Down) => self.down(),
                Event::Key(Key::Up) => self.up(),
                Event::Key(Key::Esc) => {
                    *state = KeyCaptureState::Gameplay;
                    break;
                }
                Event::Key(Key::Char('c')) => {
                    self.empty_board();
                    self.reset_cursor();
                }
                Event::Key(Key::Char('d')) => {
                    self.empty_square(self.x, self.y);
                    self.update_square(self.x, self.y);
                    self.reset_cursor();
                }
                Event::Key(Key::Char('k')) => {
                    *piece_to_place = Piece::King;
                    *state = KeyCaptureState::ChooseColour;
                    break;
                }
                Event::Key(Key::Char('q')) => {
                    *piece_to_place = Piece::Queen;
                    *state = KeyCaptureState::ChooseColour;
                    break;
                }
                Event::Key(Key::Char('r')) => {
                    *piece_to_place = Piece::Rook;
                    *state = KeyCaptureState::ChooseColour;
                    break;
                }
                Event::Key(Key::Char('n')) => {
                    *piece_to_place = Piece::Knight;
                    *state = KeyCaptureState::ChooseColour;
                    break;
                }
                Event::Key(Key::Char('b')) => {
                    *piece_to_place = Piece::Bishop;
                    *state = KeyCaptureState::ChooseColour;
                    break;
                }
                Event::Key(Key::Char('p')) => {
                    *piece_to_place = Piece::Pawn;
                    *state = KeyCaptureState::ChooseColour;
                    break;
                }
                _ => (),
            }
        }
    }

    fn handle_colour_chooser_event(&mut self, state: &mut KeyCaptureState, piece_to_place: &Piece) {
        write!(
            self.stdout,
            "{}{}{}w:White b:Black{}",
            termion::cursor::Goto(1, 10),
            termion::clear::AfterCursor,
            color::Bg(color::Red),
            style::Reset,
        )
        .unwrap();
        self.reset_cursor();
        loop {
            let b = self.stdin.next().unwrap().unwrap();
            match b {
                Event::Key(Key::Char('w')) => {
                    self.place_piece(piece_to_place.clone(), 0, self.x, self.y);
                    if *piece_to_place == Piece::King {
                        self.king_coords[0] = [self.x, self.y];
                        self.castling_rights[0] = [false, false];
                    }
                    break;
                }
                Event::Key(Key::Char('b')) => {
                    self.place_piece(piece_to_place.clone(), 1, self.x, self.y);
                    if *piece_to_place == Piece::King {
                        self.king_coords[1] = [self.x, self.y];
                        self.castling_rights[1] = [false, false];
                    }
                    break;
                }
                _ => (),
            }
        }
        *state = KeyCaptureState::EditBoard;
    }

    fn run_game(&mut self) {
        let mut state: KeyCaptureState = KeyCaptureState::Gameplay;
        let mut piece_to_place: Piece = Piece::Empty;
        loop {
            match state {
                KeyCaptureState::Gameplay => self.handle_gameplay_event(&mut state),
                KeyCaptureState::EditBoard => {
                    self.handle_edit_board_event(&mut state, &mut piece_to_place)
                }
                KeyCaptureState::ChooseColour => {
                    self.handle_colour_chooser_event(&mut state, &piece_to_place)
                }
                KeyCaptureState::PromotePawn => {
                    self.handle_promote_pawn_event(&mut state);
                }
                _ => return,
            }
        }
    }

    fn start(&mut self) {
        self.init_board();
        self.print_initial_board();
        write!(self.stdout, "{}", termion::cursor::Goto(2, 1)).unwrap();
        self.stdout.flush().unwrap();
        //self.fill_board_from_fen_string("4Q3/3N2p1/8/p4kPp/P4p1P/8/1P2PPB1/2R1K3 w - - 2 33".to_string());
        self.run_game();
        write!(
            self.stdout,
            "{}{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1),
            style::Reset,
        )
        .unwrap();
    }
}

fn main() {
    let stdout = MouseTerminal::from(stdout().lock().into_raw_mode().unwrap());
    let stdin = stdin().lock();
    init_game(stdout, stdin);
}
