use std::{
    error::Error,
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};

#[derive(Clone)]
struct Disk {
    path: String,
    size: String,
    model: String,
}

struct BrowserEntry {
    path: PathBuf,
    label: String,
    is_dir: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Screen {
    Iso,
    Usb,
    Confirm,
}

fn external_disks() -> Result<Vec<Disk>, Box<dyn Error>> {
    let output = Command::new("lsblk")
        .args(["--json", "-d", "-o", "PATH,SIZE,MODEL,TRAN,TYPE"])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "lsblk failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let data: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    Ok(data["blockdevices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|d| {
            d["type"].as_str() == Some("disk")
                && d["tran"].as_str() == Some("usb")
        })
        .map(|d| Disk {
            path: d["path"].as_str().unwrap_or("?").to_string(),
            size: d["size"].as_str().unwrap_or("?").to_string(),
            model: d["model"].as_str().unwrap_or("").trim().to_string(),
        })
        .collect())
}

fn directory_entries(dir: &Path) -> io::Result<Vec<BrowserEntry>> {
    let mut entries = Vec::new();

    if let Some(parent) = dir.parent() {
        if parent != dir {
            entries.push(BrowserEntry {
                path: parent.to_path_buf(),
                label: "../".to_string(),
                is_dir: true,
            });
        }
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        // Don't follow symlinks in this simple file browser.
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let is_dir = file_type.is_dir();
        let is_iso = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("iso"));

        if is_dir || (file_type.is_file() && is_iso) {
            let name = entry.file_name().to_string_lossy().into_owned();
            entries.push(BrowserEntry {
                path,
                label: if is_dir { format!("{name}/") } else { name },
                is_dir,
            });
        }
    }

let sort_start = 1.min(entries.len());
entries[sort_start..].sort_by_key(|entry| {
    (!entry.is_dir, entry.label.to_lowercase())
});


    Ok(entries)
}

