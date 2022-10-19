use std::{fs::File, io::Read, path::Path};

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
    fn new(file: &str) -> Self { 
        let mut result = Self {
            left_side: String::new(),
            right_side: String::new(),
            hidden: false,
            scroll: 0u16,
            length: 0u16,
        };

        result.update_file(file);

        result
    }

    fn update_file(&mut self, file: &str) { 
        let mut content = String::new();
        File::options()
            .read(true)
            .write(false)
            .open(file)
            .expect("couldn't open the file")
            .read_to_string(&mut content)
            .expect("couldn't read the file content");

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
    }

    fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> std::io::Result<()> { 
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
            })?;

            // TODO: enter key to choose file
            if let Event::Key(key) = event::read()? {
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
                        // -1 so the last line is visible when scrolled all the way
                        if self.scroll != self.length - 1 {
                            self.scroll += 1;
                        }
                    },
                    KeyCode::Down => {
                        // -1 so the last line is visible when scrolled all the way
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

// TODO: as of right now it is assumed 
//       that the input file is in the specified format
//       it makes sense to error if the file format is wrong
//       but idk the good way of doing it
//       like dictionary maybe?
//
// TODO: multiple files
fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("usage: kanjitest filename");
        std::process::exit(0);
    }

    if !Path::new(&args[1]).is_file() {
        println!("the argument is not a file");
        std::process::exit(0);
    }

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let program = Program::new(&args[1]);
    let run_result = program.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    terminal.show_cursor()?;


    // doing it here so the terminal is restored before panicing
    run_result?;

    Ok(())
}
