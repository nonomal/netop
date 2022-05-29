use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use netraffic::Filter;
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::{app::App, ui};

pub enum InputMode {
    Normal,
    Editing,
}

pub fn run(tick_rate: Duration, enhanced_graphics: bool) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;
        if app.rules.len() > 0 && last_tick.elapsed() >= Duration::from_millis(1000) {
            let current_rule = &app.rules[app.index];
            if app.chart.len() >= 100 {
                app.chart.pop();
            }
            app.chart.insert(
                0,
                app.traffic
                    .clone()
                    .get_data()
                    .get(current_rule)
                    .unwrap()
                    .len,
            );
            let total = app
                .traffic
                .clone()
                .get_data()
                .get(current_rule)
                .unwrap()
                .total;
            if app.net_speed.len() >= 100 {
                app.window[0] += 1.0;
                app.window[1] += 1.0;
                app.net_speed.remove(0);
            }
            app.net_speed
                .push((app.second as f64, (total - app.last_total) as f64));
            app.last_total = total;
            app.second += 1;
            last_tick = Instant::now();
        }

        let tick_rate = Duration::from_millis(250);
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let input: String = app.input.drain(..).collect();
                            app.rules.push(input.clone());
                            app.traffic
                                .add_listener(Filter::new("en0".to_string(), input));
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}