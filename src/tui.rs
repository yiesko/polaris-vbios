use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, execute, queue, style};

use crate::compare::render_compare;
use crate::render::color::{Palette, pad, truncate, visible_truncate, wrap_ansi};
use crate::render::sections::Section;
use crate::render::text::render_section;
use crate::rom::{self, types::ParsedRom};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    A,
    B,
    Compare,
}

struct State {
    rom_a: ParsedRom,
    rom_b: Option<ParsedRom>,
    selected: usize,
    scroll: usize,
    view: ViewMode,
    diff_only: bool,
    show_help: bool,
    status_message: Option<String>,
}

impl State {
    fn content_lines(&self, pal: &Palette) -> Vec<String> {
        let section = Section::ALL[self.selected];
        let text = match self.view {
            ViewMode::A => render_section(&self.rom_a, section, pal, None),
            ViewMode::B => match &self.rom_b {
                Some(b) => render_section(b, section, pal, None),
                None => String::new(),
            },
            ViewMode::Compare => match &self.rom_b {
                Some(b) => {
                    // render_compare already prints the general header; here
                    // we want only the section, so we call the single-section
                    // function through the comparison wrapper.
                    compare_one_section(&self.rom_a, b, section, pal, self.diff_only)
                }
                None => String::new(),
            },
        };
        text.lines().map(|l| l.to_string()).collect()
    }

    fn view_label(&self) -> String {
        match self.view {
            ViewMode::A => "ROM A".to_string(),
            ViewMode::B => "ROM B".to_string(),
            ViewMode::Compare => {
                if self.diff_only {
                    "Comparison A × B [diff-only]".to_string()
                } else {
                    "Comparison A × B".to_string()
                }
            }
        }
    }
}

