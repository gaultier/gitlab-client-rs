use serde::Deserialize;
use std::env::VarError;
use std::io::{stdout, Stdout, Write};
use tui::backend::CrosstermBackend;
// use tui::layout::{Constraint, Direction, Layout};
use anyhow::{Context, Result};
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tokio::sync::mpsc;
use tui::layout::Constraint;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Cell, Row, Table};
use tui::Terminal;

#[derive(Debug, Deserialize)]
struct User {
    id: u64,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Pipeline {
    id: u64,
    project_id: u64,
    #[serde(rename(deserialize = "ref"))]
    reference: Option<String>,
    sha: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Job {
    #[serde(skip)]
    project_id: usize,
    created_at: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration: Option<f64>,
    id: u64,
    name: Option<String>,
    reference: Option<String>,
    stage: Option<String>,
    status: Option<String>,
    web_url: Option<String>,
    pipeline: Pipeline,
    user: User,
}

// shutdown the CLI and show terminal
fn shutdown(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn start_ui() -> Result<()> {
    let mut stdout = stdout();
    // Terminal initialization
    // not capturing mouse to make text select/copy possible
    execute!(stdout, EnterAlternateScreen)?;
    // see https://docs.rs/crossterm/0.17.7/crossterm/terminal/#raw-mode
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    loop {
        terminal.draw(|f| {
            let table = Table::new(vec![
                // Row can be created from simple strings.
                Row::new(vec!["Row11", "Row12", "Row13"]),
                // You can style the entire row.
                Row::new(vec!["Row21", "Row22", "Row23"]).style(Style::default().fg(Color::Blue)),
                // If you need more control over the styling you may need to create Cells directly
                Row::new(vec![
                    Cell::from("Row31"),
                    Cell::from("Row32").style(Style::default().fg(Color::Yellow)),
                    Cell::from(Spans::from(vec![
                        Span::raw("Row"),
                        Span::styled("33", Style::default().fg(Color::Green)),
                    ])),
                ]),
                // If a Row need to display some content over multiple lines, you just have to change
                // its height.
                Row::new(vec![
                    Cell::from("Row\n41"),
                    Cell::from("Row\n42"),
                    Cell::from("Row\n43"),
                ])
                .height(2),
            ])
            // You can set the style of the entire Table.
            .style(Style::default().fg(Color::White))
            // It has an optional header, which is simply a Row always visible at the top.
            .header(
                Row::new(vec!["Col1", "Col2", "Col3"])
                    .style(Style::default().fg(Color::Yellow))
                    // If you want some space between the header and the rest of the rows, you can always
                    // specify some margin at the bottom.
                    .bottom_margin(1),
            )
            // As any other widget, a Table can be wrapped in a Block.
            .block(Block::default().title("Table"))
            // Columns widths are constrained in the same way as Layout...
            .widths(&[
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(10),
            ])
            // ...and they can be separated by a fixed spacing.
            .column_spacing(1)
            // If you wish to highlight a row in any specific way when it is selected...
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            // ...and potentially show a symbol in front of the selection.
            .highlight_symbol(">>");

            let size = f.size();
            f.render_widget(table, size);
        })?;

        match read()? {
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: _,
            }) => break,
            _ => {}
        };
    }
    terminal.show_cursor()?;
    shutdown(terminal)?;
    Ok(())
}

#[derive(Debug)]
enum AppEvent {
    FetchJob(mpsc::Sender<Result<Vec<Job>>>, usize),
}

async fn fetch_project_jobs(
    client: reqwest::Client,
    token: Result<String, VarError>,
    project_id: usize,
) -> Result<Vec<Job>> {
    let mut req = client.get(format!(
        "https://gitlab.ppro.com/api/v4/projects/{}/jobs",
        project_id
    ));
    if let Ok(token) = token {
        req = req.header("PRIVATE-TOKEN", token.clone());
    }

    let json = req.send().await?.text().await?;

    let jobs: Vec<Job> = serde_json::from_str(&json).context("failed to parse to JSON")?;

    Ok(jobs
        .into_iter()
        .map(|job| Job {
            project_id: project_id,
            ..job
        })
        .collect::<Vec<_>>())
}

async fn handle_network_event<'a>(
    client: reqwest::Client,
    token: Result<String, VarError>,
    event: AppEvent,
) {
    match event {
        AppEvent::FetchJob(tx, project_id) => {
            tokio::spawn(async move {
                let jobs = fetch_project_jobs(client, token, project_id).await;
                if let Err(_) = tx.send(jobs).await {
                    eprintln!("Receiver dropped!");
                }
            });
        }
    }
}

#[tokio::main]
async fn start_network(mut io_rx: mpsc::Receiver<AppEvent>) {
    let token = std::env::var("GITLAB_TOKEN");
    let project_ids: Vec<u64> = vec![138, 125, 156, 889, 594];
    let client = reqwest::Client::new();
    let _projects_count = project_ids.len();

    while let Some(event) = io_rx.recv().await {
        handle_network_event(client.clone(), token.clone(), event).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (_sync_io_tx, sync_io_rx) = mpsc::channel::<AppEvent>(500);

    std::thread::spawn(move || {
        start_network(sync_io_rx);
    });
    start_ui().await?;
    Ok(())
}
