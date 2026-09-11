mod utils;

use std::io;
use std::io::Write;
use terminal_draw::geometry::*;
use terminal_draw::*;

#[derive(Clone)]
struct Frame {
    time_ms: u64,
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
                    contents: cur_contents.trim_end().to_string(),
                });
                cur_contents.clear();
            }
        } else if line.starts_with("time_ms:") {
            // parse time for accumulating frame
            cur_time_ms = line.split(':').nth(1).unwrap().trim().parse().unwrap();
        } else {
            // accumulate content lines
            if !line.is_empty() {
                cur_contents.push_str(line);
                cur_contents.push('\n');
            }
        }
    }

    // don't forget last frame!
    if !cur_contents.is_empty() {
        frames.push(Frame {
            time_ms: cur_time_ms,
            contents: cur_contents.trim_end().to_string(),
        });
    }

    Ok(frames)
}

fn max_frame_width(frames: Vec<Frame>) -> u16 {
    // get string blocks from frames
    let str_block_refs: Vec<&str> = frames.iter().map(|f| f.contents.as_str()).collect();

    // extract each line from blocks into flat vec
    let all_lines: Vec<&str> = str_block_refs.iter().flat_map(|s| s.lines()).collect();

    let mut max_width = 0;
    for line in all_lines {
        let line_len = line.chars().count() as u16;
        if line_len > max_width {
            max_width = line_len;
        }
    }
    max_width
}

fn max_frame_height(frames: Vec<Frame>) -> u16 {
    // get string blocks from frames
    let str_block_refs: Vec<&str> = frames.iter().map(|f| f.contents.as_str()).collect();

    let mut max_height: u16 = 0;
    for s in str_block_refs {
        let height = 1 + s.chars().filter(|&c| c == '\n').count() as u16;
        if height > max_height {
            max_height = height;
        }
    }
    max_height
}

fn fix_frame_sizes(frames: &mut Vec<Frame>, target_width: u16, target_height: u16) {
    for frame in frames {
        let mut rows: Vec<String> = frame
            .contents
            .lines()
            .map(|line| {
                let pad = target_width.saturating_sub(line.chars().count() as u16);
                format!("{}{}", line, " ".repeat(pad as usize))
            })
            .collect();

        rows.resize(target_height as usize, " ".repeat(target_width as usize));
        frame.contents = rows.join("\n");
    }
}

fn render<T: Write>(renderable: &str, writer: &mut T, width: u16, height: u16) {
    // make style for text
    let style = Style::new().italic().fg(Color::Green);

    // build grid of chars for frame
    let grid: Vec<Vec<char>> = renderable
        .lines()
        .map(|line| {
            let mut row: Vec<char> = line.chars().collect();
            row.resize(width as usize, ' ');
            row
        })
        .collect();

    draw_to(writer, DefRect::new(0, 0, width, height), |(x, y)| {
        let c = grid
            .get(y as usize)
            .and_then(|row| row.get(x as usize))
            .copied()
            .unwrap_or(' ');
        AnsiChar(c, style)
    })
    .unwrap();
}

fn main() {
    let mut out = std::io::stdout();
    let contents = utils::read("res/frames.txt").unwrap();
    let mut frames = make_frames(&contents).unwrap();
    let width = max_frame_width(frames.clone());
    let height = max_frame_height(frames.clone());
    fix_frame_sizes(&mut frames, width, height);

    // initial clear and flush
    clear_all(&mut out).unwrap();
    out.flush().unwrap();

    let mut bar_ratio = 0.0; // test
    let mut frame_idx = 0;

    loop {
        let frame = &frames[frame_idx % frames.len()];

        // clear last buffer and render new
        clear_area(&mut out, DefRect::new(0, 0, width, height)).unwrap();
        move_cursor_to(&mut out, (0, 0)).unwrap();
        render(&frame.contents, &mut out, width, height);
        out.flush().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(frame.time_ms));

        bar_ratio += 0.1; // test
        frame_idx += 1;
    }

    // cleanup
    move_cursor_to(&mut out, (0, height)).unwrap();
    out.flush().unwrap();
}