fn run_ui(
    disks: &[Disk],
    start_dir: PathBuf,
) -> Result<Option<(PathBuf, Disk)>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> Result<Option<(PathBuf, Disk)>, Box<dyn Error>> {
        let mut screen = Screen::Iso;
        let mut current_dir = start_dir;
        let mut files = directory_entries(&current_dir)?;
        let mut selected = ListState::default();
        let mut index = 0usize;
        let mut iso: Option<PathBuf> = None;
        let mut selected_disk: Option<Disk> = None;
        let mut typed_confirmation = String::new();
        let mut notice = String::new();

        loop {
            if screen == Screen::Iso {
                files = directory_entries(&current_dir)?;
                if files.is_empty() {
                    index = 0;
                    selected.select(None);
                } else {
                    index = index.min(files.len() - 1);
                    selected.select(Some(index));
                }
            } else if screen == Screen::Usb {
                if disks.is_empty() {
                    selected.select(None);
                } else {
                    index = index.min(disks.len() - 1);
                    selected.select(Some(index));
                }
            }

            terminal.draw(|frame| {
                let areas = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(frame.area());

                let title = match screen {
                    Screen::Iso => format!("Choose ISO — {}", current_dir.display()),
                    Screen::Usb => format!(
                        "Choose USB — ISO: {}",
                        iso.as_deref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    ),
                    Screen::Confirm => "Confirm selection — no data will be written".to_string(),
                };

                frame.render_widget(
                    Paragraph::new(title)
                        .block(Block::default().borders(Borders::ALL)),
                    areas[0],
                );

                let items: Vec<ListItem> = match screen {
                    Screen::Iso => files
                        .iter()
                        .map(|entry| ListItem::new(entry.label.clone()))
                        .collect(),
                    Screen::Usb => disks
                        .iter()
                        .map(|d| {
                            ListItem::new(format!("{}  {}  {}", d.path, d.size, d.model))
                        })
                        .collect(),
                    Screen::Confirm => {
                        let disk_text = selected_disk.as_ref().map_or(
                            "No USB selected".to_string(),
                            |d| format!("USB: {}  {}  {}", d.path, d.size, d.model),
                        );
                        vec![
                            ListItem::new(format!(
                                "ISO: {}",
                                iso.as_deref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default()
                            )),
                            ListItem::new(disk_text),
                            ListItem::new("This is only a selection preview; nothing will be written."),
                        ]
                    }
                };

                let list_title = match screen {
                    Screen::Iso => "ISO files and folders",
                    Screen::Usb => "USB devices reported by lsblk",
                    Screen::Confirm => "Review",
                };

                let list = List::new(items)
                    .block(Block::default().title(list_title).borders(Borders::ALL))
                    .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan))
                    .highlight_symbol("> ");

                frame.render_stateful_widget(list, areas[1], &mut selected);

                let help = match screen {
    Screen::Iso => {
        "↑/↓ select  Enter open folder/choose ISO  q quit".to_string()
    }
    Screen::Usb => {
        "↑/↓ select  Enter review  Esc back to ISO  q quit".to_string()
    }
    Screen::Confirm => format!(
        "Entered: [{}] — type {} exactly, then Enter; Backspace edits; Esc back",
        typed_confirmation,
        selected_disk.as_ref().map_or("USB path", |d| d.path.as_str())
    ),
};





                let footer = if notice.is_empty() {
                    help
                } else {
                    format!("{notice} | {help}")
                };

                frame.render_widget(
                    Paragraph::new(footer).block(Block::default().borders(Borders::ALL)),
                    areas[2],
                );
            })?;

            notice.clear();

            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    match screen {
                        Screen::Iso => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                            KeyCode::Down if !files.is_empty() => {
                                index = (index + 1).min(files.len() - 1);
                            }
                            KeyCode::Up if !files.is_empty() => {
                                index = index.saturating_sub(1);
                            }
                            KeyCode::Enter if !files.is_empty() => {
                                let entry = &files[index];
                                if entry.is_dir {
                                    current_dir = entry.path.clone();
                                    index = 0;
                                } else {
                                    iso = Some(entry.path.canonicalize()?);
                                    screen = Screen::Usb;
                                    index = 0;
                                }
                            }
                            _ => {}
                        },
                        Screen::Usb => match key.code {
                            KeyCode::Char('q') => return Ok(None),
                            KeyCode::Esc => {
                                screen = Screen::Iso;
                                index = 0;
                            }
                            KeyCode::Down if !disks.is_empty() => {
                                index = (index + 1).min(disks.len() - 1);
                            }
                            KeyCode::Up if !disks.is_empty() => {
                                index = index.saturating_sub(1);
                            }
                            KeyCode::Enter if !disks.is_empty() => {
                                selected_disk = Some(disks[index].clone());
                                typed_confirmation.clear();
                                screen = Screen::Confirm;
                            }
                            _ => {}
                        },
                        Screen::Confirm => match key.code {
                            KeyCode::Char('q') => return Ok(None),
                            KeyCode::Esc => {
                                screen = Screen::Usb;
                                typed_confirmation.clear();
                            }
                            KeyCode::Backspace => {
                                typed_confirmation.pop();
                            }
                            KeyCode::Char(c) => {
                                typed_confirmation.push(c);
                            }
                            KeyCode::Enter => {
                                if let (Some(iso_path), Some(disk)) =
                                    (iso.clone(), selected_disk.clone())
                                {
                                    if typed_confirmation == disk.path {
                                        return Ok(Some((iso_path, disk)));
                                    }
                                    notice = "Device path did not match.".to_string();
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn write_iso(iso: &Path, disk: &Disk) -> Result<(), Box<dyn Error>> {
    eprintln!();
    eprintln!("WARNING: this will erase everything on {}.", disk.path);
    eprintln!("ISO:  {}", iso.display());
    eprintln!("USB:  {} — {} {}", disk.path, disk.size, disk.model);
    eprintln!("Make sure this is the correct USB device.");
    eprint!("To proceed, type ERASE {} exactly: ", disk.path);

    let mut confirmation = String::new();
    io::stdin().read_line(&mut confirmation)?;

    if confirmation.trim() != format!("ERASE {}", disk.path) {
        println!("Cancelled; nothing was written.");
        return Ok(());
    }

    let status = Command::new("sudo")
        .arg("dd")
        .arg(format!("if={}", iso.display()))
        .arg(format!("of={}", disk.path))
        .args(["bs=4M", "status=progress", "conv=fsync"])
        .status()?;

    if !status.success() {
        return Err(format!("dd failed with status: {status}").into());
    }

    let status = Command::new("sudo").arg("sync").status()?;
    if !status.success() {
        return Err(format!("sync failed with status: {status}").into());
    }

    println!("Write completed. Safely eject the USB before unplugging it.");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);

    let disks = external_disks()?;

  if let Some((iso, disk)) = run_ui(&disks, home)? {
    write_iso(&iso, &disk)?;
} else {
    println!("Cancelled; nothing was written.");
}


    Ok(())
}

