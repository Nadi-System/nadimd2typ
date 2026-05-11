use anyhow::Context;
use clap::Parser;
use pulldown_cmark::{Alignment, CodeBlockKind, CowStr, Event, HeadingLevel, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

mod cmark_events;
mod code_args;
mod output;

use code_args::*;

#[derive(Parser)]
struct CliArgs {
    // requires output so that we don't have to make a filename
    /// Output a markdown file instead of typst
    #[arg(short, long, requires = "output")]
    markdown: bool,
    /// prelude to include in typst file
    #[arg(short, long)]
    prelude: Option<PathBuf>,
    /// Output file to write, if not writes a file with .typ extension
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Input markdown file
    input: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = CliArgs::parse();

    let content = std::fs::read_to_string(&cli.input)?;
    let events = process_nadi_tasks(
        &content,
        &cli.input
            .parent()
            .context("Can not determine parent path")?,
    )?;

    let out = cli.output.unwrap_or_else(|| {
        if cli.markdown {
            // Clap will make sure this is not called, but writing
            // this code here just in case
            cli.input.with_file_name("output.md")
        } else {
            cli.input.with_extension("typ")
        }
    });
    let file = std::fs::File::create(out)?;
    let mut writer = std::io::BufWriter::new(file);
    if let Some(p) = &cli.prelude {
        writeln!(writer, "{}", std::fs::read_to_string(p)?)?;
    }
    if cli.markdown {
        let mut buf = String::new();
        cmark(events.into_iter(), &mut buf)?;
        writeln!(writer, "{}", buf)?;
    } else {
        write_typst(&mut writer, events)?;
    }
    Ok(())
}

pub fn process_nadi_tasks<'a, 'b>(chap: &'a str, pwd: &'b Path) -> anyhow::Result<Vec<Event<'a>>> {
    enum State {
        None,
        Open,
        Gather,
    }

    let mut state = State::None;
    let options = mdbook_markdown::MarkdownOptions::default();
    let mut parser = mdbook_markdown::new_cmark_parser(chap, &options);
    let mut args = String::new();
    let mut handler: CodeHandler = run_task;

    let mut task_script = String::new();
    parser.try_fold(vec![], |mut acc, ref e| -> anyhow::Result<Vec<Event<'_>>> {
        use CodeBlockKind::*;
        use CowStr::*;
        use Event::*;
        use State::*;
        match (e, &mut state) {
            (Start(Tag::CodeBlock(Fenced(Borrowed(mark)))), None) => {
                acc.push(Start(Tag::CodeBlock(Fenced(Borrowed(
                    mark.split(' ').next().unwrap_or_default(),
                )))));
                if let Some((h, a)) = nadi_code_args(mark) {
                    state = Open;
                    args = a;
                    handler = h;
                }
            }
            (Text(Borrowed(txt)), Open) => {
                acc.push(e.clone());
                task_script.clear();
                task_script.push_str(txt);
                state = Gather;
            }
            (Text(Borrowed(txt)), Gather) => {
                task_script.push_str(txt);
                acc.push(e.clone());
            }
            (End(TagEnd::CodeBlock), Gather) => {
                state = None;
                acc.push(e.clone());
                let response = handler(&task_script, &args, pwd)?;
                acc.extend(response);
            }
            _ => {
                acc.push(e.clone());
            }
        };
        Ok(acc)
    })
}

#[derive(Default)]
struct MdTable {
    aligns: Vec<&'static str>,
    headers: Vec<String>,
    on_cell: bool,
    thiscell: String,
    cells: Vec<String>,
}