fn compare_one_section(
    a: &ParsedRom,
    b: &ParsedRom,
    section: Section,
    pal: &Palette,
    diff_only: bool,
) -> String {
    // render_compare builds the full report; we extract only the block
    // for the requested section by reusing the same function with a 1-element slice.
    // When diff_only is active, there is an extra warning line right
    // after the title - hence the number of skipped lines varies.
    let skip = if diff_only { 3 } else { 2 };
    render_compare(a, b, &[section], pal, diff_only, None)
        .lines()
        .skip(skip)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn run(rom_a_path: PathBuf, rom_b_path: Option<PathBuf>) -> Result<()> {
    let rom_a = rom::parse_rom(&rom_a_path)?;
    let rom_b = match rom_b_path {
        Some(p) => Some(rom::parse_rom(&p)?),
        None => None,
    };

    let mut state = State {
        view: ViewMode::A,
        rom_a,
        rom_b,
        selected: 0,
        scroll: 0,
        diff_only: false,
        show_help: false,
        status_message: None,
    };

    terminal::enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;

    let result = event_loop(&mut out, &mut state);

    execute!(out, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn event_loop(out: &mut impl Write, state: &mut State) -> Result<()> {
    let pal = Palette::new(true);
    loop {
        draw(out, state, &pal)?;
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if key.code != KeyCode::Char('y') {
                state.status_message = None;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if state.show_help {
                        state.show_help = false;
                    } else {
                        break;
                    }
                }
                KeyCode::Char('?') => {
                    state.show_help = !state.show_help;
                }
                KeyCode::Up | KeyCode::Char('k') if !state.show_help && state.selected > 0 => {
                    state.selected -= 1;
                    state.scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j')
                    if !state.show_help && state.selected + 1 < Section::ALL.len() =>
                {
                    state.selected += 1;
                    state.scroll = 0;
                }
                KeyCode::PageUp if !state.show_help => {
                    state.scroll = state.scroll.saturating_sub(10);
                }
                KeyCode::PageDown if !state.show_help => {
                    state.scroll += 10;
                }
                KeyCode::Tab if !state.show_help && state.rom_b.is_some() => {
                    state.view = match state.view {
                        ViewMode::A => ViewMode::B,
                        ViewMode::B => ViewMode::Compare,
                        ViewMode::Compare => ViewMode::A,
                    };
                    state.scroll = 0;
                }
                KeyCode::Char('d') if !state.show_help && state.rom_b.is_some() => {
                    state.diff_only = !state.diff_only;
                    state.scroll = 0;
                }
                KeyCode::Char('y') if !state.show_help => {
                    let content = state.content_lines(&pal).join("\n");
                    state.status_message =
                        Some(match crate::clipboard::copy_to_clipboard(&content) {
                            Ok(()) => format!(
                                "section copied to clipboard ({} lines, via OSC52)",
                                content.lines().count()
                            ),
                            Err(e) => format!("copy failed: {e}"),
                        });
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(out: &mut impl Write, state: &State, pal: &Palette) -> Result<()> {
    let (cols, rows) = terminal::size()?;
    let (cols, rows) = (cols as usize, rows as usize);
    // Left column: full section name (no truncation) when the terminal
    // allows it; never smaller than a readable size.
    let max_label = Section::ALL
        .iter()
        .map(|s| s.label().chars().count())
        .max()
        .unwrap_or(20);
    let left_w = (max_label + 3).clamp(18, cols.saturating_sub(2).max(18) / 2);
    let body_rows = rows.saturating_sub(3);
    let content_w = cols.saturating_sub(left_w + 2);
    // Wrap the content pane into fixed-width lines (auto line-wrap, so
    // long tables fit without shrinking the terminal / Ctrl+minus).
    let wrapped: Vec<String> = state
        .content_lines(pal)
        .iter()
        .flat_map(|l| wrap_ansi(l, content_w))
        .collect();

    queue!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

    // Title bar
    let title = format!(
        " polaris-vbios · {} · [{}] ",
        state.rom_a.file_name,
        state.view_label()
    );
    let title = truncate(&title, cols);
    queue!(out, style::Print(pal.accent(&title)))?;
    queue!(out, cursor::MoveTo(0, 1))?;
    queue!(out, style::Print(pal.label(&"─".repeat(cols))))?;

    if state.show_help {
        draw_help(out, body_rows, cols, pal)?;
    } else {
        draw_body(out, state, pal, body_rows, left_w, &wrapped)?;
    }

    queue!(out, cursor::MoveTo(0, rows.saturating_sub(1) as u16))?;
    let footer = if state.show_help {
        " any key closes help ".to_string()
    } else if let Some(msg) = &state.status_message {
        format!(" {} ", msg)
    } else {
        let scroll_info = {
            let total = wrapped.len();
            if total > body_rows {
                format!(
                    " · lines {}–{}/{}",
                    state.scroll + 1,
                    (state.scroll + body_rows).min(total),
                    total
                )
            } else {
                String::new()
            }
        };
        let base = if state.rom_b.is_some() {
            " ↑↓/jk nav · PgUp/PgDn scroll · Tab A/B/Compare · d diff-only · y copy · ? help · q quit "
        } else {
            " ↑↓/jk nav · PgUp/PgDn scroll · y copy · ? help · q quit "
        };
        format!("{base}{scroll_info}")
    };
    let footer_styled = if state.status_message.is_some() {
        pal.good(&footer)
    } else {
        pal.label(&footer)
    };
    queue!(out, style::Print(visible_truncate(&footer_styled, cols)))?;

    out.flush()?;
    Ok(())
}

fn draw_body(
    out: &mut impl Write,
    state: &State,
    pal: &Palette,
    body_rows: usize,
    left_w: usize,
    wrapped: &[String],
) -> Result<()> {
    let visible_content: Vec<&String> = wrapped.iter().skip(state.scroll).take(body_rows).collect();

    for row in 0..body_rows {
        queue!(out, cursor::MoveTo(0, (row + 2) as u16))?;

        // left column: full section name, never truncated
        if row < Section::ALL.len() {
            let sec = Section::ALL[row];
            let marker = if row == state.selected { "› " } else { "  " };
            let raw = format!("{marker}{}", sec.label());
            let padded = pad(&raw, left_w);
            let padded = if padded.chars().count() > left_w {
                truncate(&padded, left_w)
            } else {
                padded
            };
            let printed = if row == state.selected {
                pal.value(&padded)
            } else {
                padded
            };
            queue!(out, style::Print(printed))?;
        } else {
            queue!(out, style::Print(" ".repeat(left_w)))?;
        }

        queue!(out, style::Print(pal.label("│ ")))?;

        if let Some(line) = visible_content.get(row) {
            queue!(out, style::Print(line))?;
        }
    }
    Ok(())
}

fn draw_help(out: &mut impl Write, body_rows: usize, cols: usize, pal: &Palette) -> Result<()> {
    let lines = [
        "Navigation",
        "  ↑ / k          previous section",
        "  ↓ / j          next section",
        "  PgUp / PgDn    scroll current section content",
        "",
        "Comparison (2 ROMs only)",
        "  Tab            cycle through ROM A, ROM B and Comparison",
        "  d              toggle diff-only mode (show only differing fields)",
        "",
        "General",
        "  y              copy current section content to clipboard (via OSC52)",
        "  ?              show/hide this help",
        "  q / Esc        quit (or close help if open)",
    ];
    for (row, line) in lines.iter().enumerate().take(body_rows) {
        queue!(out, cursor::MoveTo(2, (row + 2) as u16))?;
        let styled = if line.starts_with(' ') {
            line.to_string()
        } else {
            pal.title(line)
        };
        queue!(out, style::Print(truncate(&styled, cols.saturating_sub(4))))?;
    }
    Ok(())
}
