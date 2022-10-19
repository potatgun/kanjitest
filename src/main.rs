use std::{fs::File, io::{Read, Stdout}, path::Path, panic::PanicInfo};

use tui::{
    widgets::Paragraph,
    backend::{CrosstermBackend, Backend},
    Terminal, layout::{Constraint, Direction, Layout}
};

use crossterm::{
    execute,
    event::{
        self, 
        DisableMouseCapture, 
        EnableMouseCapture, 
        Event,
        KeyCode
    },
    terminal::{
        disable_raw_mode, 
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen
    },
};


#[derive(Debug)]
enum Error {
    CantOpenFile(std::io::Error),
    CantReadFileContent(std::io::Error),
    GenericIoError(std::io::Error)
}

type Result<T> = ::std::result::Result<T, Error>;

struct Program { 
    // left side will have kanji on it
    left_side: String,
    // right side will have meaning and reading on it
    // it can be hidden with spacebar
    right_side: String,
    // bool determines if right side is supposed to be hidden
    hidden: bool,
    // vertical sroll of the file
    scroll: u16,
    // amout of lines in the file
    length: u16,
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
            .map_err(Error::CantOpenFile)?
            .read_to_string(&mut content)
            .map_err(Error::CantReadFileContent)?;

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

    fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> Result<()> { 
        loop {
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
            }).map_err(Error::GenericIoError)?;

            if let Event::Key(key) = event::read().map_err(Error::GenericIoError)? {
                match key.code {
                    // toggle hidden
                    KeyCode::Char(' ') => {
                        self.hidden = !self.hidden
                    }

                    // scrolll up (?)
                    KeyCode::Char('k') => {
                        if self.scroll != 0 {
                            self.scroll -= 1;
                        }
                    },
                    KeyCode::Up => {
                        if self.scroll != 0 {
                            self.scroll -= 1;
                        }
                    },

                    // scroll down 
                    // scroll is limited at the bottom of the screen 
                    KeyCode::Char('j') => {
                        // -1 so that the last line is visible when scrolled all the way
                        if self.scroll != self.length - 1 {
                            self.scroll += 1;
                        }
                    },
                    KeyCode::Down => {
                        // -1 so that the last line is visible when scrolled all the way
                        if self.scroll != self.length - 1 {
                            self.scroll += 1;
                        }
                    },

                    // leave the program here
                    KeyCode::Esc => {
                        return Ok(())
                    },

                    _ => ()
                }
            }
        }
    }
}

macro_rules! restore_terminal {
    ($terminal: expr) => {{
        disable_raw_mode().map_err(Error::GenericIoError)?;
        execute!(
            $terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
            ).map_err(Error::GenericIoError)?;

        $terminal.show_cursor().map_err(Error::GenericIoError)?;
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

    enable_raw_mode().map_err(Error::GenericIoError)?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(Error::GenericIoError)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(Error::GenericIoError)?;

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