pub fn write_typst(writer: &mut BufWriter<File>, events: Vec<Event>) -> anyhow::Result<()> {
    let mut table: Option<MdTable> = None;
    let mut list: Option<u64> = None;
    let mut consec_par = false;
    let mut in_listitem = false;
    let mut in_code = false;
    for event in events {
        match event {
            Event::Code(c) => {
                if let Some(table) = &mut table {
                    table.thiscell.push_str(&format!("`{c}`"));
                } else {
                    write!(writer, "`{c}`")?
                }
            }
            Event::Text(c) => {
                let txt = if in_code {
                    let l = c
                        .lines()
                        .map(|l| l.trim_start_matches('!'))
                        .collect::<Vec<&str>>();
                    format!("{}\n", l.join("\n"))
                } else {
                    escape_typst(c)
                };
                if let Some(table) = &mut table {
                    table.thiscell.push_str(&txt);
                } else {
                    write!(writer, "{txt}")?
                }
            }
            Event::Html(_) => return Err(anyhow::Error::msg("HTML block not supported")),
            Event::SoftBreak => write!(writer, "\n")?,
            Event::HardBreak => write!(writer, "\n\n")?,
            // it makes four empty line, but overkill better than incorrect
            Event::Start(Tag::Paragraph) => {
                if !(in_listitem | consec_par) {
                    writeln!(writer, "\n\n")?
                }
            }
            Event::End(TagEnd::Paragraph) => {
                writeln!(writer, "\n")?;
                consec_par = true;
                continue;
            }
            Event::Start(Tag::Strong) => write!(writer, "*")?,
            Event::End(TagEnd::Strong) => write!(writer, "*")?,
            Event::Start(Tag::Link { dest_url, .. }) => {
                if let Some(table) = &mut table {
                    table.thiscell.push_str(&format_link(dest_url));
                } else {
                    write!(writer, "{}", format_link(dest_url))?
                }
            }
            Event::Start(Tag::CodeBlock(ck)) => {
                match ck {
                    CodeBlockKind::Fenced(lang) => writeln!(writer, "\n``````{lang}")?,
                    CodeBlockKind::Indented => writeln!(writer, "\n``````")?,
                }
                in_code = true;
            }
            Event::End(TagEnd::Link) => {
                if let Some(table) = &mut table {
                    table.thiscell.push_str("]");
                } else {
                    write!(writer, "]")?;
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                writeln!(writer, "``````")?
            }
            Event::Start(Tag::List(l)) => {
                writeln!(writer)?;
                list = l;
            }
            Event::Start(Tag::Item) => {
                if let Some(l) = &mut list {
                    write!(writer, "{l}. ")?;
                    *l += 1;
                } else {
                    write!(writer, "- ")?;
                }
                in_listitem = true;
            }
            Event::End(TagEnd::Item) => {
                writeln!(writer)?;
                in_listitem = false;
            }
            Event::End(TagEnd::List(_)) => {
                list = None;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let hl = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                write!(
                    writer,
                    "\n{} ",
                    std::iter::repeat("=").take(hl).collect::<String>(),
                )?;
            }
            Event::End(TagEnd::Heading(_)) => {
                writeln!(writer)?;
            }
            Event::Start(Tag::Image { dest_url, .. }) => write!(
                writer,
                "\n#figure(image({:?}), caption: [",
                dest_url.to_string()
            )?,
            Event::End(TagEnd::Image) => {
                writeln!(writer, "])")?;
            }
            Event::Start(Tag::Table(al)) => {
                let mut tab = MdTable::default();
                tab.aligns = al
                    .into_iter()
                    .map(|a| match a {
                        Alignment::None => "none",
                        Alignment::Left => "left",
                        Alignment::Right => "right",
                        Alignment::Center => "center",
                    })
                    .collect();
                table = Some(tab);
            }
            Event::Start(Tag::TableHead) => {
                if let Some(table) = &mut table {
                    table.on_cell = false;
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(table) = &mut table {
                    table.on_cell = true;
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(table) = &mut table {
                    let cell = table.thiscell.clone();
                    table.thiscell.clear();
                    if table.on_cell {
                        table.cells.push(cell);
                    } else {
                        table.headers.push(cell);
                    }
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some(table) = table.take() {
                    writeln!(
                        writer,
                        "
#table(
  columns: {},
  table.header({}),
  {}
)
",
                        table.aligns.len(),
                        table
                            .headers
                            .iter()
                            .map(|h| format!("[*{h}*]"))
                            .collect::<Vec<String>>()
                            .join(", "),
                        table
                            .cells
                            .iter()
                            .map(|h| format!("[{h}]"))
                            .collect::<Vec<String>>()
                            .join(", "),
                    )?
                }
            }

            // Event::FootnoteReference(r) => write!(writer, "#ft()")?,
            _ => (),
        }
        consec_par = false;
    }

    Ok(())
}

fn escape_typst(text: pulldown_cmark::CowStr) -> String {
    text.replace('*', "\\*").replace('#', "\\#")
}

fn format_link(link: pulldown_cmark::CowStr) -> String {
    format!("#link(\"{link}\")[")
}
