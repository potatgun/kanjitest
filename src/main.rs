use std::{
    fs::File, 
    io::Read
};

use tui::{
    widgets::Paragraph,
    backend::{CrosstermBackend, Backend},
    layout::{Constraint, Direction, Layout},
    Terminal
};

use crossterm::{
    execute,
    event::{
        self, 
        DisableMouseCapture, 
        EnableMouseCapture, 
        Event,
        KeyCode, 
        MouseEventKind
    },
    terminal::{
        disable_raw_mode, 
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen
    },
};


const MOUSE_SCROLL_AMOUNT: u16 = 5;
const KEYBOARD_SCROLL_AMOUT: u16 = 1;

#[derive(Debug)]
enum Error {
    OpenFile(std::io::Error),
    ReadFileContent(std::io::Error),
    Draw(std::io::Error),
    Event(std::io::Error),
    Setup(std::io::Error),
    Restore(std::io::Error),
}

type Result<T> = ::std::result::Result<T, Error>;

struct Program { 
    // left side will have kanji on it
    left_side: String,
    // right side will have meaning and reading on it
    // it can be hidden with spacebar
    right_side: String,
    // this bool determines if right side is supposed to be hidden
    hidden: bool,
    // vertical sroll of the file
    scroll: u16,
    // amount of lines in the file
    length: u16,
    // this bool determines if program should be left
    leave: bool,
}

impl Program {
    // input format looks like so 
    // 日:
    //     day, sun, Japan
    //     ニチ, ジツ
    //     ひ, -び, -か
    //
    // all the uneeded lines for left side will be replaced with an empty line
    // same for the right side
    fn new(file: &str) -> Result<Self> { 
        let mut result = Self {
            left_side: String::new(),
            right_side: String::new(),
            hidden: false,
            scroll: 0u16,
            length: 0u16,
            leave:  false,
        };

        result.update_file(file)?;

        Ok(result)
    }

    fn update_file(&mut self, file: &str) -> Result<()> { 
        let mut content = String::new();
        File::options()
            .read(true)
            .write(false)
            .open(file)
            .map_err(Error::OpenFile)?
            .read_to_string(&mut content)
            .map_err(Error::ReadFileContent)?;

        // replace every uneeded line with an empty one for the left side
        let mut left_side = String::new();
        for line in content.lines() {
            if line.contains(':') || line == "-" {
                left_side.push_str(line);
            }

            left_side.push('\n');
        }

        // replace every uneeded line with an empty one for the right side
        let mut right_side = String::new();
        for line in content.lines() {
            if !line.contains(':') && line != "-" {
                right_side.push_str(line);
            }

            right_side.push('\n');
        }

        self.left_side  = left_side;
        self.right_side = right_side;
        self.length = content.lines().count() as u16;

        Ok(())
    }

    fn scroll_up(&mut self, amount: u16) { 
        if self.scroll >= amount {
            self.scroll -= amount;
        } else {
            self.scroll = 0;
        }
    }

    fn scroll_down(&mut self, amount: u16) { 
        if self.scroll + amount > self.length {
            // -1 so that the last line is visible when scrolled all the way
            self.scroll = self.length - 1;
        } else {
            self.scroll += amount;
        }
    }

    fn key_input(&mut self, key: KeyCode) { 
        match key {
            // toggle hidden
            KeyCode::Char(' ') => self.hidden = !self.hidden,

            // scrolll up 
            KeyCode::Char('k') => self.scroll_up(KEYBOARD_SCROLL_AMOUT),
            KeyCode::Up => self.scroll_up(KEYBOARD_SCROLL_AMOUT),

            // scroll down 
            // scroll is limited at the bottom of the screen 
            KeyCode::Char('j') => self.scroll_down(KEYBOARD_SCROLL_AMOUT),
            KeyCode::Down => self.scroll_down(KEYBOARD_SCROLL_AMOUT),

            // leave the program here
            KeyCode::Esc => self.leave = true,

            _ => ()
        }
    }

    fn mouse_input(&mut self, event: MouseEventKind) { 
        match event {
            MouseEventKind::ScrollUp => self.scroll_up(MOUSE_SCROLL_AMOUNT),    
            MouseEventKind::ScrollDown => self.scroll_down(MOUSE_SCROLL_AMOUNT),
            _ => ()
        }
    }

    fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> { 
        while !self.leave {
            terminal.draw(|f| {
                // split screen into two parts
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                                 Constraint::Percentage(40),
                                 Constraint::Percentage(60),
                    ])
                    .split(f.size());

                // render left side
                let paragraph = 
                    Paragraph::new(self.left_side.clone())
                    .scroll((self.scroll, 0));
                f.render_widget(paragraph, chunks[0]);

                // render right side if it is not hidden
                if !self.hidden {
                    let paragraph = 
                        Paragraph::new(self.right_side.clone())
                        .scroll((self.scroll, 0));
                    f.render_widget(paragraph, chunks[1]);
                }
            }).map_err(Error::Draw)?;

            match event::read().map_err(Error::Event)? {
                Event::Key(key) => self.key_input(key.code),
                Event::Mouse(mouse_event) => self.mouse_input(mouse_event.kind), 
                _ => (),
            }
        }

        Ok(())
    }
}

macro_rules! restore_terminal {
    ($terminal: expr) => {{
        disable_raw_mode().map_err(Error::Restore)?;
        execute!(
            $terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).map_err(Error::Restore)?;

        $terminal.show_cursor().map_err(Error::Restore)?;
        Ok(())
    }}
}

macro_rules! restore_panic {
    ($terminal: expr, $error: expr) => {{
        restore_terminal!($terminal)?;
        panic!("{:#?}", $error);
    }}
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("usage: kanjitest filename");
        std::process::exit(0);
    }

    // setup terminal 
    enable_raw_mode().map_err(Error::Setup)?;
    let mut stdout = std::io::stdout();
    execute!(stdout, 
             EnterAlternateScreen, 
             EnableMouseCapture
    ).map_err(Error::Setup)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(Error::Setup)?;

    let path = &args[1];
    let program = match Program::new(path) {
        Ok(p) => p,
        Err(e) => restore_panic!(terminal, e)
    };

    match program.run(&mut terminal) {
        Ok(_) => restore_terminal!(terminal)?,
        Err(e) => restore_panic!(terminal, e)
    }

    Ok(())
}

