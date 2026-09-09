mod utils;

use std::io;
use std::io::Write;
use terminal_draw::geometry::*;
use terminal_draw::*;

struct Frame {
    time_ms: i32,
    contents: String,
}

// takes in raw contents and converts to vec of frame structs
fn make_frames(raw: &str) -> Result<Vec<Frame>, io::Error> {
    let mut frames = Vec::new();
    let mut cur_time_ms = 0;
    let mut cur_contents = String::new();

    for line in raw.lines() {
        if line.contains("---frame---") {
            // flush current content and build frame
            if !cur_contents.is_empty() {
                frames.push(Frame {
                    time_ms: cur_time_ms,
                    contents: cur_contents.trim().to_string(),
                });
                cur_contents.clear();
            }
        } else if line.starts_with("time_ms:") {
            // parse time for accumulating frame
            cur_time_ms = line.split(':').nth(1).unwrap().trim().parse().unwrap();
        } else {
            // accumulate content lines
            cur_contents.push_str(line);
            cur_contents.push('\n');
        }
    }

    // don't forget last frame!
    if !cur_contents.is_empty() {
        frames.push(Frame {
            time_ms: cur_time_ms,
            contents: cur_contents.trim().to_string(),
        });
    }

    Ok(frames)
}

fn render<T: Write>(renderable: &str, writer: &mut T, width: u16, height: u16) {
    let style = Style::new().italic().fg(Color::Green);

    // turn our string into ansi chars using our style
    let ansi_chars: Vec<AnsiChar> = renderable.chars().map(|c| AnsiChar(c, style)).collect();

    // draw entire chunk at once
    // need to supply callback, im using lambda
    draw_to(writer, DefRect::new(0, 0, width, height), |(x, y)| {
        let i = (y * width + x) as usize;
        ansi_chars[i] // return char for x,y cell
    })
    .unwrap();
}

fn main() {
    let contents = utils::read("res/frames.txt").unwrap();
    let frames = make_frames(&contents).unwrap();
    let mut out = std::io::stdout();
}
